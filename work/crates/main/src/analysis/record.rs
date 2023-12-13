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
        memo::{get_function, Function, Memo},
        AnalysisError,
        AnalysisResult,
        AttrRef,
        Computable,
        Grammar,
        Revision,
    },
    report::debug_unreachable,
    std::*,
    sync::{Latch, Shared, SyncBuildHasher},
    syntax::NodeRef,
};

const WRITERS_CAPACITY: usize = 8;

pub(super) struct Record<N: Grammar, S: SyncBuildHasher> {
    lock: RwLock<Cell<N, S>>,
    writers: Mutex<HashSet<usize, S>>,
}

impl<N: Grammar, S: SyncBuildHasher> Record<N, S> {
    #[inline(always)]
    pub(super) fn new<C: Computable<Node = N> + Eq>(node_ref: NodeRef) -> Self {
        Self {
            lock: RwLock::new(Cell::new::<C>(node_ref)),
            writers: Mutex::new(HashSet::with_capacity_and_hasher(
                WRITERS_CAPACITY,
                S::default(),
            )),
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

    #[inline(always)]
    pub(super) fn read(&self) -> RwLockReadGuard<Cell<N, S>> {
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
    guard: RwLockWriteGuard<'a, Cell<N, S>>,
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
    type Target = Cell<N, S>;

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

pub(super) struct Cell<N: Grammar, S: SyncBuildHasher> {
    pub(super) verified_at: Revision,
    pub(super) cache: Option<Cache<S>>,
    pub(super) node_ref: NodeRef,
    pub(super) function: &'static dyn Function<N, S>,
}

impl<N: Grammar, S: SyncBuildHasher> Cell<N, S> {
    #[inline(always)]
    fn new<C: Computable<Node = N> + Eq>(node_ref: NodeRef) -> Self {
        Self {
            verified_at: 0,
            cache: None,
            node_ref,
            function: get_function::<C, S>(),
        }
    }
}

pub(super) struct Cache<S> {
    pub(super) dirty: bool,
    pub(super) updated_at: Revision,
    pub(super) memo: Box<dyn Memo>,
    pub(super) deps: Shared<HashSet<AttrRef, S>>,
}

impl<S> Cache<S> {
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
}
