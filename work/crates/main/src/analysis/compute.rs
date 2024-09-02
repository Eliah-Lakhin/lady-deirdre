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
    collections::HashSet,
    fmt::{Debug, Display, Formatter},
    hash::RandomState,
    mem::transmute,
    ops::{Deref, DerefMut},
};

use crate::{
    analysis::{
        database::{
            AttrRecord,
            AttrRecordCache,
            AttrRecordData,
            CacheDeps,
            DocRecords,
            SlotRecordData,
        },
        lock::TimeoutRwLockReadGuard,
        AnalysisError,
        AnalysisResult,
        Analyzer,
        AttrRef,
        Classifier,
        DocumentReadGuard,
        Event,
        Grammar,
        MutationAccess,
        Revision,
        SlotRef,
        TaskHandle,
        TriggerHandle,
        DOC_REMOVED_EVENT,
        DOC_UPDATED_EVENT,
    },
    arena::{Id, Repo},
    report::ld_unreachable,
    sync::{Shared, SyncBuildHasher, TableReadGuard},
    syntax::{NodeRef, NIL_NODE_REF},
};

/// A function that computes [attribute](crate::analysis::Attr)'s value.
///
/// By implementing this trait on a custom type `T` of the programming language
/// semantic model, the type `T` becomes eligible as
/// an [Attr](crate::analysis::Attr)'s parameter.
///
/// The [compute](Computable::compute) function infers a particular fact (or a
/// set of facts) of the semantic model from the syntax tree and other
/// attributes.
pub trait Computable: Send + Sync + 'static {
    /// A type of the syntax tree node.
    ///
    /// This type should match the [Grammar] type.
    type Node: Grammar;

    /// Returns a value of the [attribute](crate::analysis::Attr) inferred
    /// from the syntax tree nodes and other attributes.
    ///
    /// The `context` object of the [AttrContext] type provides access to the
    /// [Analyzer]'s documents, attributes, and other objects needed to infer
    /// the returning value.
    ///
    /// Interactions with the `context` object subscribes the underlying
    /// attribute to changes in the Analyzer's semantic graph. These
    /// subscriptions form relations between the attributes. The Analyzer uses
    /// these relations to decide when to recompute corresponding attributes
    /// of the semantics graph.
    ///
    /// The compute function is allowed to reconfigure these relations on each
    /// call, but the implementation should be deterministic such that
    /// **the returning value should not relay on the external environment**
    /// outside of the `context`.
    ///
    /// Note that the **implementation should not introduce recursive
    /// dependencies** between the attributes. In other words, the compute
    /// function should never read its own value directly or indirectly through
    /// other attributes.
    ///
    /// Also, note that the compute function normally should only use
    /// the `context`'s current node ([AttrContext::node_ref]) state when
    /// computing the returning value or the state of the nodes referred to by
    /// the semantic graph attributes because **the Analyzer unable
    /// to subscribe the attribute to the changes in the syntax tree**.
    ///
    /// The only exception from this rule is for [scope](Grammar::is_scope)
    /// nodes. The scoped attributes of the scoped node are allowed to inspect
    /// the syntax tree nodes within the scoped subtree because the Analyzer
    /// intentionally invalidates such attributes when detecting any change
    /// within the scoped subtree. In this sense the scoped attributes are the
    /// entry-point attributes performing initial mapping of the syntax tree
    /// to the semantic model of the programming language.
    ///
    /// See [Analyzer] specification for details.
    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self>
    where
        Self: Sized;
}

/// A function that computes [attribute](crate::analysis::Attr)'s value
/// wrapped in [Shared].
///
/// By implementing this trait on a custom type `T` of the programming language
/// semantic model, the type `Shared<T>` becomes eligible as
/// an [Attr](crate::analysis::Attr)'s parameter.
///
/// The compute_shared function infers a particular fact (or a set of facts) of
/// the semantic model from the syntax tree and other attributes, and wraps it
/// into Shared.
///
/// The SharedComputable is of particular interest for the attributes that
/// map shared parts of the semantic model, computed by other attributes. This
/// kind of spreading of the shared parts makes the semantic graph more
/// granular, which improves the incremental computation performance of
/// the programming language semantics.
pub trait SharedComputable: Send + Sync + 'static {
    /// A type of the syntax tree node.
    ///
    /// This type should match the [Grammar] type.
    type Node: Grammar;

    /// Returns a value of the [attribute](crate::analysis::Attr) inferred
    /// from the syntax tree nodes and other attributes, wrapped into [Shared].
    ///
    /// See [Computable::compute] function for details.
    ///
    /// Also, see [Analyzer] for the full specification of the semantics model.
    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>>
    where
        Self: Sized;
}

