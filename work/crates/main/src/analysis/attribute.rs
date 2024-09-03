////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
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
        AttrReadGuard,
        Computable,
        Feature,
        Grammar,
        Initializer,
        Invalidator,
        MutationAccess,
        Revision,
        SemanticAccess,
        SlotRef,
        TaskHandle,
        NIL_SLOT_REF,
    },
    arena::{Entry, Id, Identifiable},
    sync::SyncBuildHasher,
    syntax::{Key, NodeRef},
};

/// An [AttrRef] reference that does not point to any [attribute](Attr).
///
/// The value of this static equals to the [AttrRef::nil] value.
pub static NIL_ATTR_REF: AttrRef = AttrRef::nil();

/// A node of the [Analyzer](crate::analysis::Analyzer)'s semantics graph.
///
/// The purpose of the attribute is to represent a subset of the user-defined
/// semantic model (via the value of the attribute) of the programming language
/// that is assume to belong to a particular node of the syntax tree.
///
/// For example, one attribute could represent a programming language type
/// of the variable introduced in the source code, while another attribute could
/// represent a set of references of a function parameter in the source code.
///
/// The type of the value, denoted by `T`, implements a [Computable] trait,
/// which is a function that computes the value from the syntax tree and other
/// attribute values.
///
/// Under the hood, the value and the attribute metadata are owned by
/// the Analyzer, but the Attr object acts as a formal owner of this data.
/// Whenever the Attr instance is dropped, this data is removed from
/// the semantics graph.
///
/// Attr objects, in turn, are owned by the instances of the syntax tree nodes
/// (through the system of [features](Feature)) of the documents managed by
/// the Analyzer. Therefore, attribute creation and deletion are inherently
/// managed by the Analyzer too.
///
/// The Attr object does not implement the [Clone] trait, and normally it should
/// not be moved from the node that owns the instance. However, since
/// the attribute's data is owned by the Analyzer, you can use the [AttrRef]
/// referential object that points to this data inside the Analyzer's semantics
/// graph.
///
/// The [AttrRef] can be obtained using the [AsRef] and the [Feature]
/// implementations of the Attr.
///
/// See also [Slot](crate::analysis::Slot), a specialized version of an
/// attribute that enables manual control over the attribute's value.
#[repr(transparent)]
pub struct Attr<C: Computable> {
    inner: AttrInner,
    _data: PhantomData<C>,
}

impl<C: Computable> Debug for Attr<C> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        let attr_ref = self.as_ref();

        match attr_ref.is_nil() {
            false => formatter.write_fmt(format_args!(
                "Attr(id: {:?}, entry: {:?})",
                attr_ref.id, attr_ref.entry,
            )),

            true => formatter.write_str("Attr(Nil)"),
        }
    }
}

impl<C: Computable> Identifiable for Attr<C> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.as_ref().id
    }
}

impl<T: Computable, U: Computable> PartialEq<Attr<U>> for Attr<T> {
    #[inline(always)]
    fn eq(&self, other: &Attr<U>) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl<C: Computable> Eq for Attr<C> {}

impl<T: Computable, U: Computable> PartialOrd<Attr<U>> for Attr<T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Attr<U>) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<C: Computable> Ord for Attr<C> {
    #[inline(always)]
    fn cmp(&self, other: &Attr<C>) -> Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<C: Computable> Hash for Attr<C> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<C: Computable> AsRef<AttrRef> for Attr<C> {
    #[inline(always)]
    fn as_ref(&self) -> &AttrRef {
        let AttrInner::Init { attr_ref, .. } = &self.inner else {
            return &NIL_ATTR_REF;
        };

        attr_ref
    }
}

impl<C: Computable> Drop for Attr<C> {
    fn drop(&mut self) {
        let AttrInner::Init { attr_ref, database } = &self.inner else {
            return;
        };

        let Some(database) = database.upgrade() else {
            return;
        };

        database.deregister_attribute(attr_ref.id, &attr_ref.entry);
    }
}

impl<C: Computable> AbstractFeature for Attr<C> {
    #[inline(always)]
    fn attr_ref(&self) -> &AttrRef {
        self.as_ref()
    }

