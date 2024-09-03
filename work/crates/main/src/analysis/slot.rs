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
    cmp::Ordering,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
    sync::Weak,
};

use crate::{
    analysis::{
        database::AbstractDatabase,
        AbstractFeature,
        AbstractTask,
        AnalysisError,
        AnalysisResult,
        AttrContext,
        AttrRef,
        Feature,
        Grammar,
        Initializer,
        Invalidator,
        MutationAccess,
        Revision,
        SemanticAccess,
        SlotReadGuard,
        TaskHandle,
        NIL_ATTR_REF,
    },
    arena::{Entry, Id, Identifiable},
    sync::SyncBuildHasher,
    syntax::{Key, NodeRef},
};

/// An [SlotRef] reference that does not point to any [slot](Slot).
///
/// The value of this static equals to the [SlotRef::nil] value.
pub static NIL_SLOT_REF: SlotRef = SlotRef::nil();

/// A specialized version of an attribute that enables manual control over the
/// underlying value.
///
/// The purpose of a Slot is to provide a conventional mechanism for injecting
/// metadata from the environment, which is external to the Analyzer, into the
/// Analyzer's semantic model. Slots are particularly useful in the Analyzer's
/// [common semantics](Grammar::CommonSemantics) for storing common
/// configurations, such as the mapping between file names and their
/// corresponding document [IDs](Id) within the Analyzer.
///
/// The value of a Slot is fully integrated into the Analyzer's semantic graph
/// and semantic model, except that the content of the value is managed manually
/// by the API user.
///
/// Unlike a typical [attribute](crate::analysis::Attr), a Slot does not have an
/// associated function that computes its value. Instead, the Slot's value is
/// initialized using the [Default] implementation of type `T`, and the API user
/// manually modifies the value's content using the [Slot::mutate] function.
///
/// This interface is similar to the [Attr](crate::analysis::Attr) object, in
/// that attributes can [read](Slot::read) (and thus subscribe to changes in)
/// the Slot's content within the attribute's
/// [Computable](crate::analysis::Computable) implementation. The API user can
/// also obtain a [snapshot](Slot::snapshot) outside of the computation
/// procedure.
///
/// Note that, unlike Attr objects, Slot values will not be automatically
/// invalidated even if the Slot is part of a scoped node. However, they may be
/// recreated (and reset to their defaults) by the Analyzer when a document is
/// [edited](MutationAccess::write_to_doc). Therefore, Slots within syntax tree
/// nodes need to be maintained with extra care.
///
/// An associated [SlotRef] referential interface can be obtained using the
/// [AsRef] and the [Feature] implementations of the Slot.
#[repr(transparent)]
pub struct Slot<N: Grammar, T: Default + Send + Sync + 'static> {
    inner: SlotInner,
    _node: PhantomData<N>,
    _data: PhantomData<T>,
}

impl<N, T> Debug for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        let slot_ref = self.as_ref();

        match (slot_ref.id.is_nil(), slot_ref.entry.is_nil()) {
            (false, _) => formatter.write_fmt(format_args!(
                "Slot(id: {:?}, entry: {:?})",
                slot_ref.id, slot_ref.entry,
            )),

            (true, false) => formatter.write_fmt(format_args!("Slot(entry: {:?})", slot_ref.entry)),

            (true, true) => formatter.write_str("Slot(Nil)"),
        }
    }
}

impl<N: Grammar, T: Default + Send + Sync + 'static> Identifiable for Slot<N, T> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.as_ref().id
    }
}

impl<N, T, U> PartialEq<Slot<N, U>> for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
    U: Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn eq(&self, other: &Slot<N, U>) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl<N, T> Eq for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
}

impl<N, T, U> PartialOrd<Slot<N, U>> for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
    U: Default + Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn partial_cmp(&self, other: &Slot<N, U>) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<N, T> Ord for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn cmp(&self, other: &Slot<N, T>) -> Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<N, T> Hash for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<N, T> AsRef<SlotRef> for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn as_ref(&self) -> &SlotRef {
        let SlotInner::Init { attr_ref, .. } = &self.inner else {
            return &NIL_SLOT_REF;
        };

        attr_ref
    }
}

