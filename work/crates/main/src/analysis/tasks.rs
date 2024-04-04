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
        Analyzer,
        Classifier,
        DocumentReadGuard,
        Event,
        Grammar,
        Handle,
        Revision,
    },
    arena::Id,
    lexis::{ToSpan, TokenBuffer},
    std::*,
    sync::{Shared, SyncBuildHasher},
    syntax::NodeRef,
    units::Document,
};

pub struct AnalysisTask<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    analyzer: &'a Analyzer<N, S>,
    revision: Revision,
    handle: &'a Handle,
}

impl<'a, N: Grammar, S: SyncBuildHasher> SemanticAccess<N, S> for AnalysisTask<'a, N, S> {}

impl<'a, N: Grammar, S: SyncBuildHasher> AbstractTask<N, S> for AnalysisTask<'a, N, S> {
    #[inline(always)]
    fn handle(&self) -> &Handle {
        self.handle
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> TaskSealed<N, S> for AnalysisTask<'a, N, S> {
    #[inline(always)]
    fn analyzer(&self) -> &Analyzer<N, S> {
        self.analyzer
    }

    #[inline(always)]
    fn revision(&self) -> Revision {
        self.revision
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Drop for AnalysisTask<'a, N, S> {
    fn drop(&mut self) {
        self.analyzer.tasks.release_analysis(self.handle);
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> AnalysisTask<'a, N, S> {
    #[inline(always)]
    pub(super) fn new(analyzer: &'a Analyzer<N, S>, handle: &'a Handle) -> Self {
        Self {
            analyzer,
            revision: analyzer.db.load_revision(),
            handle,
        }
    }
}

pub struct MutationTask<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    analyzer: &'a Analyzer<N, S>,
    handle: &'a Handle,
}

impl<'a, N: Grammar, S: SyncBuildHasher> MutationAccess<N, S> for MutationTask<'a, N, S> {}

impl<'a, N: Grammar, S: SyncBuildHasher> AbstractTask<N, S> for MutationTask<'a, N, S> {
    #[inline(always)]
    fn handle(&self) -> &Handle {
        self.handle
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> TaskSealed<N, S> for MutationTask<'a, N, S> {
    #[inline(always)]
    fn analyzer(&self) -> &Analyzer<N, S> {
        self.analyzer
    }

    #[inline(always)]
    fn revision(&self) -> Revision {
        self.analyzer.db.load_revision()
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Drop for MutationTask<'a, N, S> {
    fn drop(&mut self) {
        self.analyzer.tasks.release_mutation(self.handle);
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> MutationTask<'a, N, S> {
    #[inline(always)]
    pub(super) fn new(analyzer: &'a Analyzer<N, S>, handle: &'a Handle) -> Self {
        Self { analyzer, handle }
    }
}

pub struct ExclusiveTask<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    analyzer: &'a Analyzer<N, S>,
    handle: &'a Handle,
}

impl<'a, N: Grammar, S: SyncBuildHasher> SemanticAccess<N, S> for ExclusiveTask<'a, N, S> {}

impl<'a, N: Grammar, S: SyncBuildHasher> MutationAccess<N, S> for ExclusiveTask<'a, N, S> {}

impl<'a, N: Grammar, S: SyncBuildHasher> AbstractTask<N, S> for ExclusiveTask<'a, N, S> {
    #[inline(always)]
    fn handle(&self) -> &Handle {
        self.handle
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> TaskSealed<N, S> for ExclusiveTask<'a, N, S> {
    #[inline(always)]
    fn analyzer(&self) -> &Analyzer<N, S> {
        self.analyzer
    }

    #[inline(always)]
    fn revision(&self) -> Revision {
        self.analyzer.db.load_revision()
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Drop for ExclusiveTask<'a, N, S> {
    fn drop(&mut self) {
        self.analyzer.tasks.release_exclusive(self.handle);
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> ExclusiveTask<'a, N, S> {
    #[inline(always)]
    pub(super) fn new(analyzer: &'a Analyzer<N, S>, handle: &'a Handle) -> Self {
        Self { analyzer, handle }
    }
}

pub trait MutationAccess<N: Grammar, S: SyncBuildHasher>: AbstractTask<N, S> {
    #[inline(always)]
    fn add_mutable_doc(&mut self, text: impl Into<TokenBuffer<N::Token>>) -> Id {
        self.analyzer().register_doc(Document::new_mutable(text))
    }

    #[inline(always)]
    fn add_immutable_doc(&mut self, text: impl Into<TokenBuffer<N::Token>>) -> Id {
        self.analyzer().register_doc(Document::new_immutable(text))
    }

    #[inline(always)]
    fn write_to_doc(
        &mut self,
        id: Id,
        span: impl ToSpan,
        text: impl AsRef<str>,
    ) -> AnalysisResult<()> {
        self.analyzer().write_to_doc(self.handle(), id, span, text)
    }

    #[inline(always)]
    fn remove_doc(&mut self, id: Id) -> bool {
        self.analyzer().remove_doc(id)
    }

    #[inline(always)]
    fn trigger_event(&mut self, id: Id, event: Event) {
        let revision = self.analyzer().db.commit_revision();

        self.analyzer().trigger_event(id, event, revision)
    }
}

pub trait SemanticAccess<N: Grammar, S: SyncBuildHasher>: AbstractTask<N, S> {}

pub trait AbstractTask<N: Grammar, S: SyncBuildHasher>: TaskSealed<N, S> {
    fn handle(&self) -> &Handle;

    #[inline(always)]
    fn proceed(&self) -> AnalysisResult<()> {
        if self.handle().triggered() {
            return Err(AnalysisError::Interrupted);
        }

        Ok(())
    }

    #[inline(always)]
    fn contains_doc(&self, id: Id) -> bool {
        self.analyzer().docs.contains_key(&id)
    }

    #[inline(always)]
    fn read_doc(&self, id: Id) -> AnalysisResult<DocumentReadGuard<N, S>> {
        let Some(guard) = self.analyzer().docs.get(&id) else {
            return Err(AnalysisError::MissingDocument);
        };

        Ok(DocumentReadGuard::from(guard))
    }

    #[inline(always)]
    fn try_read_doc(&self, id: Id) -> Option<DocumentReadGuard<N, S>> {
        Some(DocumentReadGuard::from(self.analyzer().docs.try_get(&id)?))
    }

    #[inline(always)]
    fn is_doc_mutable(&self, id: Id) -> bool {
        let Some(guard) = self.analyzer().docs.get(&id) else {
            return false;
        };

        guard.doc.is_mutable()
    }

    #[inline(always)]
    fn is_doc_immutable(&self, id: Id) -> bool {
        let Some(guard) = self.analyzer().docs.get(&id) else {
            return false;
        };

        guard.doc.is_mutable()
    }

    #[inline(always)]
    fn snapshot_class(
        &self,
        id: Id,
        class: &<N::Classifier as Classifier>::Class,
    ) -> AnalysisResult<Shared<HashSet<NodeRef, S>>> {
        let Some(guard) = self.analyzer().docs.get(&id) else {
            return Err(AnalysisError::MissingDocument);
        };

        let Some(class_to_nodes) = guard.classes_to_nodes.get(class) else {
            return Ok(Shared::default());
        };

        Ok(class_to_nodes.nodes.clone())
    }
}

pub trait TaskSealed<N: Grammar, S: SyncBuildHasher> {
    fn analyzer(&self) -> &Analyzer<N, S>;

    fn revision(&self) -> Revision;
}