    #[inline(always)]
    fn slot_ref(&self) -> &SlotRef {
        &NIL_SLOT_REF
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

impl<C: Computable + Eq> Feature for Attr<C> {
    type Node = C::Node;

    #[inline(always)]
    fn new(node_ref: NodeRef) -> Self {
        Self {
            inner: AttrInner::Uninit(node_ref),
            _data: PhantomData,
        }
    }

    fn init<H: TaskHandle, S: SyncBuildHasher>(
        &mut self,
        initializer: &mut Initializer<Self::Node, H, S>,
    ) {
        let AttrInner::Uninit(node_ref) = &self.inner else {
            return;
        };

        let id = node_ref.id;

        #[cfg(debug_assertions)]
        if initializer.id() != id {
            panic!("Attribute and Compilation Unit mismatch.");
        }

        let node_ref = *node_ref;

        let (database, entry) = initializer.register_attribute::<C>(node_ref);

        self.inner = AttrInner::Init {
            attr_ref: AttrRef { id, entry },
            database,
        };
    }

    fn invalidate<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        invalidator: &mut Invalidator<Self::Node, H, S>,
    ) {
        let AttrInner::Init { attr_ref, .. } = &self.inner else {
            return;
        };

        #[cfg(debug_assertions)]
        if invalidator.id() != attr_ref.id {
            panic!("Attribute and Compilation Unit mismatch.");
        }

        invalidator.invalidate_attribute(&attr_ref.entry);
    }
}

impl<C: Computable> Attr<C> {
    /// Requests a copy of the attribute's value.
    ///
    /// Returns a pair of two elements:
    ///  1. The [revision](Revision) under which the attribute's value has been
    ///     computed.
    ///  2. A copy of the attribute's value.
    ///
    /// This function is supposed to be called **outside** of
    /// the [computation context](Computable::compute).
    ///
    /// As a general rule, if the returning revision number equals the revision
    /// number of the previous call to the snapshot function, you can treat
    /// both copies of the attribute value as equal. Otherwise, the equality is
    /// not guaranteed.
    ///
    /// Depending on the current semantic graph state, the Analyzer could
    /// return the value from the cache or recompute a subset of
    /// the semantics graph required to validate the cache.
    ///
    /// The `task` parameter grants access to the Analyzer's semantics and
    /// could be either an [AnalysisTask](crate::analysis::AnalysisTask) or
    /// an [ExclusiveTask](crate::analysis::ExclusiveTask).
    ///
    /// If the specified task has been interrupted, the function returns
    /// an [Interrupted](AnalysisError::Interrupted) error.
    ///
    /// If the value takes too long to compute, or if the semantic graphs
    /// contains cycles (which is an issue in the graph design), the function
    /// returns a [Timeout](AnalysisError::Timeout) error.
    ///
    /// The function can also return any [AnalysisError] yielded by
    /// the underling [Computable::compute] functions involved in this
    /// attribute's value computation process.
    #[inline(always)]
    pub fn snapshot<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        task: &impl SemanticAccess<C::Node, H, S>,
    ) -> AnalysisResult<(Revision, C)>
    where
        C: Clone,
    {
        let mut reader = AttrContext::new(task.analyzer(), task.revision(), task.handle());

        let result = self.read(&mut reader)?;
        let revision = result.attr_revision();
        let data = result.deref().clone();

        Ok((revision, data))
    }

