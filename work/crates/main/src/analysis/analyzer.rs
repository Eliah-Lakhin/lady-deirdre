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
        manager::TaskManager,
        table::{UnitTable, UnitTableReadGuard},
        AnalysisResult,
        AnalysisTask,
        ExclusiveTask,
        FeatureInitializer,
        Grammar,
        MutationTask,
    },
    arena::{Identifiable, Repo},
    std::*,
    sync::{Latch, Shared, SyncBuildHasher},
    syntax::{ErrorRef, NodeRef, SyntaxTree},
    units::Document,
};

pub type Revision = u64;

pub struct Analyzer<N: Grammar, S: SyncBuildHasher = RandomState> {
    pub(super) docs: UnitTable<DocEntry<N, S>, S>,
    pub(super) database: Arc<Database<N, S>>,
    pub(super) tasks: TaskManager<S>,
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
            docs: UnitTable::new_single(),
            database: Arc::new(Database::new_single()),
            tasks: TaskManager::new(),
        }
    }

    pub fn for_many_documents() -> Self {
        Self {
            docs: UnitTable::new_multi(1),
            database: Arc::new(Database::new_multi()),
            tasks: TaskManager::new(),
        }
    }

    pub fn analyze<'a>(&'a self, handle: &'a Latch) -> AnalysisResult<AnalysisTask<'a, N, S>> {
        self.tasks.acquire_analysis(handle, true)?;

        Ok(AnalysisTask::new(self, handle))
    }

    pub fn try_analyze<'a>(&'a self, handle: &'a Latch) -> AnalysisResult<AnalysisTask<'a, N, S>> {
        self.tasks.acquire_analysis(handle, false)?;

        Ok(AnalysisTask::new(self, handle))
    }

    pub fn mutate<'a>(&'a self, handle: &'a Latch) -> AnalysisResult<MutationTask<'a, N, S>> {
        self.tasks.acquire_mutation(handle, true)?;

        Ok(MutationTask::new(self, handle))
    }

    pub fn try_mutate<'a>(&'a self, handle: &'a Latch) -> AnalysisResult<MutationTask<'a, N, S>> {
        self.tasks.acquire_mutation(handle, false)?;

        Ok(MutationTask::new(self, handle))
    }

    pub fn exclusive<'a>(&'a self, handle: &'a Latch) -> AnalysisResult<ExclusiveTask<'a, N, S>> {
        self.tasks.acquire_exclusive(handle, true)?;

        Ok(ExclusiveTask::new(self, handle))
    }

    pub fn try_exclusive<'a>(
        &'a self,
        handle: &'a Latch,
    ) -> AnalysisResult<ExclusiveTask<'a, N, S>> {
        self.tasks.acquire_exclusive(handle, false)?;

        Ok(ExclusiveTask::new(self, handle))
    }

    pub fn interrupt(&self, tasks_mask: u8) {
        self.tasks.interrupt(tasks_mask);
    }

    pub(super) fn register_doc(&self, mut document: Document<N>) {
        let id = document.id();

        let node_refs = document.node_refs().collect::<Vec<_>>();
        let mut records = Repo::with_capacity(node_refs.len());
        let mut scopes = HashSet::with_capacity_and_hasher(node_refs.len(), S::default());

        if !node_refs.is_empty() {
            let mut initializer = FeatureInitializer {
                id,
                database: Arc::downgrade(&self.database) as Weak<_>,
                records: &mut records,
            };

            for node_ref in node_refs {
                let Some(node) = node_ref.deref_mut(&mut document) else {
                    continue;
                };

                if node.is_scope() {
                    let _ = scopes.insert(node_ref);
                }

                node.initialize(&mut initializer);
            }

            self.database.commit();
        }

        let errors = document.error_refs().collect();

        // Safety: Ids are globally unique.
        unsafe {
            self.docs.insert(
                id,
                DocEntry {
                    document,
                    scope_accumulator: Shared::new(scopes),
                    error_accumulator: Shared::new(errors),
                },
            );
        }

        // Safety: records are always in sync with documents.
        unsafe {
            self.database.records.insert(id, records);
        }
    }
}

#[repr(transparent)]
pub struct DocumentReadGuard<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    guard: UnitTableReadGuard<'a, DocEntry<N, S>, S>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Deref for DocumentReadGuard<'a, N, S> {
    type Target = Document<N>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.guard.deref().document
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> From<UnitTableReadGuard<'a, DocEntry<N, S>, S>>
    for DocumentReadGuard<'a, N, S>
{
    #[inline(always)]
    fn from(guard: UnitTableReadGuard<'a, DocEntry<N, S>, S>) -> Self {
        Self { guard }
    }
}

pub(super) struct DocEntry<N: Grammar, S: SyncBuildHasher> {
    pub(super) document: Document<N>,
    pub(super) scope_accumulator: Shared<HashSet<NodeRef, S>>,
    pub(super) error_accumulator: Shared<HashSet<ErrorRef, S>>,
}