impl<D: SharedComputable> Computable for Shared<D> {
    type Node = D::Node;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        Ok(D::compute_shared(context)?)
    }
}

/// A context of the [Computable] and the [SharedComputable] functions.
///
/// When the [compute](Computable::compute) or
/// the [compute_shared](SharedComputable::compute_shared) functions are called,
/// a mutable reference to this object is passed to these functions.
///
/// The user's code uses this object to read Analyzer's documents (and their
/// syntax trees), to read other attribute values, to read the classes of
/// the syntax tree nodes, and to subscribe to the Analyzer-wide events.
///
/// Essentially, the AttrContext provides access to the Analyzer's content
/// through which the computable function reads input for the computation. Also,
/// the AttrContext object tracks the objects that the computable function
/// interacts with and subscribes to changes in these objects, such that
/// the changes trigger future recomputations of
/// the [attribute](crate::analysis::Attr) values.
///
/// The Analyzer is capable to subscribe the attribute on these three types
/// of objects:
///
///  - Other attributes of the Analyzer's semantic graph. (subscribed by
///    calling the [Attr::read](crate::analysis::Attr) or [AttrRef::read]
///    functions).
///  - Changes in the classes. (subscribed by calling
///    the [AttrContext::read_class] function)
///  - Analyzer-wide event triggers. (subscribed by calling
///    the [AttrContext::subscribe] and related functions)
///
/// Any other types of the possible inputs used inside the computable function
/// are considered a side effect of the function. In particular,
/// **inspecting the syntax tree is a side effect**. Hence, the computable
/// function by default is only allowed to relay on
/// the [current node](AttrContext::node_ref)'s inner state. The exception to
/// this rule is for the [scope](Grammar::is_scope) node
/// scoped attributes that are allowed to read the scope subtrees structure to
/// map the syntax tree to the semantic model.
///
/// See [Analyzer] for the full specification of the semantics model.
pub struct AttrContext<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> {
    analyzer: &'a Analyzer<N, H, S>,
    revision: Revision,
    handle: &'a H,
    node_ref: &'a NodeRef,
    deps: CacheDeps<N, S>,
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> AttrContext<'a, N, H, S> {
    #[inline(always)]
    pub(super) fn new(analyzer: &'a Analyzer<N, H, S>, revision: Revision, handle: &'a H) -> Self {
        Self {
            analyzer,
            revision,
            handle,
            node_ref: &NIL_NODE_REF,
            deps: CacheDeps::default(),
        }
    }