    /// Provides read-only access to the attribute's value.
    ///
    /// This function is supposed to be called **inside** of
    /// the computation context.
    ///
    /// The `context` parameter is the "context" argument of the current
    /// [computable function](Computable::compute).
    ///
    /// By calling this function, the computable attribute **subscribes** to
    /// changes in this attribute, establishing relations between semantics
    /// graph nodes.
    ///
    /// Note that you should ensure that the attribute currently being computed
    /// will never read its own value using the read function, neither directly
    /// nor indirectly through other attributes. Cyclic relations in
    /// the semantics graph are forbidden and will lead to errors, such as
    /// [Timeout](AnalysisError::Timeout) errors.
    ///
    /// If the specified task has been interrupted, the function returns
    /// an [Interrupted](AnalysisError::Interrupted) error, which should be
    /// further returned from the current computable function.
    ///
    /// Otherwise, if the function returns any other type of error, which is
    /// an [abnormal](AnalysisError::is_abnormal) error, it is recommended
    /// to panic in place to indicate the issue as early as possible.
    /// You are encouraged to use
    /// the [AnalysisResultEx](crate::analysis::AnalysisResultEx) helper trait
    /// for this purpose.
    #[inline(always)]
    pub fn read<'a, H: TaskHandle, S: SyncBuildHasher>(
        &self,
        context: &mut AttrContext<'a, C::Node, H, S>,
    ) -> AnalysisResult<AttrReadGuard<'a, C, H, S>> {
        let attr_ref = self.as_ref();

        if attr_ref.is_nil() {
            return Err(AnalysisError::UninitAttribute);
        }

        // Safety: Attributes data came from the C::compute function.
        unsafe { attr_ref.fetch::<false, C, H, S>(context) }
    }
}

/// A reference of the [attribute](Attr) in the Analyzer's semantics graph.
///
/// Essentially, AttrRef is a composite index within the Analyzer’s inner
/// database. Both components of this index form a unique pair within
/// the lifetime of the Analyzer.
///
/// If the attribute instance has been removed from the Analyzer's semantics
/// graph over time, new attributes within this Analyzer will never occupy
/// the same AttrRef object. However, the AttrRef referred to the removed
/// attribute would become _invalid_.
///
/// You can obtain a copy of the AttrRef using the [AsRef] and
/// the [Feature] implementations of the [Attr] object.
///
/// In general, it is recommended to read attribute values directly using
/// the [Attr::snapshot] and [Attr::read] functions to avoid extra checks.
/// However, you can use similar [AttrRef::snapshot] and [AttrRef::read]
/// functions that require specifying the type of the attribute's value
/// explicitly and involve extra checks of the type (even though these checks
/// are relatively cheap to perform).
///
/// The [nil](AttrRef::nil) AttrRefs are special references that are considered
/// to be always invalid. They intentionally don't refer any attribute within
/// any Analyzer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttrRef {
    /// An identifier of the document managed by the Analyzer to which
    /// the attribute belongs.
    ///
    /// If the attribute belongs to the
    /// [common semantics](Grammar::CommonSemantics), this value is [Id::nil].
    pub id: Id,

    /// A versioned index of the attribute instance within the Analyzer's inner
    /// database.
    pub entry: Entry,
}

impl Debug for AttrRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        match (self.id.is_nil(), self.entry.is_nil()) {
            (false, _) => formatter.write_fmt(format_args!(
                "Attr(id: {:?}, entry: {:?})",
                self.id, self.entry,
            )),

            (true, false) => formatter.write_fmt(format_args!("Attr(entry: {:?})", self.entry)),

            (true, true) => formatter.write_str("Attr(Nil)"),
        }
    }
}

impl Identifiable for AttrRef {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl Default for AttrRef {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl AttrRef {
    /// Returns an AttrRef that intentionally does not refer to any attribute
    /// within any Analyzer.
    ///
    /// If you need just a static reference to the nil AttrRef, use
    /// the predefined [NIL_ATTR_REF] static.
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            entry: Entry::nil(),
        }
    }

