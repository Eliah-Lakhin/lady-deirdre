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
        entry::DocEntry,
        manager::TaskManager,
        AnalysisResult,
        AnalysisTask,
        Event,
        ExclusiveTask,
        Grammar,
        Handle,
        MutationTask,
        Revision,
    },
    arena::Id,
    std::*,
    sync::{SyncBuildHasher, Table},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub struct AnalyzerConfig {
    pub single_document: bool,
    pub attributes_timeout: Duration,
}

impl Default for AnalyzerConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyzerConfig {
    #[inline(always)]
    pub const fn new() -> Self {
        let attributes_timeout;

        #[cfg(debug_assertions)]
        {
            attributes_timeout = 1000;
        }

        #[cfg(not(debug_assertions))]
        {
            attributes_timeout = 5000;
        }

        Self {
            single_document: false,
            attributes_timeout: Duration::from_millis(attributes_timeout),
        }
    }
}

pub struct Analyzer<N: Grammar, S: SyncBuildHasher = RandomState> {
    pub(super) docs: Table<Id, DocEntry<N, S>, S>,
    pub(super) events: Table<Id, HashMap<Event, Revision>, S>,
    pub(super) db: Arc<Database<N, S>>,
    pub(super) tasks: TaskManager<S>,
}

impl<N: Grammar, S: SyncBuildHasher> Default for Analyzer<N, S> {
    #[inline(always)]
    fn default() -> Self {
        Self::new(AnalyzerConfig::default())
    }
}

impl<N: Grammar, S: SyncBuildHasher> Analyzer<N, S> {
    pub fn new(config: AnalyzerConfig) -> Self {
        Self {
            docs: match config.single_document {
                true => Table::with_capacity_and_hasher_and_shards(1, S::default(), 1),
                false => Table::new(),
            },
            events: match config.single_document {
                true => Table::with_capacity_and_hasher_and_shards(1, S::default(), 1),
                false => Table::with_capacity_and_hasher_and_shards(1, S::default(), 1),
            },
            db: Arc::new(Database::new(&config)),
            tasks: TaskManager::new(),
        }
    }

    pub fn analyze<'a>(&'a self, handle: &'a Handle) -> AnalysisResult<AnalysisTask<'a, N, S>> {
        self.tasks.acquire_analysis(handle, true)?;

        Ok(AnalysisTask::new(self, handle))
    }

    pub fn try_analyze<'a>(&'a self, handle: &'a Handle) -> AnalysisResult<AnalysisTask<'a, N, S>> {
        self.tasks.acquire_analysis(handle, false)?;

        Ok(AnalysisTask::new(self, handle))
    }

    pub fn mutate<'a>(&'a self, handle: &'a Handle) -> AnalysisResult<MutationTask<'a, N, S>> {
        self.tasks.acquire_mutation(handle, true)?;

        Ok(MutationTask::new(self, handle))
    }

    pub fn try_mutate<'a>(&'a self, handle: &'a Handle) -> AnalysisResult<MutationTask<'a, N, S>> {
        self.tasks.acquire_mutation(handle, false)?;

        Ok(MutationTask::new(self, handle))
    }

    pub fn exclusive<'a>(&'a self, handle: &'a Handle) -> AnalysisResult<ExclusiveTask<'a, N, S>> {
        self.tasks.acquire_exclusive(handle, true)?;

        Ok(ExclusiveTask::new(self, handle))
    }

    pub fn try_exclusive<'a>(
        &'a self,
        handle: &'a Handle,
    ) -> AnalysisResult<ExclusiveTask<'a, N, S>> {
        self.tasks.acquire_exclusive(handle, false)?;

        Ok(ExclusiveTask::new(self, handle))
    }

    pub fn interrupt(&self, tasks_mask: u8) {
        self.tasks.interrupt(tasks_mask);
    }
}
