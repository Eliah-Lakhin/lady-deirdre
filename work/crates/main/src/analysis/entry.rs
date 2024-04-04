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
        Feature,
        Grammar,
        Handle,
        Initializer,
        Invalidator,
        Revision,
        ScopeAttr,
    },
    arena::{Entry, Id, Identifiable, Repo},
    lexis::ToSpan,
    report::debug_unreachable,
    std::*,
    sync::{Shared, SyncBuildHasher, TableReadGuard},
    syntax::{ErrorRef, NodeRef, PolyRef, SyntaxTree},
    units::{Document, Watch},
};

pub type Event = u16;

pub const DOC_ADDED_EVENT: Event = 1;
pub const DOC_REMOVED_EVENT: Event = 2;
pub const DOC_UPDATED_EVENT: Event = 3;
pub const DOC_ERRORS_EVENT: Event = 4;

#[repr(transparent)]
pub struct DocumentReadGuard<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    guard: TableReadGuard<'a, Id, DocEntry<N, S>, S>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Deref for DocumentReadGuard<'a, N, S> {
    type Target = Document<N>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.guard.deref().doc
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> From<TableReadGuard<'a, Id, DocEntry<N, S>, S>>
    for DocumentReadGuard<'a, N, S>
{
    #[inline(always)]
    fn from(guard: TableReadGuard<'a, Id, DocEntry<N, S>, S>) -> Self {
        Self { guard }
    }
}

pub(super) struct DocEntry<N: Grammar, S: SyncBuildHasher> {
    pub(super) doc: Document<N>,
    pub(super) classes_to_nodes: HashMap<<N::Classifier as Classifier>::Class, ClassToNodes<S>, S>,
    pub(super) nodes_to_classes: HashMap<Entry, NodeToClasses<N, S>, S>,
}

pub(super) struct ClassToNodes<S> {
    pub(super) nodes: Shared<HashSet<NodeRef, S>>,
    pub(super) revision: Revision,
}

pub(super) struct NodeToClasses<N: Grammar, S> {
    pub(super) classes: HashSet<<N::Classifier as Classifier>::Class, S>,
}

impl<N: Grammar, S: SyncBuildHasher> Analyzer<N, S> {
    pub(super) fn register_doc(&self, mut doc: Document<N>) -> Id {
        let id = doc.id();

        let node_refs = doc.node_refs().collect::<Vec<_>>();
        let mut records = Repo::with_capacity(node_refs.len());
        let mut classes_to_nodes =
            HashMap::<<N::Classifier as Classifier>::Class, ClassToNodes<S>, S>::default();
        let mut nodes_to_classes = HashMap::<Entry, NodeToClasses<N, S>, S>::default();

        let revision = self.db.commit_revision();

        if !node_refs.is_empty() {
            let mut initializer = Initializer {
                id,
                database: Arc::downgrade(&self.db) as Weak<_>,
                records: &mut records,
            };

            for node_ref in &node_refs {
                let Some(node) = node_ref.deref_mut(&mut doc) else {
                    continue;
                };

                node.init(&mut initializer);
            }

            for node_ref in node_refs {
                let classes = <N::Classifier as Classifier>::classify(&doc, &node_ref);

                if classes.is_empty() {
                    continue;
                }

                for class in &classes {
                    match classes_to_nodes.entry(class.clone()) {
                        HashMapEntry::Occupied(mut entry) => {
                            let Some(nodes) = entry.get_mut().nodes.get_mut() else {
                                // Shared is localized within this function during initialization.
                                unsafe {
                                    debug_unreachable!(
                                        "Class nodes are shared during initialization"
                                    )
                                }
                            };

                            let _ = nodes.insert(node_ref);
                        }

                        HashMapEntry::Vacant(entry) => {
                            let mut nodes = HashSet::default();

                            let _ = nodes.insert(node_ref);

                            let _ = entry.insert(ClassToNodes {
                                nodes: Shared::new(nodes),
                                revision,
                            });
                        }
                    }
                }

                let _ = nodes_to_classes.insert(node_ref.entry, NodeToClasses { classes });
            }
        }

        let _ = self.docs.insert(
            id,
            DocEntry {
                doc,
                classes_to_nodes,
                nodes_to_classes,
            },
        );

        let _ = self.db.records.insert(id, records);

        self.trigger_event(id, DOC_ADDED_EVENT, revision);

        id
    }