impl<N, T> Drop for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    fn drop(&mut self) {
        let SlotInner::Init { attr_ref, database } = &self.inner else {
            return;
        };

        let Some(database) = database.upgrade() else {
            return;
        };

        database.deregister_attribute(attr_ref.id, &attr_ref.entry);
    }
}

impl<N, T> AbstractFeature for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn attr_ref(&self) -> &AttrRef {
        &NIL_ATTR_REF
    }

    #[inline(always)]
    fn slot_ref(&self) -> &SlotRef {
        self.as_ref()
    }

    #[inline(always)]
    fn feature(&self, _key: Key) -> AnalysisResult<&dyn AbstractFeature> {
        Err(AnalysisError::MissingFeature)
    }

    #[inline(always)]
    fn feature_keys(&self) -> &'static [&'static Key] {
        &[]
    }
}

impl<N, T> Feature for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    type Node = N;

    #[inline(always)]
    fn new(node_ref: NodeRef) -> Self {
        Self {
            inner: SlotInner::Uninit(node_ref),
            _node: PhantomData,
            _data: PhantomData,
        }
    }

    fn init<H: TaskHandle, S: SyncBuildHasher>(
        &mut self,
        initializer: &mut Initializer<Self::Node, H, S>,
    ) {
        let SlotInner::Uninit(node_ref) = &self.inner else {
            return;
        };

        let id = node_ref.id;

        #[cfg(debug_assertions)]
        if initializer.id() != id {
            panic!("Slot and Compilation Unit mismatch.");
        }

        let (database, entry) = initializer.register_slot::<T>();

        self.inner = SlotInner::Init {
            attr_ref: SlotRef { id, entry },
            database,
        };
    }

    #[inline(always)]
    fn invalidate<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        _invalidator: &mut Invalidator<Self::Node, H, S>,
    ) {
    }
}

impl<N, T> Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    /// Requests a copy of the slot's value.
    ///
    /// Returns a pair of two elements:
    ///  1. The [revision](Revision) under which the slot's value was last
    ///     modified.
    ///  2. A copy of the slot's value.
    ///
    /// This function is supposed to be called **outside** of
    /// the [computation context](crate::analysis::Computable::compute).
    ///
    /// As a general rule, if the returning revision number equals the revision
    /// number of the previous call to the snapshot function, you can treat
    /// both copies of the attribute value as equal. Otherwise, the equality is
    /// not guaranteed.
    ///
    /// The `task` parameter grants access to the Analyzer's semantics and
    /// could be either an [AnalysisTask](crate::analysis::AnalysisTask) or
    /// an [ExclusiveTask](crate::analysis::ExclusiveTask).
    ///
    /// If the Analyzer unable to fetch the value within the current time
    /// limits, the function returns a [Timeout](AnalysisError::Timeout) error.
    #[inline(always)]
    pub fn snapshot<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        task: &impl SemanticAccess<N, H, S>,
    ) -> AnalysisResult<(Revision, T)>
    where
        T: Clone,
    {
        let mut reader = AttrContext::new(task.analyzer(), task.revision(), task.handle());

        let result = self.read::<H, S>(&mut reader)?;
        let revision = result.revision;
        let data = result.deref().clone();

        Ok((revision, data))
    }

    /// Mutates the content of the slot's value.
    ///
    /// This function is supposed be called **outside** of
    /// the [computation context](crate::analysis::Computable::compute).
    ///
    /// The `task` parameter grants write access to the Analyzer's semantic
    /// model and can be either a
    /// [MutationTask](crate::analysis::MutationTask) or an
    /// [ExclusiveTask](crate::analysis::ExclusiveTask).
    ///
    /// The `map` parameter is a callback that receives mutable references to
    /// the slot's value. This function can mutate the underlying content or
    /// leave it unchanged, and it must return a boolean flag indicating whether
    /// the content has been modified (`true` means that the value has been
    /// modified).
    ///
    /// Failure to adhere to the `map` function's flag requirement does not
    /// result in undefined behavior, but it could lead to inconsistencies in
    /// the semantic model.
    ///
    /// If the Analyzer is unable to acquire mutation access to the slot's value
    /// within the current time limits, the function returns a
    /// [Timeout](AnalysisError::Timeout) error.
    #[inline(always)]
    pub fn mutate<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        task: &impl MutationAccess<N, H, S>,
        map: impl FnOnce(&mut T) -> bool,
    ) -> AnalysisResult<()> {
        let slot_ref = self.as_ref();

        if slot_ref.is_nil() {
            return Err(AnalysisError::UninitSlot);
        }

        // Safety: Slot initialized with `T` default value.
        unsafe { slot_ref.change::<false, T, N, H, S>(task, map) }
    }

    /// Provides read-only access to the slot's value.
    ///
    /// This function is supposed be called **inside** of
    /// the computation context.
    ///
    /// The `context` parameter is the "context" argument of the current
    /// [computable function](Computable::compute).
    ///
    /// By calling this function, the computable attribute **subscribes** to
    /// changes in this slot, establishing a relationship between the attribute
    /// and the slot.
    ///
    /// If the Analyzer is unable to fetch the value within the current time
    /// limits, the function returns a [Timeout](AnalysisError::Timeout) error.
    #[inline(always)]
    pub fn read<'a, H: TaskHandle, S: SyncBuildHasher>(
        &self,
        reader: &mut AttrContext<'a, N, H, S>,
    ) -> AnalysisResult<SlotReadGuard<'a, T, N, H, S>> {
        let slot_ref = self.as_ref();

        if slot_ref.is_nil() {
            return Err(AnalysisError::UninitSlot);
        }

        // Safety: Slot initialized with `T` default value.
        unsafe { slot_ref.fetch::<true, T, N, H, S>(reader) }
    }
}