    /// Returns a [NodeRef] reference to the node of the syntax tree that
    /// owns an [attribute](crate::analysis::Attr) being computed.
    ///
    /// In particular, you can reveal
    /// the [identifier](crate::arena::Identifiable::id) of the attribute's
    /// document through this object.
    #[inline(always)]
    pub fn node_ref(&self) -> &'a NodeRef {
        self.node_ref
    }

    /// Returns true if the Analyzer contains a document with the specified `id`.
    ///
    /// The underlying attribute will be recomputed if the returning value
    /// changes from true to false.
    #[inline(always)]
    pub fn contains_doc(&mut self, id: Id) -> bool {
        let result = self.analyzer.docs.contains_key(&id);

        if result && id != self.node_ref.id {
            self.subscribe(id, DOC_REMOVED_EVENT);
        }

        result
    }

    /// Returns a RAII guard that provides read-only access to the analyzer's
    /// document with the specified `id`.
    ///
    /// Returns a [MissingDocument](AnalysisError::MissingDocument) error
    /// if there is no document with the specified `id`.
    ///
    /// The underlying attribute will be recomputed if the document
    /// removed from the Analyzer.
    #[inline(always)]
    pub fn read_doc(&mut self, id: Id) -> AnalysisResult<DocumentReadGuard<'a, N, S>> {
        let Some(guard) = self.analyzer.docs.get(&id) else {
            return Err(AnalysisError::MissingDocument);
        };

        if id != self.node_ref.id {
            self.subscribe(id, DOC_REMOVED_EVENT);
        }

        Ok(DocumentReadGuard::from(guard))
    }

    /// Returns a snapshot of the set of node references that belong
    /// to the specified `class` in the document with the `id` identifier.
    ///
    /// Returns a [MissingDocument](AnalysisError::MissingDocument) error
    /// if the document with the specified `id` does not exist in the Analyzer.
    ///
    /// The underlying attribute will be recomputed if the returning Ok value
    /// changes.
    ///
    /// See [Classifier] for details.
    #[inline(always)]
    pub fn read_class(
        &mut self,
        id: Id,
        class: &<N::Classifier as Classifier>::Class,
    ) -> AnalysisResult<Shared<HashSet<NodeRef, S>>> {
        let _ = self.deps.classes.insert((id, class.clone()));

        let Some(guard) = self.analyzer.docs.get(&id) else {
            return Err(AnalysisError::MissingDocument);
        };

        let Some(class_to_nodes) = guard.classes_to_nodes.get(class) else {
            self.subscribe(id, DOC_UPDATED_EVENT);
            return Ok(Shared::default());
        };

        if id != self.node_ref.id {
            self.subscribe(id, DOC_REMOVED_EVENT);
        }

        Ok(class_to_nodes.nodes.clone())
    }

    /// Subscribes the attribute to the specified `event`.
    ///
    /// If the `id` parameter is not [nil](Id::nil), the underlying attribute
    /// will be recomputed when the `event` is triggered for the document
    /// with this `id`; otherwise, the attribute will be recomputed on any
    /// triggering of the `event`,
    #[inline(always)]
    pub fn subscribe(&mut self, id: Id, event: Event) {
        let _ = self.deps.events.insert((id, event));
    }

    /// Returns Ok if the underlying task has not been
    /// [signaled](TaskHandle::is_triggered) for graceful shutdown yet;
    /// otherwise returns an [Interrupted](AnalysisError::Interrupted) error.
    ///
    /// If the function returns an interruption error, it is fine to return
    /// this error from the [compute](Computable::compute) function
    /// (or from the [compute_shared](SharedComputable::compute_shared)
    /// function).
    ///
    /// Usually, you don't need to call this function manually, because
    /// the Analyzer checks the interruption event in between of the attribute
    /// computation bounds. However, if the computable function performs a
    /// computation heavy procedure, it is worth calling this function manually
    /// from time to time.
    #[inline(always)]
    pub fn proceed(&self) -> AnalysisResult<()> {
        if self.handle.is_triggered() {
            return Err(AnalysisError::Interrupted);
        }

        Ok(())
    }

    #[inline(always)]
    pub(super) fn fork(&self, node_ref: &'a NodeRef) -> AttrContext<'a, N, H, S> {
        AttrContext {
            analyzer: self.analyzer,
            revision: self.revision,
            handle: self.handle,
            node_ref,
            deps: CacheDeps::default(),
        }
    }

    #[inline(always)]
    pub(super) fn track_attr(&mut self, dep: &AttrRef) {
        let _ = self.deps.attrs.insert(*dep);
    }

    #[inline(always)]
    pub(super) fn track_slot(&mut self, dep: &SlotRef) {
        let _ = self.deps.slots.insert(*dep);
    }

    #[inline(always)]
    pub(super) fn into_deps(self) -> Shared<CacheDeps<N, S>> {
        Shared::new(self.deps)
    }
}

/// A RAII guard that provides read-only access to
/// the [attribute](crate::analysis::Attr)'s value.
///
/// The underlying value can be accessed through the [Deref] implementation of
/// this object.
///
/// This object is created by the [Attr::read](crate::analysis::Attr::read) and
/// the [AttrRef::read] functions and can practically be obtain from
/// the attribute computation context only.
// Safety: Entries order reflects guards drop semantics.
#[allow(dead_code)]
pub struct AttrReadGuard<
    'a,
    C: Computable,
    H: TaskHandle = TriggerHandle,
    S: SyncBuildHasher = RandomState,
