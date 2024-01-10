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
        analyzer::DocEntry,
        AnalysisError,
        AnalysisResult,
        Analyzer,
        AttrContext,
        DocumentReadGuard,
        FeatureInitializer,
        FeatureInvalidator,
        Grammar,
        Revision,
    },
    arena::{Id, Identifiable},
    lexis::{ToSpan, TokenBuffer},
    report::{debug_unreachable, system_panic},
    std::*,
    sync::{Latch, Lazy, Shared, SyncBuildHasher},
    syntax::{ErrorRef, NodeRef, PolyRef, SyntaxTree},
    units::{Document, Watch},
};

pub struct AnalysisTask<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    analyzer: &'a Analyzer<N, S>,
    revision: Revision,
    handle: &'a Latch,
}

impl<'a, N: Grammar, S: SyncBuildHasher> SemanticAccess<N, S> for AnalysisTask<'a, N, S> {}

impl<'a, N: Grammar, S: SyncBuildHasher> AbstractTask<N, S> for AnalysisTask<'a, N, S> {
    #[inline(always)]
    fn handle(&self) -> &Latch {
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
    pub(super) fn new(analyzer: &'a Analyzer<N, S>, handle: &'a Latch) -> Self {
        Self {
            analyzer,
            revision: analyzer.database.revision(),
            handle,
        }
    }
}

pub struct MutationTask<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    analyzer: &'a Analyzer<N, S>,
    handle: &'a Latch,
}

impl<'a, N: Grammar, S: SyncBuildHasher> MutationAccess<N, S> for MutationTask<'a, N, S> {}