    pub(super) fn write_to_doc(
        &self,
        handle: &Handle,
        id: Id,
        span: impl ToSpan,
        text: impl AsRef<str>,
    ) -> AnalysisResult<()> {
        #[derive(Default)]
        struct DocWatch<S> {
            node_refs: HashSet<NodeRef, S>,
            errors_signal: bool,
        }

        impl<S: SyncBuildHasher> Watch for DocWatch<S> {
            #[inline(always)]
            fn report_node(&mut self, node_ref: &NodeRef) {
                let _ = self.node_refs.insert(*node_ref);
            }

            #[inline(always)]
            fn report_error(&mut self, _error_ref: &ErrorRef) {
                self.errors_signal = true
            }
        }
        let Some(mut guard) = self.docs.get_mut(&id) else {
            return Err(AnalysisError::MissingDocument);
        };

        let DocEntry {
            doc,
            classes_to_nodes,
            nodes_to_classes,
        } = guard.deref_mut();

        let Document::Mutable(unit) = doc else {
            return Err(AnalysisError::ImmutableDocument);
        };

        let Some(span) = span.to_site_span(unit) else {
            return Err(AnalysisError::InvalidSpan);
        };

        let mut report = DocWatch::<S>::default();

        unit.write_and_watch(span, text, &mut report);

        if report.node_refs.is_empty() && !report.errors_signal {
            return Ok(());
        }

        let revision = self.db.commit_revision();

        self.trigger_event(id, DOC_UPDATED_EVENT, revision);

        if report.errors_signal {
            self.trigger_event(id, DOC_ERRORS_EVENT, revision);
        }

        let Some(mut records) = self.db.records.get_mut(&id) else {
            // Safety:
            //   1. Records are always in sync with documents.
            //   2. Document is locked.
            unsafe { debug_unreachable!("Missing database entry.") }
        };

        let mut initializer = Initializer {
            id,
            database: Arc::downgrade(&self.db) as Weak<_>,
            records: records.deref_mut(),
        };

        for node_ref in &report.node_refs {
            let Some(node) = node_ref.deref_mut(doc) else {
                let Some(node_to_classes) = nodes_to_classes.remove(&node_ref.entry) else {
                    continue;
                };

                for class in node_to_classes.classes {
                    let Some(class_to_nodes) = classes_to_nodes.get_mut(&class) else {
                        // Safety
                        //   1. Nodes and classes are always in sync.
                        //   2. Both collections locked.
                        unsafe {
                            debug_unreachable!("Nodes and classes resynchronization.");
                        }
                    };

                    class_to_nodes.revision = class_to_nodes.revision.max(revision);

                    let nodes = class_to_nodes.nodes.make_mut();

                    if !nodes.remove(node_ref) {
                        // Safety
                        //   1. Nodes and classes are always in sync.
                        //   2. Both collections locked.
                        unsafe {
                            debug_unreachable!("Nodes and classes resynchronization.");
                        }
                    }
                }

                continue;
            };

            node.init(&mut initializer);
        }

        let mut invalidator = Invalidator {
            id,
            records: records.deref_mut(),
        };

        for node_ref in &report.node_refs {
            let Some(node) = node_ref.deref(doc) else {
                continue;
            };

            let scope_attr = node.scope_attr()?;

            scope_attr.invalidate(&mut invalidator);

            if nodes_to_classes.contains_key(&node_ref.entry) {
                continue;
            }

            let classes = <N as Grammar>::Classifier::classify(doc, node_ref);

            for class in &classes {
                let Some(class_to_nodes) = classes_to_nodes.get_mut(class) else {
                    let mut nodes = HashSet::default();

                    if !nodes.insert(*node_ref) {
                        // Safety: `nodes` is a fresh new collection.
                        unsafe {
                            debug_unreachable!("Duplicate entry.");
                        }
                    }

                    let previous = classes_to_nodes.insert(
                        class.clone(),
                        ClassToNodes {
                            nodes: Shared::new(nodes),
                            revision,
                        },
                    );

                    if previous.is_some() {
                        // Safety: Existence checked above.
                        unsafe {
                            debug_unreachable!("Duplicate entry.");
                        }
                    }

                    continue;
                };

                let nodes = class_to_nodes.nodes.make_mut();

                if !nodes.insert(*node_ref) {
                    // Safety
                    //   1. Nodes and classes are always in sync.
                    //   2. Both collections locked.
                    unsafe {
                        debug_unreachable!("Nodes and classes resynchronization.");
                    }
                }

                class_to_nodes.revision = class_to_nodes.revision.max(revision);
            }

            if nodes_to_classes
                .insert(node_ref.entry, NodeToClasses { classes })
                .is_some()
            {
                // Safety: Existence checked above.
                unsafe {
                    debug_unreachable!("Duplicate entry.");
                }
            }
        }

        let mut scope_accumulator = HashSet::<NodeRef, S>::default();

        for node_ref in &report.node_refs {
            let Some(node) = node_ref.deref(doc) else {
                continue;
            };

            let scope_attr = node.scope_attr()?;
            let scope_attr_ref = scope_attr.as_ref();

            // Safety: `scope_attr_ref` belongs to `scope_attr`.
            let scope_ref = unsafe {
                ScopeAttr::snapshot_manually(
                    scope_attr_ref,
                    handle,
                    doc,
                    records.deref(),
                    revision,
                )?
            };

            if !scope_ref.is_nil() {
                scope_accumulator.insert(scope_ref);
            }
        }

        if !scope_accumulator.is_empty() {
            let mut invalidator = Invalidator {
                id,
                records: records.deref_mut(),
            };

            for scope_ref in scope_accumulator {
                let Some(node) = scope_ref.deref(doc) else {
                    continue;
                };

                node.invalidate(&mut invalidator);
            }
        }

        Ok(())
    }

    pub(super) fn remove_doc(&self, id: Id) -> bool {
        if self.docs.remove(&id).is_none() {
            return false;
        }

        if self.db.records.remove(&id).is_none() {
            // Safety: records are always in sync with documents.
            unsafe { debug_unreachable!("Missing database entry.") }
        }

        let revision = self.db.commit_revision();

        self.trigger_event(id, DOC_REMOVED_EVENT, revision);

        true
    }

    pub(super) fn trigger_event(&self, id: Id, event: Event, revision: Revision) {
        {
            let mut guard = self.events.entry(Id::nil()).or_default();

            let event_revision = guard.entry(event).or_default();

            *event_revision = revision.max(*event_revision);
        }

        if !id.is_nil() {
            let mut guard = self.events.entry(id).or_default();

            let event_revision = guard.entry(event).or_default();

            *event_revision = revision.max(*event_revision);
        }
    }
}
