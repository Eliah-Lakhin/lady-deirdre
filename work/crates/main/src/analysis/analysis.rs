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
        analyzer::{AnalyzerStage, DEPS_CAPACITY, TASKS_CAPACITY},
        AnalysisError,
        AnalysisResult,
        Analyzer,
        AttrRef,
        Grammar,
        Revision,
    },
    report::{debug_unreachable, system_panic},
    std::*,
    sync::{Latch, Lazy, SyncBuildHasher},
    syntax::NodeRef,
};

pub struct AnalysisTask<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    fork: Option<Fork<'a, S>>,
    pub(super) analyzer: &'a Analyzer<N, S>,
    pub(super) revision: Revision,
    pub(super) handle: &'a Latch,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Drop for AnalysisTask<'a, N, S> {
    fn drop(&mut self) {
        if self.fork.is_some() {
            return;
        }

        self.handle.set();

        let mut state_guard = self
            .analyzer
            .stage
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        match state_guard.deref_mut() {
            AnalyzerStage::Analysis { tasks } => {
                tasks.remove(self.handle);

                if tasks.is_empty() {
                    tasks.shrink_to(TASKS_CAPACITY);
                }
            }

            AnalyzerStage::Interruption { queue } => {
                *queue -= 1;

                if *queue == 0 {
                    *state_guard = AnalyzerStage::Mutation { queue: 0 };
                    drop(state_guard);
                    self.analyzer.ready_for_mutation.notify_all();
                }
            }

            AnalyzerStage::Mutation { .. } => system_panic!("State mismatch."),
        }
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> AnalysisTask<'a, N, S> {
    #[inline(always)]
    pub(super) fn new(analyzer: &'a Analyzer<N, S>, handle: &'a Latch) -> Self {
        Self {
            fork: None,
            analyzer,
            revision: analyzer.database.revision(),
            handle,
        }
    }

    #[inline(always)]
    pub(super) fn fork_for_mutation(analyzer: &'a Analyzer<N, S>) -> AnalysisTask<N, S> {
        static NIL_REF: NodeRef = NodeRef::nil();
        static DUMMY_HANDLE: Lazy<Latch> = Lazy::new(|| Latch::new());

        #[cfg(debug_assertions)]
        {
            if DUMMY_HANDLE.get_relaxed() {
                system_panic!("Dummy mutation handle is set.");
            }
        }

        AnalysisTask {
            fork: Some(Fork {
                node_ref: &NIL_REF,
                deps: None,
            }),
            analyzer,
            revision: analyzer.database.revision(),
            handle: DUMMY_HANDLE.deref(),
        }
    }

    #[inline(always)]
    pub fn analyzer(&self) -> &'a Analyzer<N, S> {
        self.analyzer
    }

    #[inline(always)]
    pub fn handle(&self) -> &'a Latch {
        &self.handle
    }

    #[inline(always)]
    pub fn revision(&self) -> Revision {
        self.revision
    }

    #[inline(always)]
    pub fn node_ref(&self) -> &'a NodeRef {
        static NIL: NodeRef = NodeRef::nil();

        match &self.fork {
            Some(fork) => fork.node_ref,
            None => &NIL,
        }
    }

    #[inline(always)]
    pub fn proceed(&self) -> AnalysisResult<()> {
        if self.handle.get_relaxed() {
            return Err(AnalysisError::Interrupted);
        }

        Ok(())
    }

    #[inline(always)]
    pub(super) fn fork(&self, node_ref: &'a NodeRef) -> AnalysisTask<N, S> {
        AnalysisTask {
            fork: Some(Fork {
                node_ref,
                deps: Some(HashSet::with_capacity_and_hasher(
                    DEPS_CAPACITY,
                    S::default(),
                )),
            }),
            analyzer: self.analyzer,
            revision: self.revision,
            handle: self.handle,
        }
    }

    #[inline(always)]
    pub(super) fn track(&mut self, attr_ref: &AttrRef) {
        let Some(fork) = &mut self.fork else {
            return;
        };

        let Some(deps) = &mut fork.deps else {
            return;
        };

        deps.insert(*attr_ref);
    }

    #[inline(always)]
    pub(super) fn take_deps(mut self) -> Option<HashSet<AttrRef, S>> {
        let Some(fork) = &mut self.fork else {
            return None;
        };

        take(&mut fork.deps)
    }

    // Safety: This instance is a fork.
    #[inline(always)]
    pub(super) unsafe fn reuse(&mut self, node_ref: &'a NodeRef) {
        let Some(fork) = &mut self.fork else {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("Not a fork.") }
        };

        fork.node_ref = node_ref;
    }
}

struct Fork<'a, S: SyncBuildHasher> {
    node_ref: &'a NodeRef,
    deps: Option<HashSet<AttrRef, S>>,
}