> {
    pub(super) data: &'a C,
    pub(super) cell_guard:
        TimeoutRwLockReadGuard<'a, AttrRecordData<<C as Computable>::Node, H, S>>,
    pub(super) records_guard: TableReadGuard<'a, Id, DocRecords<C::Node, H, S>, S>,
}

impl<'a, C: Computable + Debug, H: TaskHandle, S: SyncBuildHasher> Debug
    for AttrReadGuard<'a, C, H, S>
{
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.data, formatter)
    }
}

impl<'a, C: Computable + Display, H: TaskHandle, S: SyncBuildHasher> Display
    for AttrReadGuard<'a, C, H, S>
{
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.data, formatter)
    }
}

impl<'a, C: Computable, H: TaskHandle, S: SyncBuildHasher> Deref for AttrReadGuard<'a, C, H, S> {
    type Target = C;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, C: Computable, H: TaskHandle, S: SyncBuildHasher> AttrReadGuard<'a, C, H, S> {
    #[inline(always)]
    pub(super) fn attr_revision(&self) -> Revision {
        let Some(cache) = &self.cell_guard.cache else {
            unsafe { ld_unreachable!("AttrReadGuard without cache.") }
        };

        cache.updated_at
    }
}

impl AttrRef {
    // Safety: If `CHECK == false` then `C` properly describes underlying attribute's computable data.
    pub(super) unsafe fn fetch<
        'a,
        const CHECK: bool,
        C: Computable,
        H: TaskHandle,
        S: SyncBuildHasher,
    >(
        &self,
        context: &mut AttrContext<'a, C::Node, H, S>,
    ) -> AnalysisResult<AttrReadGuard<'a, C, H, S>> {
        loop {
            let Some(records_guard) = context.analyzer.db.records.get(&self.id) else {
                return Err(AnalysisError::MissingDocument);
            };

            let Some(record) = records_guard.attrs.get(&self.entry) else {
                return Err(AnalysisError::MissingAttribute);
            };

            let record_read_guard = record.read(&context.analyzer.db.timeout)?;

            if record_read_guard.verified_at >= context.revision {
                if let Some(cache) = &record_read_guard.cache {
                    let data = match CHECK {
                        true => cache.downcast::<C>()?,

                        // Safety: Upheld by the caller.
                        false => unsafe { cache.downcast_unchecked::<C>() },
                    };

                    context.track_attr(self);

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The reference will ve valid for as long as the parent guard is held.
                    let data = unsafe { transmute::<&C, &'a C>(data) };

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The guard will ve valid for as long as the parent guard is held.
                    let cell_guard = unsafe {
                        transmute::<
                            TimeoutRwLockReadGuard<AttrRecordData<<C as Computable>::Node, H, S>>,
                            TimeoutRwLockReadGuard<
                                'a,
                                AttrRecordData<<C as Computable>::Node, H, S>,
                            >,
                        >(record_read_guard)
                    };

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The reference will ve valid for as long as the Analyzer is held.
                    let records_guard = unsafe {
                        transmute::<
                            TableReadGuard<Id, DocRecords<<C as Computable>::Node, H, S>, S>,
                            TableReadGuard<'a, Id, DocRecords<<C as Computable>::Node, H, S>, S>,
                        >(records_guard)
                    };

                    return Ok(AttrReadGuard {
                        data,
                        cell_guard,
                        records_guard,
                    });
                }
            }

            drop(record_read_guard);
            drop(records_guard);

            self.validate(context)?;
        }
    }