impl<'a, N: Grammar, S: SyncBuildHasher> AbstractTask<N, S> for MutationTask<'a, N, S> {
    #[inline(always)]
    fn handle(&self) -> &Latch {
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
        self.analyzer.database.revision()
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Drop for MutationTask<'a, N, S> {
    fn drop(&mut self) {
        self.analyzer.tasks.release_mutation(self.handle);
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> MutationTask<'a, N, S> {
    #[inline(always)]
    pub(super) fn new(analyzer: &'a Analyzer<N, S>, handle: &'a Latch) -> Self {
        Self { analyzer, handle }
    }
}

pub struct ExclusiveTask<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    analyzer: &'a Analyzer<N, S>,
    handle: &'a Latch,
}

impl<'a, N: Grammar, S: SyncBuildHasher> SemanticAccess<N, S> for ExclusiveTask<'a, N, S> {}

impl<'a, N: Grammar, S: SyncBuildHasher> MutationAccess<N, S> for ExclusiveTask<'a, N, S> {}

impl<'a, N: Grammar, S: SyncBuildHasher> AbstractTask<N, S> for ExclusiveTask<'a, N, S> {
    #[inline(always)]
    fn handle(&self) -> &Latch {
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
        self.analyzer.database.revision()
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Drop for ExclusiveTask<'a, N, S> {
    fn drop(&mut self) {
        self.analyzer.tasks.release_exclusive(self.handle);
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> ExclusiveTask<'a, N, S> {
    #[inline(always)]
    pub(super) fn new(analyzer: &'a Analyzer<N, S>, handle: &'a Latch) -> Self {
        Self { analyzer, handle }
    }
}

pub trait MutationAccess<N: Grammar, S: SyncBuildHasher>: AbstractTask<N, S> {
    fn add_mutable_doc(&mut self, text: impl Into<TokenBuffer<N::Token>>) -> Id {
        let document = Document::new_mutable(text);

        let id = document.id();

        self.analyzer().register_doc(document);

        id
    }

    #[inline(always)]
    fn add_immutable_doc(&mut self, text: impl Into<TokenBuffer<N::Token>>) -> Id {
        let document = Document::new_immutable(text);
        let id = document.id();

        self.analyzer().register_doc(document);

        id
    }

    fn write_to_doc(
        &mut self,
        id: Id,
        span: impl ToSpan,
        text: impl AsRef<str>,
    ) -> AnalysisResult<()> {
        #[derive(Default)]
        struct DocWatch {
            node_refs: Vec<NodeRef>,
            error_refs: Vec<ErrorRef>,
        }

        impl Watch for DocWatch {
            #[inline(always)]
            fn report_node(&mut self, node_ref: &NodeRef) {
                self.node_refs.push(*node_ref);
            }

            #[inline(always)]
            fn report_error(&mut self, error_ref: &ErrorRef) {
                self.error_refs.push(*error_ref);
            }
        }

        let mutations = {
            let Some(mut guard) = self.analyzer().docs.get_mut(id) else {
                return Err(AnalysisError::MissingDocument);
            };

            let DocEntry {
                document,
                scope_accumulator,
                error_accumulator,
            } = guard.deref_mut();

            let Document::Mutable(unit) = document else {
                return Err(AnalysisError::ImmutableDocument);
            };

            let Some(span) = span.to_site_span(unit) else {
                return Err(AnalysisError::InvalidSpan);
            };

            let mut report = DocWatch::default();

            unit.write_and_watch(span, text, &mut report);

            if report.node_refs.is_empty() && report.error_refs.is_empty() {
                return Ok(());
            }

            let Some(mut records) = self.analyzer().database.records.get_mut(id) else {
                // Safety:
                //   1. Records are always in sync with documents.
                //   2. Document is locked.
                unsafe { debug_unreachable!("Missing database entry.") }
            };

            let mut initializer = FeatureInitializer {
                id,
                database: Arc::downgrade(&self.analyzer().database) as Weak<_>,
                records: records.deref_mut(),
            };

            let mut trigger_root = false;

            for node_ref in &report.node_refs {
                let Some(node) = node_ref.deref_mut(document) else {
                    if scope_accumulator.as_ref().contains(node_ref) {
                        let _ = scope_accumulator.make_mut().remove(node_ref);
                        trigger_root = true;
                    }

                    continue;
                };

                match (
                    node.is_scope(),
                    scope_accumulator.as_ref().contains(node_ref),
                ) {
                    (true, false) => {
                        let _ = scope_accumulator.make_mut().insert(*node_ref);
                        trigger_root = true;
                    }

                    (false, true) => {
                        let _ = scope_accumulator.make_mut().remove(node_ref);
                        trigger_root = true;
                    }

                    _ => (),
                }

                if node.is_scope() {
                    if !scope_accumulator.as_ref().contains(node_ref) {
                        let _ = scope_accumulator.make_mut().insert(*node_ref);
                        trigger_root = true;
                    }
                }

                node.initialize(&mut initializer);
            }

            for error_ref in error_accumulator.clone().as_ref() {
                if !error_ref.is_valid_ref(document) {
                    let _ = error_accumulator.make_mut().remove(&error_ref);
                    trigger_root = true;
                }
            }

            for error_ref in report.error_refs {
                match error_ref.is_valid_ref(document) {
                    true => {
                        if !error_accumulator.as_ref().contains(&error_ref) {
                            let _ = error_accumulator.make_mut().insert(error_ref);
                            trigger_root = true;
                        }
                    }

                    false => {
                        if error_accumulator.as_ref().contains(&error_ref) {
                            let _ = error_accumulator.make_mut().remove(&error_ref);
                            trigger_root = true;
                        }
                    }
                }
            }

            let mut invalidator = FeatureInvalidator {
                id,
                records: records.deref_mut(),
            };

            if trigger_root {
                //todo this action will invalidate entire root node semantics.
                //     consider introducing additional feature markers for global triggers only.
                document.root().invalidate(&mut invalidator);
            }

            for node_ref in &report.node_refs {
                let Some(node) = node_ref.deref(document) else {
                    continue;
                };

                node.invalidate(&mut invalidator);
            }

            self.analyzer().database.commit();

            report.node_refs
        };

        if !N::has_scopes() {
            return Ok(());
        }

        if mutations.is_empty() {
            return Ok(());
        }

        let Some(guard) = self.analyzer().docs.get(id) else {
            return Ok(());
        };

        let DocEntry { document, .. } = guard.deref();

        if mutations.is_empty() {
            return Ok(());
        }

        let mut context = AttrContext::new(self.analyzer(), self.revision(), {
            static DUMMY: Lazy<Latch> = Lazy::new(Latch::new);

            #[cfg(debug_assertions)]
            if DUMMY.get_relaxed() {
                system_panic!("Dummy handle cancelled");
            }

            &DUMMY
        });

        let mut scope_refs = HashSet::with_capacity_and_hasher(1, S::default());

        for node_ref in &mutations {
            let Some(node) = node_ref.deref(document) else {
                continue;
            };

            let scope_attr = node.scope_attr()?;

            let scope_ref = scope_attr.read(&mut context)?.scope_ref;
            context.reset_deps();

            if scope_ref.is_nil() {
                continue;
            }

            let _ = scope_refs.insert(scope_ref);
        }

        if scope_refs.is_empty() {
            return Ok(());
        }

        let Some(mut records) = self.analyzer().database.records.get_mut(id) else {
            // Safety:
            //   1. Records are always in sync with documents.
            //   2. Document is locked.
            unsafe { debug_unreachable!("Missing database entry.") }
        };

        let mut invalidator = FeatureInvalidator {
            id,
            records: records.deref_mut(),
        };

        for node_ref in scope_refs {
            let Some(node) = node_ref.deref(document) else {
                continue;
            };

            node.invalidate(&mut invalidator);
        }

        self.analyzer().database.commit();

        Ok(())
    }

    #[inline(always)]
    fn remove_doc(&mut self, id: Id) -> bool {
        if !self.analyzer().docs.remove(id) {
            return false;
        }

        if !self.analyzer().database.records.remove(id) {
            // Safety: records are always in sync with documents.
            unsafe { debug_unreachable!("Missing database entry.") }
        }

        self.analyzer().database.commit();

        true
    }
}

pub trait SemanticAccess<N: Grammar, S: SyncBuildHasher>: AbstractTask<N, S> {}

pub trait AbstractTask<N: Grammar, S: SyncBuildHasher>: TaskSealed<N, S> {
    fn handle(&self) -> &Latch;

    #[inline(always)]
    fn proceed(&self) -> AnalysisResult<()> {
        if self.handle().get_relaxed() {
            return Err(AnalysisError::Interrupted);
        }

        Ok(())
    }

    #[inline(always)]
    fn read_doc(&self, id: Id) -> AnalysisResult<DocumentReadGuard<N, S>> {
        let Some(guard) = self.analyzer().docs.get(id) else {
            return Err(AnalysisError::MissingDocument);
        };

        Ok(DocumentReadGuard::from(guard))
    }

    #[inline(always)]
    fn try_read_doc(&self, id: Id) -> Option<DocumentReadGuard<N, S>> {
        Some(DocumentReadGuard::from(self.analyzer().docs.try_get(id)?))
    }

    #[inline(always)]
    fn snapshot_scopes(&self, id: Id) -> AnalysisResult<Shared<HashSet<NodeRef, S>>> {
        let Some(guard) = self.analyzer().docs.get(id) else {
            return Err(AnalysisError::MissingDocument);
        };

        Ok(guard.scope_accumulator.clone())
    }

    #[inline(always)]
    fn snapshot_errors(&self, id: Id) -> AnalysisResult<Shared<HashSet<ErrorRef, S>>> {
        let Some(guard) = self.analyzer().docs.get(id) else {
            return Err(AnalysisError::MissingDocument);
        };

        Ok(guard.error_accumulator.clone())
    }

    #[inline(always)]
    fn contains_doc(&self, id: Id) -> bool {
        self.analyzer().docs.contains(id)
    }

    #[inline(always)]
    fn is_doc_mutable(&self, id: Id) -> bool {
        let Some(guard) = self.analyzer().docs.get(id) else {
            return false;
        };

        guard.document.is_mutable()
    }

    #[inline(always)]
    fn is_doc_immutable(&self, id: Id) -> bool {
        let Some(guard) = self.analyzer().docs.get(id) else {
            return false;
        };

        guard.document.is_mutable()
    }
}

pub trait TaskSealed<N: Grammar, S: SyncBuildHasher> {
    fn analyzer(&self) -> &Analyzer<N, S>;

    fn revision(&self) -> Revision;
}