/// A reference of the [slot](Slot) in the Analyzer's semantics graph.
///
/// Essentially, SlotRef is a composite index within the Analyzer’s inner
/// database. Both components of this index form a unique pair within
/// the lifetime of the Analyzer.
///
/// If the slot instance has been removed from the Analyzer's semantics
/// graph over time, new slots within this Analyzer will never occupy
/// the same SlotRef object. However, the SlotRef referred to the removed
/// slot would become _invalid_.
///
/// You can obtain a copy of the SlotRef using the [AsRef] and
/// the [Feature] implementations of the [Slot] object.
///
/// In general, it is recommended to access slot values directly using
/// the [Slot::snapshot], [Slot::read], and [Slot::mutate] functions to avoid
/// extra checks. However, you can use similar SlotRef functions that require
/// specifying the type of the slot's value explicitly and involve extra checks
/// of the type (even though these checks are relatively cheap to perform).
///
/// The [nil](SlotRef::nil) SlotRefs are special references that are considered
/// to be always invalid. They intentionally don't refer any slot within
/// any Analyzer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotRef {
    /// An identifier of the document managed by the Analyzer to which
    /// the slot belongs.
    ///
    /// If the slot belongs to the [common semantics](Grammar::CommonSemantics),
    /// this value is [Id::nil].
    pub id: Id,

    /// A versioned index of the slot instance within the Analyzer's inner
    /// database.
    pub entry: Entry,
}

impl Debug for SlotRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        match (self.id.is_nil(), self.entry.is_nil()) {
            (false, _) => formatter.write_fmt(format_args!(
                "SlotRef(id: {:?}, entry: {:?})",
                self.id, self.entry,
            )),

            (true, false) => formatter.write_fmt(format_args!("SlotRef(entry: {:?})", self.entry)),

            (true, true) => formatter.write_str("SlotRef(Nil)"),
        }
    }
}

