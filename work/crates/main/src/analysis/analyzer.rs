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
        database::Database,
        mutation::MutationTask,
        table::{UnitTable, UnitTableReadGuard},
        AnalysisError,
        AnalysisResult,
        AnalysisTask,
        Grammar,
    },
    arena::Id,
    std::*,
    sync::{Latch, SyncBuildHasher},
    units::Document,
};

pub(super) const TASKS_CAPACITY: usize = 10;
pub(super) const DEPS_CAPACITY: usize = 30;

pub type Revision = u64;

pub struct Analyzer<N: Grammar, S: SyncBuildHasher = RandomState> {
    pub(super) documents: UnitTable<Document<N>, S>,
    pub(super) database: Arc<Database<N, S>>,
    pub(super) stage: Mutex<AnalyzerStage<S>>,
    pub(super) ready_for_analysis: Condvar,
    pub(super) ready_for_mutation: Condvar,
}

impl<N: Grammar, S: SyncBuildHasher> Default for Analyzer<N, S> {
    #[inline(always)]
    fn default() -> Self {
        Self::for_many_documents()
    }
}

impl<N: Grammar, S: SyncBuildHasher> Analyzer<N, S> {
    pub fn for_single_document() -> Self {
        Self {
            documents: UnitTable::new_single(),
            database: Arc::new(Database::new_single()),
            stage: Mutex::new(AnalyzerStage::Analysis {
                tasks: HashSet::with_capacity_and_hasher(TASKS_CAPACITY, S::default()),
            }),
            ready_for_analysis: Condvar::new(),
            ready_for_mutation: Condvar::new(),
        }
    }

    pub fn for_many_documents() -> Self {
        Self {
            documents: UnitTable::new_multi(1),
            database: Arc::new(Database::new_multi()),
            stage: Mutex::new(AnalyzerStage::Analysis {
                tasks: HashSet::with_capacity_and_hasher(TASKS_CAPACITY, S::default()),
            }),
            ready_for_analysis: Condvar::new(),
            ready_for_mutation: Condvar::new(),
        }
    }

    pub fn analyze<'a>(&'a self, handle: &'a Latch) -> AnalysisTask<'a, N, S> {
        let mut stage_guard = self
            .stage
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        loop {
            match stage_guard.deref_mut() {
                AnalyzerStage::Analysis { tasks } => {
                    let task = AnalysisTask::new(self, handle);

                    tasks.insert(handle.clone());

                    return task;
                }

                _ => (),
            }

            stage_guard = self
                .ready_for_analysis
                .wait(stage_guard)
                .unwrap_or_else(|poison| poison.into_inner());
        }
    }

    pub fn try_analyze<'a>(&'a self, handle: &'a Latch) -> Option<AnalysisTask<'a, N, S>> {
        let mut stage_guard = self
            .stage
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        match stage_guard.deref_mut() {
            AnalyzerStage::Analysis { tasks } => {
                let task = AnalysisTask::new(self, handle);

                tasks.insert(task.handle().clone());

                Some(task)
            }

            _ => None,
        }
    }

    pub fn interrupt(&self) {
        let mut stage_guard = self
            .stage
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        match stage_guard.deref_mut() {
            AnalyzerStage::Analysis { tasks } => {
                if tasks.len() == 0 {
                    return;
                }

                for task in tasks.iter() {
                    task.set();
                }

                tasks.clear();
            }

            _ => (),
        }
    }

    pub fn mutate(&self) -> MutationTask<N, S> {
        let mut stage_guard = self
            .stage
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        loop {
            match stage_guard.deref_mut() {
                AnalyzerStage::Analysis { tasks } => {
                    let queue = tasks.len();

                    if queue == 0 {
                        *stage_guard = AnalyzerStage::Mutation { queue: 1 };
                        drop(stage_guard);
                        return MutationTask::new(self);
                    }

                    for task in take(tasks) {
                        task.set();
                    }

                    *stage_guard = AnalyzerStage::Interruption { queue };
                }

                AnalyzerStage::Interruption { .. } => (),

                AnalyzerStage::Mutation { queue } => {
                    *queue += 1;
                    return MutationTask::new(self);
                }
            }

            stage_guard = self
                .ready_for_mutation
                .wait(stage_guard)
                .unwrap_or_else(|poison| poison.into_inner());
        }
    }

    #[inline(always)]
    pub fn read_document(&self, id: Id) -> AnalysisResult<DocumentReadGuard<N, S>> {
        let Some(guard) = self.documents.get(id) else {
            return Err(AnalysisError::MissingDocument);
        };

        Ok(DocumentReadGuard { guard })
    }

    #[inline(always)]
    pub fn try_read_document(&self, id: Id) -> Option<DocumentReadGuard<N, S>> {
        Some(DocumentReadGuard {
            guard: self.documents.try_get(id)?,
        })
    }

    #[inline(always)]
    pub fn contains_document(&self, id: Id) -> bool {
        self.documents.contains(id)
    }

    #[inline(always)]
    pub fn is_document_mutable(&self, id: Id) -> bool {
        let Some(document) = self.documents.get(id) else {
            return false;
        };

        document.is_mutable()
    }

    #[inline(always)]
    pub fn is_document_immutable(&self, id: Id) -> bool {
        let Some(document) = self.documents.get(id) else {
            return false;
        };

        document.is_mutable()
    }
}

#[repr(transparent)]
pub struct DocumentReadGuard<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    guard: UnitTableReadGuard<'a, Document<N>, S>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Deref for DocumentReadGuard<'a, N, S> {
    type Target = Document<N>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> AsRef<Document<N>> for DocumentReadGuard<'a, N, S> {
    #[inline(always)]
    fn as_ref(&self) -> &Document<N> {
        self.guard.deref()
    }
}

pub(super) enum AnalyzerStage<S> {
    Analysis { tasks: HashSet<Latch, S> },
    Interruption { queue: usize },
    Mutation { queue: usize },
}
