////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" Work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This Work is a proprietary software with source available code.            //
//                                                                            //
// To copy, use, distribute, and contribute into this Work you must agree to  //
// the terms of the End User License Agreement:                               //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The Agreement let you use this Work in commercial and non-commercial       //
// purposes. Commercial use of the Work is free of charge to start,           //
// but the Agreement obligates you to pay me royalties                        //
// under certain conditions.                                                  //
//                                                                            //
// If you want to contribute into the source code of this Work,               //
// the Agreement obligates you to assign me all exclusive rights to           //
// the Derivative Work or contribution made by you                            //
// (this includes GitHub forks and pull requests to my repository).           //
//                                                                            //
// The Agreement does not limit rights of the third party software developers //
// as long as the third party software uses public API of this Work only,     //
// and the third party software does not incorporate or distribute            //
// this Work directly.                                                        //
//                                                                            //
// AS FAR AS THE LAW ALLOWS, THIS SOFTWARE COMES AS IS, WITHOUT ANY WARRANTY  //
// OR CONDITION, AND I WILL NOT BE LIABLE TO ANYONE FOR ANY DAMAGES           //
// RELATED TO THIS SOFTWARE, UNDER ANY KIND OF LEGAL CLAIM.                   //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this Work.                                                      //
//                                                                            //
// Copyright (c) 2022 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use crate::{
    analysis::{
        AnalysisError,
        AnalysisResult,
        AttrContext,
        AttrRef,
        Classifier,
        Computable,
        Event,
        Grammar,
    },
    arena::{Entry, Id, Repo},
    report::debug_unreachable,
    std::*,
    sync::{Latch, Shared, SyncBuildHasher, Table},
    syntax::NodeRef,
};

pub type Revision = u64;

pub(super) struct Database<N: Grammar, S: SyncBuildHasher> {
    pub(super) records: Table<Id, Repo<Record<N, S>>, S>,
    pub(super) revision: AtomicU64,
}

