////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{
    collections::{hash_map, HashMap, HashSet},
    hash::RandomState,
    ops::{Deref, DerefMut},
    sync::{Arc, Weak},
};

use crate::{
    analysis::{
        database::DocRecords,
        AnalysisError,
        AnalysisResult,
        Analyzer,
        Classifier,
        Feature,
        Grammar,
        Initializer,
        Invalidator,
        Revision,
        ScopeAttr,
        TaskHandle,
    },
    arena::{Entry, Id, Identifiable, Repo},
    lexis::ToSpan,
    report::ld_unreachable,
    sync::{Shared, SyncBuildHasher, TableReadGuard},
    syntax::{ErrorRef, NodeRef, PolyRef, SyntaxTree},
    units::{Document, Watcher},
};

/// A type of the [Analyzer]-wide event.
///
/// The values lesser than [CUSTOM_EVENT_START_RANGE] are reserved by the crate.
/// Other values are custom user-defined events.
pub type Event = u16;

/// A built-in [Analyzer] event indicating the the document has been added
/// to the Analyzer.
pub const DOC_ADDED_EVENT: Event = 1;

/// A built-in [Analyzer] event indicating that the document has been
/// [removed](crate::analysis::MutationAccess::remove_doc) from the Analyzer.
pub const DOC_REMOVED_EVENT: Event = 2;

/// A built-in [Analyzer] event indicating that the document's content has been
/// [edited](crate::analysis::MutationAccess::write_to_doc).
pub const DOC_UPDATED_EVENT: Event = 3;

/// A built-in [Analyzer] event indicating that
/// the [syntax error](crate::syntax::SyntaxError) has occurred or been removed
/// from the [syntax tree](SyntaxTree) of the Analyzer's document.
pub const DOC_ERRORS_EVENT: Event = 4;

/// A start of the custom user-defined [events](Event) range.
pub const CUSTOM_EVENT_START_RANGE: Event = 0x100;

/// A RAII guard that provides read-only access to the [Analyzer]'s document.
///
/// The underlying document can be accessed through the [Deref] implementation
/// of this object.
///
/// The document is locked for read until the last remaining DocumentReadGuard
/// is dropped. If a task attempts
/// to [write](crate::analysis::MutationAccess::write_to_doc) into the document
/// while the document is locked for read, the writer thread will be blocked.
///
/// Note the Analyzer allows reading document's
/// [attributes](crate::analysis::Attr) while document is locked for read.
///
/// Also, the Analyzer allows parallel writing to independent documents
/// without blocking if these documents are not locked for read.
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

impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> Analyzer<N, H, S> {
    pub(super) fn register_doc(&self, mut doc: Document<N>) -> Id {
        let id = doc.id();

        let node_refs = doc.node_refs().collect::<Vec<_>>();
        let mut records = DocRecords::with_capacity(node_refs.len());
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
                        hash_map::Entry::Occupied(mut entry) => {
                            let Some(nodes) = entry.get_mut().nodes.get_mut() else {
                                // Shared is localized within this function during initialization.
                                unsafe {
                                    ld_unreachable!("Class nodes are shared during initialization")
                                }
                            };

                            let _ = nodes.insert(node_ref);
                        }

                        hash_map::Entry::Vacant(entry) => {
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
        handle: &H,
        id: Id,
        span: impl ToSpan,
        text: impl AsRef<str>,
    ) -> AnalysisResult<()> {
        #[derive(Default)]
        struct DocWatcher<S> {
            node_refs: HashSet<NodeRef, S>,
            errors_signal: bool,
        }

        impl<S: SyncBuildHasher> Watcher for DocWatcher<S> {
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

        let mut report = DocWatcher::<S>::default();

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
            unsafe { ld_unreachable!("Missing database entry.") }
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
                            ld_unreachable!("Nodes and classes resynchronization.");
                        }
                    };

                    class_to_nodes.revision = class_to_nodes.revision.max(revision);

                    let nodes = class_to_nodes.nodes.make_mut();

                    if !nodes.remove(node_ref) {
                        // Safety
                        //   1. Nodes and classes are always in sync.
                        //   2. Both collections locked.
                        unsafe {
                            ld_unreachable!("Nodes and classes resynchronization.");
                        }
                    }
                }

                continue;
            };

            node.init(&mut initializer);
        }

        let mut invalidator = Invalidator {
            id,
            records: &mut records.attrs,
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
                            ld_unreachable!("Duplicate entry.");
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
                            ld_unreachable!("Duplicate entry.");
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
                        ld_unreachable!("Nodes and classes resynchronization.");
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
                    ld_unreachable!("Duplicate entry.");
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
                ScopeAttr::snapshot_manually(scope_attr_ref, handle, doc, &records.attrs, revision)?
            };

            if !scope_ref.is_nil() {
                scope_accumulator.insert(scope_ref);
            }
        }

        if !scope_accumulator.is_empty() {
            let mut invalidator = Invalidator {
                id,
                records: &mut records.attrs,
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
            unsafe { ld_unreachable!("Missing database entry.") }
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
