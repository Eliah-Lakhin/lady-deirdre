////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{
    any::TypeId,
    collections::HashSet,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use crate::{
    analysis::{
        lock::TimeoutRwLock,
        AnalysisError,
        AnalysisResult,
        AnalyzerConfig,
        AttrContext,
        AttrRef,
        Classifier,
        Computable,
        Event,
        Grammar,
        SlotRef,
        TaskHandle,
    },
    arena::{Entry, Id, Repo},
    report::ld_unreachable,
    sync::{Shared, SyncBuildHasher, Table},
    syntax::NodeRef,
};

/// A version of the [Analyzer](crate::analysis::Analyzer)'s state.
///
/// This value always increases during the Analyzer's lifetime and never
/// decreases.
pub type Revision = u64;

pub(super) struct Database<N: Grammar, H: TaskHandle, S: SyncBuildHasher> {
    pub(super) records: Table<Id, DocRecords<N, H, S>, S>,
    pub(super) timeout: Duration,
    pub(super) revision: AtomicU64,
}

impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> Database<N, H, S> {
    #[inline(always)]
    pub(super) fn new(config: &AnalyzerConfig) -> Self {
        Self {
            records: match config.single_document {
                true => Table::with_capacity_and_hasher_and_shards(0, S::default(), 1),
                false => Table::new(),
            },
            timeout: config.analysis_timeout,
            revision: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    pub(super) fn load_revision(&self) -> Revision {
        self.revision.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub(super) fn commit_revision(&self) -> Revision {
        self.revision.fetch_add(1, Ordering::Relaxed) + 1
    }
}

pub(super) trait AbstractDatabase: Send + Sync + 'static {
    fn deregister_attribute(&self, id: Id, entry: &Entry);
}

impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> AbstractDatabase for Database<N, H, S> {
    fn deregister_attribute(&self, id: Id, entry: &Entry) {
        let Some(mut records_guard) = self.records.get_mut(&id) else {
            return;
        };

        records_guard.attrs.remove(entry);
    }
}

pub(super) struct DocRecords<N: Grammar, H: TaskHandle, S: SyncBuildHasher> {
    pub(super) attrs: Repo<AttrRecord<N, H, S>>,
    pub(super) slots: Repo<SlotRecord>,
}

impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> DocRecords<N, H, S> {
    #[inline(always)]
    pub(super) fn new() -> Self {
        Self {
            attrs: Repo::new(),
            slots: Repo::new(),
        }
    }

    #[inline(always)]
    pub(super) fn with_capacity(attrs_capacity: usize) -> Self {
        Self {
            attrs: Repo::with_capacity(attrs_capacity),
            slots: Repo::new(),
        }
    }
}

pub(super) type AttrRecord<N, H, S> = TimeoutRwLock<AttrRecordData<N, H, S>>;

impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> AttrRecord<N, H, S> {
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
}

pub(super) struct AttrRecordData<N: Grammar, H: TaskHandle, S: SyncBuildHasher> {
    pub(super) verified_at: Revision,
    pub(super) cache: Option<AttrRecordCache<N, S>>,
    pub(super) node_ref: NodeRef,
    pub(super) function: &'static dyn Function<N, H, S>,
}

impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> AttrRecordData<N, H, S> {
    #[inline(always)]
    pub(super) fn new<C: Computable<Node = N> + Eq>(node_ref: NodeRef) -> Self {
        Self {
            verified_at: 0,
            cache: None,
            node_ref,
            function: &(C::compute as fn(&mut AttrContext<C::Node, H, S>) -> AnalysisResult<C>),
        }
    }
}

pub(super) struct AttrRecordCache<N: Grammar, S: SyncBuildHasher> {
    pub(super) dirty: bool,
    pub(super) updated_at: Revision,
    pub(super) memo: Box<dyn AttrMemo>,
    pub(super) deps: Shared<CacheDeps<N, S>>,
}

impl<N: Grammar, S: SyncBuildHasher> AttrRecordCache<N, S> {
    #[inline(always)]
    pub(super) fn downcast<T: 'static>(&self) -> AnalysisResult<&T> {
        let memo = self.memo.deref();

        if memo.attr_memo_type_id() != TypeId::of::<T>() {
            return Err(AnalysisError::TypeMismatch);
        }

        // Safety: Type checked above.
        Ok(unsafe { &*(memo as *const dyn AttrMemo as *const T) })
    }

    // Safety: `T` properly describes `memo` type.
    #[inline(always)]
    pub(super) unsafe fn downcast_unchecked<T: 'static>(&self) -> &T {
        let memo = self.memo.deref();

        #[cfg(debug_assertions)]
        if memo.attr_memo_type_id() != TypeId::of::<T>() {
            // Safety: Upheld by the caller.
            unsafe { ld_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        unsafe { &*(memo as *const dyn AttrMemo as *const T) }
    }

    // Safety: `T` properly describes `memo` type.
    #[inline(always)]
    pub(super) unsafe fn downcast_unchecked_mut<T: 'static>(&mut self) -> &mut T {
        let memo = self.memo.deref_mut();

        #[cfg(debug_assertions)]
        if memo.attr_memo_type_id() != TypeId::of::<T>() {
            // Safety: Upheld by the caller.
            unsafe { ld_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        unsafe { &mut *(memo as *mut dyn AttrMemo as *mut T) }
    }
}

pub(super) struct CacheDeps<N: Grammar, S: SyncBuildHasher> {
    pub(super) attrs: HashSet<AttrRef, S>,
    pub(super) slots: HashSet<SlotRef, S>,
    pub(super) events: HashSet<(Id, Event), S>,
    pub(super) classes: HashSet<(Id, <N::Classifier as Classifier>::Class), S>,
}

impl<N: Grammar, S: SyncBuildHasher> Default for CacheDeps<N, S> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            attrs: HashSet::default(),
            slots: HashSet::default(),
            events: HashSet::default(),
            classes: HashSet::default(),
        }
    }
}

