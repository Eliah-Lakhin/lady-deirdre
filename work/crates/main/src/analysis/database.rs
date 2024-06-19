////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{
    any::TypeId,
    cell::UnsafeCell,
    collections::HashSet,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicU64, Ordering},
        Condvar,
        Mutex,
    },
    time::{Duration, Instant},
};

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
        TaskHandle,
    },
    arena::{Entry, Id, Repo},
    report::{ld_assert, ld_unreachable},
    sync::{Shared, SyncBuildHasher, Table},
    syntax::NodeRef,
};

/// A version of the [Analyzer](crate::analysis::Analyzer)'s state.
///
/// This value always increases during the Analyzer's lifetime and never
/// decreases.
pub type Revision = u64;

pub(super) struct Database<N: Grammar, H: TaskHandle, S: SyncBuildHasher> {
    pub(super) records: Table<Id, Repo<Record<N, H, S>>, S>,
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

        records_guard.remove(entry);
    }
}

const UNLOCK_MASK: usize = 0;
const READ_MASK: usize = !0 ^ 1;
const READ_BIT: usize = 1 << 1;
const WRITE_MASK: usize = 1;

pub(super) struct Record<N: Grammar, H: TaskHandle, S: SyncBuildHasher> {
    state: Mutex<usize>,
    state_changed: Condvar,
    data: UnsafeCell<RecordData<N, H, S>>,
}

unsafe impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> Send for Record<N, H, S> {}

unsafe impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> Sync for Record<N, H, S> {}

impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> Record<N, H, S> {
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
    pub(super) fn read(&self, timeout: &Duration) -> AnalysisResult<RecordReadGuard<N, H, S>> {
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
    pub(super) fn read(&self, timeout: &Duration) -> AnalysisResult<RecordReadGuard<N, H, S>> {
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
    pub(super) fn write(&self, timeout: &Duration) -> AnalysisResult<RecordWriteGuard<N, H, S>> {
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
    pub(super) fn write(&self, timeout: &Duration) -> AnalysisResult<RecordWriteGuard<N, H, S>> {
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

pub(super) struct RecordReadGuard<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> {
    record: &'a Record<N, H, S>,
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> Drop for RecordReadGuard<'a, N, H, S> {
    fn drop(&mut self) {
        let mut state_guard = self
            .record
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        ld_assert!(*state_guard & WRITE_MASK == 0, "Invalid lock state.");
        ld_assert!(*state_guard & READ_MASK > 0, "Invalid lock state.");

        *state_guard -= READ_BIT;

        if *state_guard == UNLOCK_MASK {
            drop(state_guard);
            self.record.state_changed.notify_one();
        }
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> Deref for RecordReadGuard<'a, N, H, S> {
    type Target = RecordData<N, H, S>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.record.data.get() }
    }
}

pub(super) struct RecordWriteGuard<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> {
    record: &'a Record<N, H, S>,
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> Drop for RecordWriteGuard<'a, N, H, S> {
    fn drop(&mut self) {
        let mut state_guard = self
            .record
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        ld_assert!(*state_guard & WRITE_MASK > 0, "Invalid lock state.");
        ld_assert!(*state_guard & READ_MASK == 0, "Invalid lock state.");

        *state_guard = UNLOCK_MASK;

        drop(state_guard);

        self.record.state_changed.notify_all();
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> Deref for RecordWriteGuard<'a, N, H, S> {
    type Target = RecordData<N, H, S>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.record.data.get() }
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> DerefMut for RecordWriteGuard<'a, N, H, S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.record.data.get() }
    }
}

pub(super) struct RecordData<N: Grammar, H: TaskHandle, S: SyncBuildHasher> {
    pub(super) verified_at: Revision,
    pub(super) cache: Option<RecordCache<N, S>>,
    pub(super) node_ref: NodeRef,
    pub(super) function: &'static dyn Function<N, H, S>,
}

impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> RecordData<N, H, S> {
    #[inline(always)]
    fn new<C: Computable<Node = N> + Eq>(node_ref: NodeRef) -> Self {
        Self {
            verified_at: 0,
            cache: None,
            node_ref,
            function: &(C::compute as fn(&mut AttrContext<C::Node, H, S>) -> AnalysisResult<C>),
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
            unsafe { ld_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        unsafe { &*(self.memo.deref() as *const dyn Memo as *const T) }
    }

    // Safety: `T` properly describes `memo` type.
    #[inline(always)]
    pub(super) unsafe fn downcast_unchecked_mut<T: 'static>(&mut self) -> &mut T {
        if self.memo.memo_type_id() != TypeId::of::<T>() {
            // Safety: Upheld by the caller.
            unsafe { ld_unreachable!("Incorrect memo type.") }
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
            unsafe { ld_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        let other = unsafe { &*(other as *const dyn Memo as *const T) };

        self.eq(other)
    }
}

pub(super) trait Function<N: Grammar, H: TaskHandle, S: SyncBuildHasher>:
    Send + Sync + 'static
{
    fn invoke(&self, task: &mut AttrContext<N, H, S>) -> AnalysisResult<Box<dyn Memo>>;
}

impl<T, N, H, S> Function<N, H, S> for fn(&mut AttrContext<N, H, S>) -> AnalysisResult<T>
where
    T: Eq + Send + Sync + Sized + 'static,
    N: Grammar,
    H: TaskHandle,
    S: SyncBuildHasher,
{
    fn invoke(&self, context: &mut AttrContext<N, H, S>) -> AnalysisResult<Box<dyn Memo>> {
        Ok(Box::new(self(context)?))
    }
}