    /// Returns true, if the AttrRef intentionally does not refer to any
    /// attribute within any Analyzer.
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.id.is_nil() && self.entry.is_nil()
    }

    /// Requests a copy of the attribute's value.
    ///
    /// This function is similar to the [Attr::snapshot] function, but requires
    /// an additional generic parameter `C` that specifies the type of
    /// the [Attr]'s value.
    ///
    /// If the `C` parameter does not match the Attr's value type, the function
    /// returns a [TypeMismatch](AnalysisError::TypeMismatch) error.
    #[inline(always)]
    pub fn snapshot<C: Computable + Clone, H: TaskHandle, S: SyncBuildHasher>(
        &self,
        task: &impl SemanticAccess<C::Node, H, S>,
    ) -> AnalysisResult<(Revision, C)> {
        let mut reader = AttrContext::new(task.analyzer(), task.revision(), task.handle());

        let result = self.read::<C, H, S>(&mut reader)?;
        let revision = result.attr_revision();
        let data = result.deref().clone();

        Ok((revision, data))
    }

    /// Provides read-only access to the attribute's value.
    ///
    /// This function is similar to the [Attr::read] function, but requires
    /// an additional generic parameter `C` that specifies the type of
    /// the [Attr]'s value.
    ///
    /// If the `C` parameter does not match the Attr's value type, the function
    /// returns a [TypeMismatch](AnalysisError::TypeMismatch) error.
    #[inline(always)]
    pub fn read<'a, C: Computable, H: TaskHandle, S: SyncBuildHasher>(
        &self,
        reader: &mut AttrContext<'a, C::Node, H, S>,
    ) -> AnalysisResult<AttrReadGuard<'a, C, H, S>> {
        // Safety: `CHECK` set to true
        unsafe { self.fetch::<true, C, H, S>(reader) }
    }

    /// Explicitly turns the value of the attribute to invalid state.
    ///
    /// By calling this function, you are informing
    /// the [Analyzer](crate::analysis::Analyzer) that **the value**[^1] of
    /// the attribute referred to by the AttrRef should be considered as
    /// invalid. This action enforces recomputation of a part of the Analyzer's
    /// semantics that depend on this attribute (including the attribute
    /// itself).
    ///
    /// Usually, you don't need to call this function manually, as the Analyzer
    /// manages the semantic graph validation automatically. However,
    /// if the attribute's [computable function](Computable::compute) has a side
    /// effect (depends on the external environment), you can use this function
    /// to signalize the Analyzer that the environment external to the
    /// computable function has been updated.
    ///
    /// In general, it is recommended to avoid using of this function in
    /// favor of the Analyzer's built in mechanism of tracking the side effects
    /// such as [subscriptions](AttrContext::subscribe) to the Analyzer-wide
    /// events. Use the invalidate function only when no other options work
    /// for you.
    ///
    /// The `task` parameter grants access to mutate the state of the Analyzer.
    /// It could be either a [MutationTask](crate::analysis::MutationTask) or
    /// an [ExclusiveTask](crate::analysis::ExclusiveTask).
    ///
    /// If the AttrRef itself does not point to a [valid](AsRef::is_valid_ref)
    /// attribute, or if the value referred to by the AttrRef is not valid
    /// or has not been computed yet, calling this function has no effect.
    ///
    /// [^1]: Don't be confused with AttrRef [validity](AttrRef::is_valid_ref).
    /// The invalidate function turns **the value** of the attribute to
    /// an invalid state, but it does not affect the validity status of
    /// the AttrRef reference. The AttrRef will keep referring to the attribute
    /// regardless of its value validity.
    pub fn invalidate<N: Grammar, H: TaskHandle, S: SyncBuildHasher>(
        &self,
        task: &mut impl MutationAccess<N, H, S>,
    ) {
        let Some(records) = task.analyzer().db.records.get(&self.id) else {
            #[cfg(debug_assertions)]
            {
                panic!("Attribute does not belong to specified Analyzer.");
            }

            #[cfg(not(debug_assertions))]
            {
                return;
            }
        };

        let Some(record) = records.attrs.get(&self.entry) else {
            return;
        };

        record.invalidate();
        task.analyzer().db.commit_revision();
    }

    /// Returns true if the attribute referred to by this AttrRef exists in
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

        records.attrs.contains(&self.entry)
    }
}

enum AttrInner {
    Uninit(NodeRef),

    Init {
        attr_ref: AttrRef,
        database: Weak<dyn AbstractDatabase>,
    },
}