pub(super) type SlotRecord = TimeoutRwLock<SlotRecordData>;

pub(super) struct SlotRecordData {
    pub(super) revision: Revision,
    pub(super) memo: Box<dyn SlotMemo>,
}

impl SlotRecordData {
    #[inline(always)]
    pub(super) fn new<T: Default + Send + Sync + 'static>() -> Self {
        Self {
            revision: 0,
            memo: Box::new(T::default()),
        }
    }

    #[inline(always)]
    pub(super) fn downcast<T: 'static>(&self) -> AnalysisResult<&T> {
        let memo = self.memo.deref();

        if memo.slot_memo_type_id() != TypeId::of::<T>() {
            return Err(AnalysisError::TypeMismatch);
        }

        // Safety: Type checked above.
        Ok(unsafe { &*(memo as *const dyn SlotMemo as *const T) })
    }

    #[inline(always)]
    pub(super) fn downcast_mut<T: 'static>(&mut self) -> AnalysisResult<&mut T> {
        let memo = self.memo.deref_mut();

        if memo.slot_memo_type_id() != TypeId::of::<T>() {
            return Err(AnalysisError::TypeMismatch);
        }

        // Safety: Type checked above.
        Ok(unsafe { &mut *(memo as *mut dyn SlotMemo as *mut T) })
    }

    // Safety: `T` properly describes underlying `memo` type.
    #[inline(always)]
    pub(super) unsafe fn downcast_unchecked<T: 'static>(&self) -> &T {
        let memo = self.memo.deref();

        #[cfg(debug_assertions)]
        if memo.slot_memo_type_id() != TypeId::of::<T>() {
            // Safety: Upheld by the caller.
            unsafe { ld_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        unsafe { &*(memo as *const dyn SlotMemo as *const T) }
    }

    // Safety: `T` properly describes underlying `memo` type.
    #[inline(always)]
    pub(super) unsafe fn downcast_unchecked_mut<T: 'static>(&mut self) -> &mut T {
        let memo = Box::deref_mut(&mut self.memo);

        #[cfg(debug_assertions)]
        if memo.slot_memo_type_id() != TypeId::of::<T>() {
            // Safety: Upheld by the caller.
            unsafe { ld_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        unsafe { &mut *(memo as *mut dyn SlotMemo as *mut T) }
    }
}

pub(super) trait SlotMemo: Send + Sync + 'static {
    fn slot_memo_type_id(&self) -> TypeId;
}

impl<T: Default + Send + Sync + 'static> SlotMemo for T {
    #[inline(always)]
    fn slot_memo_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

pub(super) trait AttrMemo: Send + Sync + 'static {
    fn attr_memo_type_id(&self) -> TypeId;

    // Safety: `self` and `other` represent the same type.
    unsafe fn attr_memo_eq(&self, other: &dyn AttrMemo) -> bool;
}

impl<T: Eq + Send + Sync + 'static> AttrMemo for T {
    #[inline(always)]
    fn attr_memo_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    #[inline(always)]
    unsafe fn attr_memo_eq(&self, other: &dyn AttrMemo) -> bool {
        if self.attr_memo_type_id() != other.attr_memo_type_id() {
            // Safety: Upheld by the caller.
            unsafe { ld_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        let other = unsafe { &*(other as *const dyn AttrMemo as *const T) };

        self.eq(other)
    }
}

pub(super) trait Function<N: Grammar, H: TaskHandle, S: SyncBuildHasher>:
    Send + Sync + 'static
{
    fn invoke(&self, task: &mut AttrContext<N, H, S>) -> AnalysisResult<Box<dyn AttrMemo>>;
}

impl<T, N, H, S> Function<N, H, S> for fn(&mut AttrContext<N, H, S>) -> AnalysisResult<T>
where
    T: Eq + Send + Sync + Sized + 'static,
    N: Grammar,
    H: TaskHandle,
    S: SyncBuildHasher,
{
    fn invoke(&self, context: &mut AttrContext<N, H, S>) -> AnalysisResult<Box<dyn AttrMemo>> {
        Ok(Box::new(self(context)?))
    }
}
