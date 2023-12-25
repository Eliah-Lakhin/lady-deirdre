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
        tasks::TaskManager,
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

pub type Revision = u64;

pub struct Analyzer<N: Grammar, S: SyncBuildHasher = RandomState> {
    pub(super) documents: UnitTable<Document<N>, S>,
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
            documents: UnitTable::new_single(),
            database: Arc::new(Database::new_single()),
            tasks: TaskManager::new(),
        }
    }

    pub fn for_many_documents() -> Self {
        Self {
            documents: UnitTable::new_multi(1),
            database: Arc::new(Database::new_multi()),
            tasks: TaskManager::new(),
        }
    }

    pub fn analyze<'a>(&'a self, handle: &'a Latch) -> AnalysisResult<AnalysisTask<'a, N, S>> {
        self.tasks.acquire_analysis(handle, true)?;

        Ok(AnalysisTask::new_non_exclusive(self, handle))
    }

    pub fn try_analyze<'a>(&'a self, handle: &'a Latch) -> AnalysisResult<AnalysisTask<'a, N, S>> {
        self.tasks.acquire_analysis(handle, false)?;

        Ok(AnalysisTask::new_non_exclusive(self, handle))
    }

    pub fn mutate<'a>(&'a self, handle: &'a Latch) -> AnalysisResult<MutationTask<'a, N, S>> {
        self.tasks.acquire_mutation(handle, true)?;

        Ok(MutationTask::new_non_exclusive(self, handle))
    }

    pub fn try_mutate<'a>(&'a self, handle: &'a Latch) -> AnalysisResult<MutationTask<'a, N, S>> {
        self.tasks.acquire_mutation(handle, false)?;

        Ok(MutationTask::new_non_exclusive(self, handle))
    }

    pub fn mutate_exclusive<'a>(
        &'a self,
        handle: &'a Latch,
    ) -> AnalysisResult<MutationTask<'a, N, S>> {
        self.tasks.acquire_exclusive(handle, true)?;

        Ok(MutationTask::new_exclusive(self, handle))
    }

    pub fn try_mutate_exclusive<'a>(
        &'a self,
        handle: &'a Latch,
    ) -> AnalysisResult<MutationTask<'a, N, S>> {
        self.tasks.acquire_exclusive(handle, false)?;

        Ok(MutationTask::new_exclusive(self, handle))
    }

    pub fn interrupt(&self, tasks_mask: u8) {
        self.tasks.interrupt(tasks_mask);
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