impl Identifiable for SlotRef {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl Default for SlotRef {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl SlotRef {
    /// Returns a SlotRef that intentionally does not refer to any slot
    /// within any Analyzer.
    ///
    /// If you need just a static reference to the nil SlotRef, use
    /// the predefined [NIL_SLOT_REF] static.
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            entry: Entry::nil(),
        }
    }

    /// Returns true, if the SlotRef intentionally does not refer to any
    /// slot within any Analyzer.
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.id.is_nil() && self.entry.is_nil()
    }

    /// Requests a copy of the slot's value.
    ///
    /// This function is similar to the [Slot::snapshot] function, but requires
    /// an additional generic parameter `T` that specifies the type of
    /// the [Slot]'s value.
    ///
    /// If the `T` parameter does not match the Slot's value type, the function
    /// returns a [TypeMismatch](AnalysisError::TypeMismatch) error.
    #[inline(always)]
    pub fn snapshot<T, N, H, S>(
        &self,
        task: &impl SemanticAccess<N, H, S>,
    ) -> AnalysisResult<(Revision, T)>
    where
        T: Clone + Default + Send + Sync + 'static,
        N: Grammar,
        H: TaskHandle,
        S: SyncBuildHasher,
    {
        let mut reader = AttrContext::new(task.analyzer(), task.revision(), task.handle());

        let result = self.read::<T, N, H, S>(&mut reader)?;
        let revision = result.revision;
        let data = result.deref().clone();

        Ok((revision, data))
    }

    /// Provides mutation access to the slot's value.
    ///
    /// This function is similar to the [Slot::mutate] function, but requires
    /// an additional generic parameter `T` that specifies the type of
    /// the [Slot]'s value.
    ///
    /// If the `T` parameter does not match the Slot's value type, the function
    /// returns a [TypeMismatch](AnalysisError::TypeMismatch) error.
    #[inline(always)]
    pub fn mutate<T, N, H, S>(
        &self,
        task: &impl MutationAccess<N, H, S>,
        map: impl FnOnce(&mut T) -> bool,
    ) -> AnalysisResult<()>
    where
        T: Default + Send + Sync + 'static,
        N: Grammar,
        H: TaskHandle,
        S: SyncBuildHasher,
    {
        // Safety: `CHECK` set to true
        unsafe { self.change::<true, T, N, H, S>(task, map) }
    }

    /// Provides read-only access to the slot's value.
    ///
    /// This function is similar to the [Slot::read] function, but requires
    /// an additional generic parameter `T` that specifies the type of
    /// the [Slot]'s value.
    ///
    /// If the `T` parameter does not match the Slot's value type, the function
    /// returns a [TypeMismatch](AnalysisError::TypeMismatch) error.
    #[inline(always)]
    pub fn read<'a, T, N, H, S>(
        &self,
        reader: &mut AttrContext<'a, N, H, S>,
    ) -> AnalysisResult<SlotReadGuard<'a, T, N, H, S>>
    where
        T: Default + Send + Sync + 'static,
        N: Grammar,
        H: TaskHandle,
        S: SyncBuildHasher,
    {
        // Safety: `CHECK` set to true
        unsafe { self.fetch::<true, T, N, H, S>(reader) }
    }

    /// Returns true if the slot referred to by this SlotRef exists in
    /// the Analyzer's database.
    ///
    /// The `task` parameter could be a task that grants any kind of access to
    /// the Analyzer:
    /// [AnalysisTask](crate::analysis::AnalysisTask),
    /// [MutationTask](crate::analysis::MutationTask),
    /// or [ExclusiveTask](crate::analysis::ExclusiveTask).
    #[inline(always)]
    pub fn is_valid_ref<N: Grammar, H: TaskHandle, S: SyncBuildHasher>(
        &self,
        task: &mut impl AbstractTask<N, H, S>,
    ) -> bool {
        let Some(records) = task.analyzer().db.records.get(&self.id) else {
            return false;
        };

        records.slots.contains(&self.entry)
    }
}

enum SlotInner {
    Uninit(NodeRef),

    Init {
        attr_ref: SlotRef,
        database: Weak<dyn AbstractDatabase>,
    },
}
