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
        attribute::Computable,
        database::AbstractDatabase,
        record::Record,
        tasks::Exclusivity,
        AbstractTask,
        AnalysisError,
        AnalysisResult,
        AnalysisTask,
        Analyzer,
        AttrContext,
        Grammar,
    },
    arena::{Entry, Id, Identifiable, Repository},
    lexis::{ToSpan, TokenBuffer},
    report::debug_unreachable,
    std::*,
    sync::{Latch, SyncBuildHasher},
    syntax::{NodeRef, PolyRef, SyntaxTree},
    units::{Document, MutableUnit},
};

pub struct MutationTask<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    exclusivity: Exclusivity,
    analyzer: &'a Analyzer<N, S>,
    handle: &'a Latch,
}

impl<'a, N: Grammar, S: SyncBuildHasher> AbstractTask<'a, N, S> for MutationTask<'a, N, S> {
    #[inline(always)]
    fn analyzer(&self) -> &'a Analyzer<N, S> {
        self.analyzer
    }

    #[inline(always)]
    fn handle(&self) -> &'a Latch {
        self.handle
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Drop for MutationTask<'a, N, S> {
    fn drop(&mut self) {
        match &self.exclusivity {
            Exclusivity::NonExclusive => {
                self.analyzer.tasks.release_mutation(self.handle);
            }

            Exclusivity::Exclusive => {
                self.analyzer.tasks.release_exclusive(self.handle);
            }
        }
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> MutationTask<'a, N, S> {
    #[inline(always)]
    pub(super) fn new_non_exclusive(analyzer: &'a Analyzer<N, S>, handle: &'a Latch) -> Self {
        Self {
            exclusivity: Exclusivity::NonExclusive,
            analyzer,
            handle,
        }
    }

    #[inline(always)]
    pub(super) fn new_exclusive(analyzer: &'a Analyzer<N, S>, handle: &'a Latch) -> Self {
        Self {
            exclusivity: Exclusivity::Exclusive,
            analyzer,
            handle,
        }
    }

    pub fn add_mutable_document(&self, text: impl Into<TokenBuffer<N::Token>>) -> Id {
        let document = {
            let mut unit = MutableUnit::new(text, false);

            unit.mutations().watch(true);

            Document::from(unit)
        };

        let id = document.id();

        self.add_document(document);

        id
    }

    #[inline(always)]
    pub fn add_immutable_document(&self, text: impl Into<TokenBuffer<N::Token>>) -> Id {
        let document = Document::new_immutable(text);
        let id = document.id();

        self.add_document(document);

        id
    }

    pub fn write_to_document(
        &self,
        id: Id,
        span: impl ToSpan,
        text: impl AsRef<str>,
    ) -> AnalysisResult<()> {
        let mutations = {
            let Some(mut document) = self.analyzer.documents.get_mut(id) else {
                return Err(AnalysisError::MissingDocument);
            };

            let Document::Mutable(unit) = &mut document.deref_mut() else {
                return Err(AnalysisError::ImmutableDocument);
            };

            let Some(span) = span.to_site_span(unit) else {
                return Err(AnalysisError::InvalidSpan);
            };

            if unit.write(span, text).is_nil() {
                return Ok(());
            }

            let mutations = unit.mutations().take();

            let Some(mut records) = self.analyzer.database.records.get_mut(id) else {
                // Safety:
                //   1. Records are always in sync with documents.
                //   2. Document is locked.
                unsafe { debug_unreachable!("Missing database entry.") }
            };

            let mut initializer = FeatureInitializer {
                id,
                database: Arc::downgrade(&self.analyzer.database) as Weak<_>,
                records: records.deref_mut(),
            };

            for node_ref in &mutations {
                let Some(node) = node_ref.deref_mut(document.deref_mut()) else {
                    continue;
                };

                node.initialize(&mut initializer);
            }

            let mut invalidator = FeatureInvalidator {
                id,
                records: records.deref_mut(),
            };

            for node_ref in &mutations {
                let Some(node) = node_ref.deref(document.deref()) else {
                    continue;
                };

                node.invalidate(&mut invalidator);
            }

            self.analyzer.database.commit();

            mutations
        };

        if !N::has_scopes() {
            return Ok(());
        }

        if mutations.is_empty() {
            return Ok(());
        }

        let Some(document) = self.analyzer.documents.get(id) else {
            return Ok(());
        };

        let mut context = AttrContext::for_mutation_task(self);

        let mut scope_refs = HashSet::with_capacity_and_hasher(1, S::default());

        for node_ref in &mutations {
            let Some(node) = node_ref.deref(document.deref()) else {
                continue;
            };

            let scope_attr = node.scope_attr()?;

            let scope_ref = scope_attr.query(&mut context)?.scope_ref;
            context.reset_deps();

            if scope_ref.is_nil() {
                continue;
            }

            let _ = scope_refs.insert(scope_ref);
        }

        if scope_refs.is_empty() {
            return Ok(());
        }

        let Some(mut records) = self.analyzer.database.records.get_mut(id) else {
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
            let Some(node) = node_ref.deref(document.deref()) else {
                continue;
            };

            node.invalidate(&mut invalidator);
        }

        self.analyzer.database.commit();

        Ok(())
    }

    #[inline(always)]
    pub fn remove_document(&self, id: Id) -> bool {
        if !self.analyzer.documents.remove(id) {
            return false;
        }

        if !self.analyzer.database.records.remove(id) {
            // Safety: records are always in sync with documents.
            unsafe { debug_unreachable!("Missing database entry.") }
        }

        self.analyzer.database.commit();

        true
    }

    #[inline(always)]
    pub fn analyze(&mut self) -> AnalysisTask<'a, N, S> {
        let Exclusivity::Exclusive = &self.exclusivity else {
            panic!("An attempt to fork non-exclusive mutation task for analysis.");
        };

        AnalysisTask::new_exclusive(self.analyzer, self.handle)
    }

    fn add_document(&self, mut document: Document<N>) {
        let id = document.id();

        let node_refs = document.node_refs().collect::<Vec<_>>();
        let mut records = Repository::with_capacity(node_refs.len());

        if !node_refs.is_empty() {
            let mut initializer = FeatureInitializer {
                id,
                database: Arc::downgrade(&self.analyzer.database) as Weak<_>,
                records: &mut records,
            };

            for node_ref in node_refs {
                let Some(node) = node_ref.deref_mut(&mut document) else {
                    continue;
                };

                node.initialize(&mut initializer);
            }

            self.analyzer.database.commit();
        }

        // Safety: Ids are globally unique.
        unsafe {
            self.analyzer.documents.insert(id, document);
        }

        // Safety: records are always in sync with documents.
        unsafe {
            self.analyzer.database.records.insert(id, records);
        }
    }
}

pub struct FeatureInitializer<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    id: Id,
    database: Weak<dyn AbstractDatabase>,
    records: &'a mut Repository<Record<N, S>>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Identifiable for FeatureInitializer<'a, N, S> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> FeatureInitializer<'a, N, S> {
    #[inline(always)]
    pub(super) fn register_attribute<C: Computable<Node = N> + Eq>(
        &mut self,
        node_ref: NodeRef,
    ) -> (Weak<dyn AbstractDatabase>, Entry) {
        (
            self.database.clone(),
            self.records.insert(Record::new::<C>(node_ref)),
        )
    }
}

pub struct FeatureInvalidator<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    id: Id,
    records: &'a mut Repository<Record<N, S>>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Identifiable for FeatureInvalidator<'a, N, S> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> FeatureInvalidator<'a, N, S> {
    #[inline(always)]
    pub(super) fn invalidate_attribute(&mut self, entry: &Entry) {
        let Some(record) = self.records.get(entry) else {
            return;
        };

        record.invalidate();
    }
}