    fn validate<N: Grammar, H: TaskHandle, S: SyncBuildHasher>(
        &self,
        context: &AttrContext<N, H, S>,
    ) -> AnalysisResult<()> {
        loop {
            let Some(records) = context.analyzer.db.records.get(&self.id) else {
                return Err(AnalysisError::MissingDocument);
            };

            let Some(record) = records.attrs.get(&self.entry) else {
                return Err(AnalysisError::MissingAttribute);
            };

            {
                let record_read_guard = record.read(&context.analyzer.db.timeout)?;

                if record_read_guard.verified_at >= context.revision {
                    return Ok(());
                }
            }

            let mut record_write_guard = record.write(&context.analyzer.db.timeout)?;

            let record_data = record_write_guard.deref_mut();

            let Some(cache) = &mut record_data.cache else {
                let mut forked = context.fork(&record_data.node_ref);
                let memo = record_data.function.invoke(&mut forked)?;
                let deps = forked.into_deps();

                record_data.cache = Some(AttrRecordCache {
                    dirty: false,
                    updated_at: context.revision,
                    memo,
                    deps,
                });

                record_data.verified_at = context.revision;

                return Ok(());
            };

            if record_data.verified_at >= context.revision {
                return Ok(());
            }

            if !cache.dirty && !cache.deps.as_ref().events.is_empty() {
                for (id, event) in &cache.deps.as_ref().events {
                    let Some(guard) = context.analyzer.events.get(id) else {
                        continue;
                    };

                    let Some(updated_at) = guard.get(event) else {
                        continue;
                    };

                    if *updated_at > record_data.verified_at {
                        cache.dirty = true;
                        break;
                    }
                }
            }

            if !cache.dirty && !cache.deps.as_ref().classes.is_empty() {
                for (id, class) in &cache.deps.as_ref().classes {
                    let Some(guard) = context.analyzer.docs.get(id) else {
                        continue;
                    };

                    let Some(class_to_nodes) = guard.classes_to_nodes.get(class) else {
                        continue;
                    };

                    if class_to_nodes.revision > record_data.verified_at {
                        cache.dirty = true;
                        break;
                    }
                }
            }

            if !cache.dirty && !cache.deps.as_ref().slots.is_empty() {
                for slot_ref in &cache.deps.as_ref().slots {
                    let Some(dep_records) = context.analyzer.db.records.get(&slot_ref.id) else {
                        cache.dirty = true;
                        break;
                    };

                    let Some(dep_record) = dep_records.slots.get(&slot_ref.entry) else {
                        cache.dirty = true;
                        break;
                    };

                    let dep_record_read_guard = dep_record.read(&context.analyzer.db.timeout)?;

                    if dep_record_read_guard.revision > record_data.verified_at {
                        cache.dirty = true;
                        break;
                    }
                }
            }

            if !cache.dirty && !cache.deps.as_ref().attrs.is_empty() {
                let mut deps_verified = true;

                for attr_ref in &cache.deps.as_ref().attrs {
                    let Some(dep_records) = context.analyzer.db.records.get(&attr_ref.id) else {
                        cache.dirty = true;
                        break;
                    };

                    let Some(dep_record) = dep_records.attrs.get(&attr_ref.entry) else {
                        cache.dirty = true;
                        break;
                    };

                    let dep_record_read_guard = dep_record.read(&context.analyzer.db.timeout)?;

                    let Some(dep_cache) = &dep_record_read_guard.cache else {
                        cache.dirty = true;
                        break;
                    };

                    if dep_cache.dirty {
                        cache.dirty = true;
                        break;
                    }

                    if dep_cache.updated_at > record_data.verified_at {
                        cache.dirty = true;
                        break;
                    }

                    deps_verified =
                        deps_verified && dep_record_read_guard.verified_at >= context.revision;
                }

                if !cache.dirty {
                    if deps_verified {
                        record_data.verified_at = context.revision;
                        return Ok(());
                    }

                    context.proceed()?;

                    let deps = cache.deps.clone();

                    drop(record_write_guard);

                    for attr_ref in &deps.as_ref().attrs {
                        attr_ref.validate(context)?;
                    }

                    continue;
                }
            }

            if !cache.dirty {
                record_data.verified_at = context.revision;
                return Ok(());
            }

            let mut forked = context.fork(&record_data.node_ref);
            let new_memo = record_data.function.invoke(&mut forked)?;
            let new_deps = forked.into_deps();

            // Safety: New and previous values produced by the same Cell function.
            let same = unsafe { cache.memo.attr_memo_eq(new_memo.as_ref()) };

            cache.dirty = false;
            cache.memo = new_memo;
            cache.deps = new_deps;

            if !same {
                cache.updated_at = context.revision;
            }

            record_data.verified_at = context.revision;

            return Ok(());
        }
    }
}

#[allow(dead_code)]
pub struct SlotReadGuard<
    'a,
    T: Default + Send + Sync + 'static,
    N: Grammar,
    H: TaskHandle = TriggerHandle,
    S: SyncBuildHasher = RandomState,
> {
    pub(super) revision: Revision,
    pub(super) data: &'a T,
    pub(super) cell_guard: TimeoutRwLockReadGuard<'a, SlotRecordData>,
    pub(super) records_guard: TableReadGuard<'a, Id, DocRecords<N, H, S>, S>,
}

