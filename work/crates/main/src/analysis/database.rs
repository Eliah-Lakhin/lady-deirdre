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
        AnalyzerConfig,
        AttrContext,
        AttrRef,
        Classifier,
        Computable,
        Event,
        Grammar,
        Handle,
    },
    arena::{Entry, Id, Repo},
    report::{debug_assert, debug_unreachable, system_panic},
    std::*,
    sync::{Shared, SyncBuildHasher, Table},
    syntax::NodeRef,
};

pub type Revision = u64;

pub(super) struct Database<N: Grammar, S: SyncBuildHasher> {
    pub(super) records: Table<Id, Repo<Record<N, S>>, S>,
    pub(super) timeout: Duration,
    pub(super) revision: AtomicU64,
}

impl<N: Grammar, S: SyncBuildHasher> Database<N, S> {
    #[inline(always)]
    pub(super) fn new(config: &AnalyzerConfig) -> Self {
        Self {
            records: match config.single_document {
                true => Table::with_capacity_and_hasher_and_shards(0, S::default(), 1),
                false => Table::new(),
            },
            timeout: config.attributes_timeout,
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

const UNLOCK_MASK: usize = 0;
const READ_MASK: usize = !0 ^ 1;
const READ_BIT: usize = 1 << 1;
const WRITE_MASK: usize = 1;
const TIMEOUT: u64 = 1 << 10;

pub(super) struct Record<N: Grammar, S: SyncBuildHasher> {
    state: Mutex<usize>,
    state_changed: Condvar,
    data: UnsafeCell<RecordData<N, S>>,
}

unsafe impl<N: Grammar, S: SyncBuildHasher> Send for Record<N, S> {}

unsafe impl<N: Grammar, S: SyncBuildHasher> Sync for Record<N, S> {}

impl<N: Grammar, S: SyncBuildHasher> Record<N, S> {
    #[inline(always)]
    pub(super) fn new<C: Computable<Node = N> + Eq>(node_ref: NodeRef) -> Self {
        Self {
            state: Mutex::new(UNLOCK_MASK),
            state_changed: Condvar::new(),
            data: UnsafeCell::new(RecordData::new::<C>(node_ref)),
        }
    }

    #[inline(always)]
    pub(super) fn invalidate(&self) {
        let Ok(mut guard) = self.write(&Duration::ZERO) else {
            panic!("Invalidation timeout.");
        };

        let Some(cache) = &mut guard.cache else {
            return;
        };

        cache.dirty = true;
    }

    #[cfg(not(target_family = "wasm"))]
    pub(super) fn read(&self, timeout: &Duration) -> AnalysisResult<RecordReadGuard<N, S>> {
        let mut state_guard = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        let mut cooldown = 1;
        let time = Instant::now();

        loop {
            if *state_guard & WRITE_MASK == 0 {
                *state_guard += READ_BIT;
                return Ok(RecordReadGuard { record: self });
            }

            (state_guard, _) = self
                .state_changed
                .wait_timeout(state_guard, Duration::from_millis(cooldown))
                .unwrap_or_else(|poison| poison.into_inner());

            if &time.elapsed() > timeout {
                return Err(AnalysisError::Timeout);
            }

            cooldown <<= 1;
        }
    }

    #[cfg(target_family = "wasm")]
    pub(super) fn read(&self, timeout: &Duration) -> AnalysisResult<RecordReadGuard<N, S>> {
        let mut state_guard = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        if *state_guard & WRITE_MASK == 0 {
            *state_guard += READ_BIT;
            return Ok(RecordReadGuard { record: self });
        }

        Err(AnalysisError::Timeout)
    }

    #[cfg(not(target_family = "wasm"))]
    pub(super) fn write(&self, timeout: &Duration) -> AnalysisResult<RecordWriteGuard<N, S>> {
        let mut state_guard = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        let mut culldown = 1;
        let time = Instant::now();

        loop {
            if *state_guard == UNLOCK_MASK {
                *state_guard = WRITE_MASK;
                return Ok(RecordWriteGuard { record: self });
            }

            (state_guard, _) = self
                .state_changed
                .wait_timeout(state_guard, Duration::from_millis(culldown))
                .unwrap_or_else(|poison| poison.into_inner());

            if &time.elapsed() > timeout {
                return Err(AnalysisError::Timeout);
            }

            culldown <<= 1;
        }
    }

    #[cfg(target_family = "wasm")]
    pub(super) fn write(&self, timeout: &Duration) -> AnalysisResult<RecordWriteGuard<N, S>> {
        let mut state_guard = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        if *state_guard == UNLOCK_MASK {
            *state_guard = WRITE_MASK;
            return Ok(RecordWriteGuard { record: self });
        }

        Err(AnalysisError::Timeout)
    }
}

pub(super) struct RecordReadGuard<'a, N: Grammar, S: SyncBuildHasher> {
    record: &'a Record<N, S>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Drop for RecordReadGuard<'a, N, S> {
    fn drop(&mut self) {
        let mut state_guard = self
            .record
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        debug_assert!(*state_guard & WRITE_MASK == 0, "Invalid lock state.");
        debug_assert!(*state_guard & READ_MASK > 0, "Invalid lock state.");

        *state_guard -= READ_BIT;

        if *state_guard == UNLOCK_MASK {
            drop(state_guard);
            self.record.state_changed.notify_one();
        }
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Deref for RecordReadGuard<'a, N, S> {
    type Target = RecordData<N, S>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.record.data.get() }
    }
}

pub(super) struct RecordWriteGuard<'a, N: Grammar, S: SyncBuildHasher> {
    record: &'a Record<N, S>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Drop for RecordWriteGuard<'a, N, S> {
    fn drop(&mut self) {
        let mut state_guard = self
            .record
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        debug_assert!(*state_guard & WRITE_MASK > 0, "Invalid lock state.");
        debug_assert!(*state_guard & READ_MASK == 0, "Invalid lock state.");

        *state_guard = UNLOCK_MASK;

        drop(state_guard);

        self.record.state_changed.notify_all();
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Deref for RecordWriteGuard<'a, N, S> {
    type Target = RecordData<N, S>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.record.data.get() }
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> DerefMut for RecordWriteGuard<'a, N, S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.record.data.get() }
    }
}

pub(super) struct RecordData<N: Grammar, S: SyncBuildHasher> {
    pub(super) verified_at: Revision,
    pub(super) cache: Option<RecordCache<N, S>>,
    pub(super) node_ref: NodeRef,
    pub(super) function: &'static dyn Function<N, S>,
}

impl<N: Grammar, S: SyncBuildHasher> RecordData<N, S> {
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
