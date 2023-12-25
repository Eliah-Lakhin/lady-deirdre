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
        tasks::Exclusivity,
        AbstractTask,
        AnalysisResult,
        Analyzer,
        Attr,
        AttrContext,
        AttrReadGuard,
        AttrRef,
        Computable,
        Grammar,
        Revision,
    },
    std::*,
    sync::{Latch, SyncBuildHasher},
};

pub struct AnalysisTask<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    exclusivity: Exclusivity,
    analyzer: &'a Analyzer<N, S>,
    revision: Revision,
    handle: &'a Latch,
}

impl<'a, N: Grammar, S: SyncBuildHasher> AbstractTask<'a, N, S> for AnalysisTask<'a, N, S> {
    #[inline(always)]
    fn analyzer(&self) -> &'a Analyzer<N, S> {
        self.analyzer
    }

    #[inline(always)]
    fn handle(&self) -> &'a Latch {
        self.handle
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Drop for AnalysisTask<'a, N, S> {
    fn drop(&mut self) {
        if let Exclusivity::NonExclusive = &self.exclusivity {
            self.analyzer.tasks.release_analysis(self.handle);
        }
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> AnalysisTask<'a, N, S> {
    #[inline(always)]
    pub(super) fn new_non_exclusive(analyzer: &'a Analyzer<N, S>, handle: &'a Latch) -> Self {
        Self {
            exclusivity: Exclusivity::NonExclusive,
            analyzer,
            revision: analyzer.database.revision(),
            handle,
        }
    }

    #[inline(always)]
    pub(super) fn new_exclusive(analyzer: &'a Analyzer<N, S>, handle: &'a Latch) -> Self {
        Self {
            exclusivity: Exclusivity::Exclusive,
            analyzer,
            revision: analyzer.database.revision(),
            handle,
        }
    }

    #[inline(always)]
    pub fn read_attr<C: Computable<Node = N>>(
        &self,
        attr: &Attr<C>,
    ) -> AnalysisResult<AttrReadGuard<C, S>> {
        let mut reader = AttrContext::for_analysis_task(self);
        attr.query(&mut reader)
    }

    #[inline(always)]
    pub fn read_attr_ref<C: Computable<Node = N>>(
        &self,
        attr: &AttrRef,
    ) -> AnalysisResult<AttrReadGuard<C, S>> {
        let mut reader = AttrContext::for_analysis_task(self);
        attr.query(&mut reader)
    }

    #[inline(always)]
    pub(super) fn db_revision(&self) -> Revision {
        self.revision
    }
}