impl<N: Grammar, S: SyncBuildHasher> Database<N, S> {
    #[inline(always)]
    pub(super) fn new_single() -> Self {
        Self {
            records: Table::with_capacity_and_hasher_and_shards(0, S::default(), 1),
            revision: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    pub(super) fn new_many() -> Self {
        Self {
            records: Table::new(),
            revision: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    pub(super) fn load_revision(&self) -> Revision {
        self.revision.load(AtomicOrdering::Relaxed)
    }

    #[inline(always)]
    pub(super) fn commit_revision(&self) -> Revision {
        self.revision.fetch_add(1, AtomicOrdering::Relaxed) + 1
    }
}

pub(super) trait AbstractDatabase: Send + Sync + 'static {
    fn deregister_attribute(&self, id: Id, entry: &Entry);
}

impl<N: Grammar, S: SyncBuildHasher> AbstractDatabase for Database<N, S> {
    fn deregister_attribute(&self, id: Id, entry: &Entry) {
        let Some(mut records_guard) = self.records.get_mut(&id) else {
            return;
        };

        records_guard.remove(entry);
    }
}

pub(super) struct Record<N: Grammar, S: SyncBuildHasher> {
    lock: RwLock<RecordInner<N, S>>,
    writers: Mutex<HashSet<usize, S>>,
}

impl<N: Grammar, S: SyncBuildHasher> Record<N, S> {
    #[inline(always)]
    pub(super) fn new<C: Computable<Node = N> + Eq>(node_ref: NodeRef) -> Self {
        Self {
            lock: RwLock::new(RecordInner::new::<C>(node_ref)),
            writers: Mutex::new(HashSet::default()),
        }
    }

    #[inline(always)]
    pub(super) fn invalidate(&self) {
        let mut guard = self
            .lock
            .write()
            .unwrap_or_else(|poison| poison.into_inner());

        let Some(cache) = &mut guard.cache else {
            return;
        };

        cache.dirty = true;
    }

    //todo check writers too?
    #[inline(always)]
    pub(super) fn read(&self) -> RwLockReadGuard<RecordInner<N, S>> {
        self.lock
            .read()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    #[inline(always)]
    pub(super) fn write(&self, task: &Latch) -> AnalysisResult<RecordWriteGuard<N, S>> {
        let writer = task.addr();

        let mut writers_guard = self
            .writers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        if !writers_guard.insert(writer) {
            return Err(AnalysisError::CycleDetected);
        }

        drop(writers_guard);

        let guard = self
            .lock
            .write()
            .unwrap_or_else(|poison| poison.into_inner());

        Ok(RecordWriteGuard {
            writers: &self.writers,
            writer,
            guard,
        })
    }
}

pub(super) struct RecordWriteGuard<'a, N: Grammar, S: SyncBuildHasher> {
    writers: &'a Mutex<HashSet<usize, S>>,
    writer: usize,
    guard: RwLockWriteGuard<'a, RecordInner<N, S>>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Drop for RecordWriteGuard<'a, N, S> {
    fn drop(&mut self) {
        let mut writers_guard = self
            .writers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        let _ = writers_guard.remove(&self.writer);
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Deref for RecordWriteGuard<'a, N, S> {
    type Target = RecordInner<N, S>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> DerefMut for RecordWriteGuard<'a, N, S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

pub(super) struct RecordInner<N: Grammar, S: SyncBuildHasher> {
    pub(super) verified_at: Revision,
    pub(super) cache: Option<RecordCache<N, S>>,
    pub(super) node_ref: NodeRef,
    pub(super) function: &'static dyn Function<N, S>,
}

impl<N: Grammar, S: SyncBuildHasher> RecordInner<N, S> {
    #[inline(always)]
    fn new<C: Computable<Node = N> + Eq>(node_ref: NodeRef) -> Self {
        Self {
            verified_at: 0,
            cache: None,
            node_ref,
            function: &(C::compute as fn(&mut AttrContext<C::Node, S>) -> AnalysisResult<C>),
        }
    }
}

pub(super) struct RecordCache<N: Grammar, S: SyncBuildHasher> {
    pub(super) dirty: bool,
    pub(super) updated_at: Revision,
    pub(super) memo: Box<dyn Memo>,
    pub(super) deps: Shared<CacheDeps<N, S>>,
}

impl<N: Grammar, S: SyncBuildHasher> RecordCache<N, S> {
    #[inline(always)]
    pub(super) fn downcast<T: 'static>(&self) -> AnalysisResult<&T> {
        if self.memo.memo_type_id() != TypeId::of::<T>() {
            return Err(AnalysisError::TypeMismatch);
        }

        // Safety: Type checked above.
        Ok(unsafe { &*(self.memo.deref() as *const dyn Memo as *const T) })
    }

    // Safety: `T` properly describes `memo` type.
    #[inline(always)]
    pub(super) unsafe fn downcast_unchecked<T: 'static>(&self) -> &T {
        if self.memo.memo_type_id() != TypeId::of::<T>() {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        unsafe { &*(self.memo.deref() as *const dyn Memo as *const T) }
    }

    // Safety: `T` properly describes `memo` type.
    #[inline(always)]
    pub(super) unsafe fn downcast_unchecked_mut<T: 'static>(&mut self) -> &mut T {
        if self.memo.memo_type_id() != TypeId::of::<T>() {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        unsafe { &mut *(self.memo.deref_mut() as *mut dyn Memo as *mut T) }
    }
}

pub(super) struct CacheDeps<N: Grammar, S: SyncBuildHasher> {
    pub(super) attrs: HashSet<AttrRef, S>,
    pub(super) events: HashSet<(Id, Event), S>,
    pub(super) classes: HashSet<(Id, <N::Classifier as Classifier>::Class), S>,
}

impl<N: Grammar, S: SyncBuildHasher> Default for CacheDeps<N, S> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            attrs: HashSet::default(),
            events: HashSet::default(),
            classes: HashSet::default(),
        }
    }
}

pub(super) trait Memo: Send + Sync + 'static {
    fn memo_type_id(&self) -> TypeId;

    // Safety: `self` and `other` represent the same type.
    unsafe fn memo_eq(&self, other: &dyn Memo) -> bool;
}

impl<T: Eq + Send + Sync + 'static> Memo for T {
    #[inline(always)]
    fn memo_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    #[inline(always)]
    unsafe fn memo_eq(&self, other: &dyn Memo) -> bool {
        if self.memo_type_id() != other.memo_type_id() {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        let other = unsafe { &*(other as *const dyn Memo as *const T) };

        self.eq(other)
    }
}

pub(super) trait Function<N: Grammar, S: SyncBuildHasher>: Send + Sync + 'static {
    fn invoke(&self, task: &mut AttrContext<N, S>) -> AnalysisResult<Box<dyn Memo>>;
}

impl<N, T, S> Function<N, S> for fn(&mut AttrContext<N, S>) -> AnalysisResult<T>
where
    N: Grammar,
    T: Eq + Send + Sync + Sized + 'static,
    S: SyncBuildHasher,
{
    fn invoke(&self, context: &mut AttrContext<N, S>) -> AnalysisResult<Box<dyn Memo>> {
        Ok(Box::new(self(context)?))
    }
}