impl<'a, T, N, H, S> Debug for SlotReadGuard<'a, T, N, H, S>
where
    T: Debug + Default + Send + Sync + 'static,
    N: Grammar,
    H: TaskHandle,
    S: SyncBuildHasher,
{
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.data, formatter)
    }
}

impl<'a, T, N, H, S> Display for SlotReadGuard<'a, T, N, H, S>
where
    T: Display + Default + Send + Sync + 'static,
    N: Grammar,
    H: TaskHandle,
    S: SyncBuildHasher,
{
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.data, formatter)
    }
}

impl<'a, T, N, H, S> Deref for SlotReadGuard<'a, T, N, H, S>
where
    T: Default + Send + Sync + 'static,
    N: Grammar,
    H: TaskHandle,
    S: SyncBuildHasher,
{
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl SlotRef {
    // Safety: If `CHECK == false` then `T` properly describes underlying slot's computable data.
    pub(super) unsafe fn fetch<
        'a,
        const CHECK: bool,
        T: Default + Send + Sync + 'static,
        N: Grammar,
        H: TaskHandle,
        S: SyncBuildHasher,
    >(
        &self,
        context: &mut AttrContext<'a, N, H, S>,
    ) -> AnalysisResult<SlotReadGuard<'a, T, N, H, S>> {
        let Some(records_guard) = context.analyzer.db.records.get(&self.id) else {
            return Err(AnalysisError::MissingDocument);
        };

        let Some(record) = records_guard.slots.get(&self.entry) else {
            return Err(AnalysisError::MissingSlot);
        };

        let record_read_guard = record.read(&context.analyzer.db.timeout)?;

        let data = match CHECK {
            true => record_read_guard.downcast::<T>()?,

            // Safety: Upheld by the caller.
            false => unsafe { record_read_guard.downcast_unchecked::<T>() },
        };

        context.track_slot(self);

        let revision = record_read_guard.revision;

        // Safety: Prolongs lifetime to Analyzer's lifetime.
        //         The reference will ve valid for as long as the parent guard is held.
        let data = unsafe { transmute::<&T, &'a T>(data) };

        // Safety: Prolongs lifetime to Analyzer's lifetime.
        //         The guard will ve valid for as long as the parent guard is held.
        let cell_guard = unsafe {
            transmute::<
                TimeoutRwLockReadGuard<SlotRecordData>,
                TimeoutRwLockReadGuard<'a, SlotRecordData>,
            >(record_read_guard)
        };

        // Safety: Prolongs lifetime to Analyzer's lifetime.
        //         The reference will ve valid for as long as the Analyzer is held.
        let records_guard = unsafe {
            transmute::<
                TableReadGuard<Id, DocRecords<N, H, S>, S>,
                TableReadGuard<'a, Id, DocRecords<N, H, S>, S>,
            >(records_guard)
        };

        Ok(SlotReadGuard {
            revision,
            data,
            cell_guard,
            records_guard,
        })
    }

    // Safety: If `CHECK == false` then `T` properly describes underlying slot's computable data.
    pub(super) unsafe fn change<
        'a,
        const CHECK: bool,
        T: Default + Send + Sync + 'static,
        N: Grammar,
        H: TaskHandle,
        S: SyncBuildHasher,
    >(
        &self,
        task: &mut impl MutationAccess<N, H, S>,
        map: impl FnOnce(&mut T) -> bool,
    ) -> AnalysisResult<()> {
        let Some(records_guard) = task.analyzer().db.records.get(&self.id) else {
            return Err(AnalysisError::MissingDocument);
        };

        let Some(record) = records_guard.slots.get(&self.entry) else {
            return Err(AnalysisError::MissingSlot);
        };

        let mut record_write_guard = record.write(&task.analyzer().db.timeout)?;

        let data = match CHECK {
            true => record_write_guard.downcast_mut::<T>()?,

            // Safety: Upheld by the caller.
            false => unsafe { record_write_guard.downcast_unchecked_mut::<T>() },
        };

        let mutated = map(data);

        if mutated {
            record_write_guard.revision = task.analyzer().db.commit_revision();
        }

        Ok(())
    }
}
