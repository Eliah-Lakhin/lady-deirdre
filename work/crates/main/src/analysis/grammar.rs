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

extern crate lady_deirdre_derive;

use std::{
    collections::HashSet,
    hash::{Hash, RandomState},
    marker::PhantomData,
    ops::Deref,
    sync::Weak,
};

pub use lady_deirdre_derive::Feature;

use crate::{
    analysis::{
        database::{
            AbstractDatabase,
            AttrRecord,
            AttrRecordData,
            DocRecords,
            SlotRecord,
            SlotRecordData,
        },
        AnalysisError,
        AnalysisResult,
        AttrRef,
        Computable,
        ScopeAttr,
        SlotRef,
        TaskHandle,
        TriggerHandle,
        NIL_ATTR_REF,
        NIL_SLOT_REF,
    },
    arena::{Entry, Id, Identifiable, Repo},
    sync::SyncBuildHasher,
    syntax::{Key, Node, NodeRef},
    units::Document,
};

/// A full grammar of the programming language that includes lexis, syntax, and
/// semantics of the language.
///
/// The [Node](lady_deirdre_derive::Node) derive macro implements this trait,
/// which is a canonical way of implementing the Grammar.
///
/// The [Node] supertrait of this trait provides lexical and syntax components
/// of the programming language. The rest of the trait's API related to the
/// semantics description.
///
/// **NOTE**: This trait API is not stabilized yet. New trait members may be
/// added in future minor versions of Lady Deirdre.
pub trait Grammar: Node + AbstractFeature {
    /// A syntax tree node classifier that indexes the nodes by classes.
    ///
    /// When using the [Node](lady_deirdre_derive::Node) macro, this value is
    /// set to [VoidClassifier] by default, or could be overridden using
    /// the `#[classifier(...)]` attribute:
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// #[classifier(MyClassifier)]
    /// enum MyNode {
    ///     // ...
    /// }
    /// ```
    type Classifier: Classifier<Node = Self>;

    type CommonSemantics: Feature<Node = Self>;

    /// Initializes a new node semantics.
    ///
    /// This function should only be called once the node is created
    /// by the [Analyzer](crate::analysis::Analyzer)'s inner algorithm. This is
    /// usually happens during the syntax tree initial parsing and
    /// incremental reparsing when the parser creates a new node.
    ///
    /// This function initializes the node's semantics: all
    /// [attribute](crate::analysis::Attr) objects associated with this instance.
    ///
    /// When using the [Node](lady_deirdre_derive::Node) macro,
    /// it calls the [init](Feature::init) function on the field annotated with
    /// the `#[semantics]` attribute:
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     #[rule(...)]
    ///     SomeVariant {
    ///         #[semantics]
    ///         semantics: Semantics<SomeVariantSemantics>,
    ///     }
    /// }
    /// ```
    fn init<H: TaskHandle, S: SyncBuildHasher>(
        &mut self,
        initializer: &mut Initializer<Self, H, S>,
    );

    /// Invalidates a [scope](Self::is_scope) node semantics.
    ///
    /// This function should only be called by
    /// the [Analyzer](crate::analysis::Analyzer)'s inner algorithm on
    /// the [scope](Self::is_scope) nodes when the incremental reparser detects
    /// that any node within this scope has been affected (created, deleted, or
    /// updated) during the incremental reparsing.
    ///
    /// When using the [Node](lady_deirdre_derive::Node) macro,
    /// it calls the [invalidate](Feature::invalidate) function on the field
    /// annotated with the `#[semantics]` attribute:
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     #[rule(...)]
    ///     SomeVariant {
    ///         #[semantics]
    ///         semantics: Semantics<SomeVariantSemantics>,
    ///     }
    /// }
    /// ```
    fn invalidate<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        invalidator: &mut Invalidator<Self, H, S>,
    );

    /// Returns a special built-in [attribute](crate::analysis::Attr) that
    /// infers the [scope](Self::is_scope) of this node.
    ///
    /// If the node does not have semantics, and the scope attribute
    /// in particular, this function returns
    /// a [MissingSemantics](crate::analysis::AnalysisError::MissingSemantics)
    /// error which indicates an issue in the grammar configuration.
    ///
    /// Normally, every Grammar node should have a scope attribute even
    /// if the semantics grammar does not have explicit scopes (in this case
    /// the root node considered to be a scope of all nodes). This is
    /// the prerequisite of the [Analyzer](crate::analysis::Analyzer)'s inner
    /// semantic analysis algorithm.
    ///
    /// In particular, the [Node](lady_deirdre_derive::Node) macro ensures that
    /// either each Node variant has a [semantics](Semantics) field (which
    /// contains the scope attribute under the hood) or none of them have this
    /// field.
    fn scope_attr(&self) -> AnalysisResult<&ScopeAttr<Self>>;

    /// Returns true if this node denotes the scoped branch of the syntax
    /// tree.
    ///
    /// Scope nodes are the entry points (the "inputs") of the semantic model
    /// of the grammar.
    ///
    /// All descendant nodes of the branch up to the descendant scope nodes
    /// considered to be scoped by this scope node.
    ///
    /// When using the [Node](lady_deirdre_derive::Node) macro,
    /// this function returns true for the nodes annotated with the `#[scope]`
    /// attribute:
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     #[rule(...)]
    ///     #[scope] // This variant denotes a scope (is_scope() returns true).
    ///     ScopeVariant {
    ///         // ...
    ///     }
    ///
    ///     #[rule(...)]
    ///     // This variant variant is not a scope, but is a scoped node within
    ///     // some of its ancestors (is_scope() returns false).
    ///     NonScopeVariant {
    ///         // ...
    ///     }
    /// }
    /// ```
    fn is_scope(&self) -> bool;
}

/// A classifier of the syntax tree nodes.
///
/// When inferring the [document](Document)-wide semantic features, it could be
/// useful to get the set of nodes that syntactically belong to particular
/// predefined class.
///
/// For example, fetching all variable names within the document with
/// particular name strings, or fetching all function nodes.
///
/// The Classifier associated with the [Grammar](Grammar) implements
/// a [classify](Self::classify) function that returns a set of all classes
/// of the node to which this node belongs.
///
/// The [Analyzer](crate::analysis::Analyzer), in turn, calls this function on
/// each created or updated node during the initial parsing and incremental
/// reparsing to maintain the index of the node classes.
///
/// You can fetch a set of all nodes belong to specific class using
/// the [snapshot_class](crate::analysis::AbstractTask::snapshot_class) function
/// of the task to fetch a set of all nodes that belong to the specified class.
///
/// Or you can call the [read_class](crate::analysis::AttrContext::read_class)
/// function in the attribute's [compute](Computable::compute) implementation
/// that also returns a set of the nodes belonging to the class, but which is
/// also subscribes this attribute to changes in the class structure.
///
/// If you don't need a node classification feature of the analyzer, you can
/// use the [VoidClassifier] which is a noop.
pub trait Classifier {
    /// A type of the syntax tree node this classifier intends to classify.
    ///
    /// This type should match the [Grammar] type.
    type Node: Node;

    /// A type of the classes into which the classifier can partition
    /// syntax tree nodes.
    type Class: Clone + Eq + Hash + Send + Sync;

    /// Returns a set of classes to which the specified node belongs.
    ///
    /// The `node_ref` parameter points to the syntax tree node inside
    /// the `doc` that needs to be classified.
    ///
    /// The function returns an empty set if the node does not belong
    /// to any class or if the node referred to by the `node_ref` does not
    /// exist in this document.
    ///
    /// The `classify` function should not perform a deep inspection of
    /// the syntax tree structure to make a decision about the node's classes.
    /// Normally, this function should make a direct decision based on the inner
    /// structure of the node only without inspecting of it's ancestor or
    /// descendant nodes. However, the function can also rely on the lexical
    /// parts of the document that belong to the node's inner structure.
    fn classify<S: SyncBuildHasher>(
        doc: &Document<Self::Node>,
        node_ref: &NodeRef,
    ) -> HashSet<Self::Class, S>;
}

/// A node [Classifier] which is a noop.
pub struct VoidClassifier<N: Node>(PhantomData<N>);

impl<N: Node> Classifier for VoidClassifier<N> {
    type Node = N;
    type Class = ();

    #[inline(always)]
    fn classify<S: SyncBuildHasher>(
        _doc: &Document<Self::Node>,
        _node_ref: &NodeRef,
    ) -> HashSet<Self::Class, S> {
        HashSet::default()
    }
}

/// A composition of the semantics objects.
///
/// The semantics of a particular syntax tree node is a composition of
/// the semantics objects.
///
/// The root of the composition is a [Semantics] object owned by the syntax tree
/// node. This object is an entry point to the syntax tree node's semantics.
///
/// The [Attr](crate::analysis::Attr) object (attribute) is a final building
/// block of the semantics composition that infers a particular fact about
/// the compilation project semantics model related to the specified syntax
/// tree node.
///
/// Any other composition part is any arbitrary user-defined type that owns
/// attributes and other composition objects. Usually, these are struct
/// types with attributes and other similar struct types.
///
/// These three kinds of composition objects implement the Feature trait, which
/// provides abstract access to their content and functions to control
/// their lifetime.
///
/// You are encouraged to use the associated
/// [Feature](lady_deirdre_derive::Feature) derive macro to
/// implement the Feature trait on arbitrary struct types to define the
/// structure of semantics of a particular syntax tree node type.
///
/// An API of the trait is divided into two traits: the Feature trait itself,
/// which consists of the object-unsafe parts of the API, and
/// the [AbstractFeature] trait, which is an object safe trait that provides
/// functions to reveal the structure of the feature.
///
/// ## Feature Lifetime
///
/// The lifetime of the Feature consists of three stages:
///
///  - The [Feature::new] function constructs the instance of the feature in
///    an "uninitialized" state. In this state the feature object may not yet be
///    fully initialized (in particular, a part of its memory may not yet be
///    allocated). This function is assumed to be cheap to call.
///
///  - The [Feature::init] function finishes the initialization process by
///    constructing the remaining parts and initializing all nested
///    [attributes](crate::analysis::Attr). This function may never be called.
///    For example, the reparser may crate and then drop the syntax tree node
///    with a constructed but uninitialized [Semantics] field during the
///    incremental reparsing. However, the Analyzer ensures to initialize all
///    node semantics presented in the final [Document].
///
///  - When the feature is initialized,
///    the [Analyzer](crate::analysis::Analyzer)'s inner algorithm may
///    [invalidate](Feature::invalidate) the feature from time to time.
///    The Analyzer invalidates the scope node [Semantics] (through
///    the [Grammar::invalidate] function) to indicate that the scoped content
///    has been changed. In this case, it is up to the feature tree
///    implementation to decide which feature inner [Attr](crate::analysis::Attr)
///    objects require invalidation. In particular,
///    the [Feature](lady_deirdre_derive::Feature) derive macro propagates
///    the invalidation event to the struct fields annotated with the `#[scoped]`
///    macro attribute.
///
/// Note that the feature creation, initialization, invalidation, and
/// destruction are controlled by the Analyzer. The end-user code usually
/// doesn't need to call related functions manually.
pub trait Feature: AbstractFeature {
    /// A type of the [Grammar] to which this semantic feature belongs.
    type Node: Grammar;

    /// Creates a new Feature in uninitialized state.
    ///
    /// The `node_ref` parameter points to the syntax tree node to
    /// which this feature belongs.
    ///
    /// This function should only be called once the node is created
    /// by the [Analyzer](crate::analysis::Analyzer)'s inner algorithm. This is
    /// usually happens during the syntax tree initial parsing and
    /// incremental reparsing when the parser creates a new node.
    fn new(node_ref: NodeRef) -> Self
    where
        Self: Sized;

    /// Initializes uninitialized feature.
    ///
    /// This function should only be called once the node is created
    /// by the [Analyzer](crate::analysis::Analyzer)'s inner algorithm. This is
    /// usually happens during the syntax tree initial parsing and
    /// incremental reparsing when the parser creates a new node.
    fn init<H: TaskHandle, S: SyncBuildHasher>(
        &mut self,
        initializer: &mut Initializer<Self::Node, H, S>,
    );

    /// Invalidates a feature that is a part of the [scope](Self::is_scope)
    /// node semantics.
    ///
    /// This function should only be called by
    /// the [Analyzer](crate::analysis::Analyzer)'s inner algorithm on
    /// the [scope](Self::is_scope) nodes when the incremental reparser detects
    /// that any node within this scope has been affected (created, deleted, or
    /// updated) during the incremental reparsing.
    fn invalidate<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        invalidator: &mut Invalidator<Self::Node, H, S>,
    );
}

/// An object-safe part of the [Feature] API.
pub trait AbstractFeature {
    /// Returns an [AttrRef] reference object of the attribute if this feature
    /// represents an [attribute](crate::analysis::Attr).
    ///
    /// Otherwise, the function returns [nil](AttrRef::nil).
    ///
    /// This function is particularly useful to determine if the Feature is
    /// an attribute or a composite object.
    fn attr_ref(&self) -> &AttrRef;

    fn slot_ref(&self) -> &SlotRef;

    /// Returns a sub-feature of this feature by `key`.
    ///
    /// If there is no corresponding sub-feature the function returns
    /// a [MissingFeature](AnalysisError::MissingFeature) error.
    ///
    /// If the feature is not uninitialized, the function may return
    /// an [UninitSemantics](AnalysisError::UninitSemantics) error
    ///
    /// When using the [Feature](lady_deirdre_derive::Feature) macro, this
    /// function returns a reference to the struct field with the same access as
    /// the type access.
    ///
    /// ```ignore
    /// #[derive(Feature)]
    /// pub(super) struct MyFeature {
    ///     pub(super) foo: SubFeature1,
    ///     bar: SubFeature2,
    /// }
    ///
    /// my_feature(Key::Name("foo")) == Ok(<SubFeature1 reference>);
    ///
    /// // Because there is no "foo2" field.
    /// my_feature(Key::Name("foo2")) == Err(MissingFeature);
    ///
    /// // Because the "bar" field has an access different to the MyFeature
    /// // type access.
    /// my_feature(Key::Name("bar")) == Err(MissingFeature)
    /// ```
    ///
    /// When using with unnamed fields:
    ///
    /// ```ignore
    /// #[derive(Feature)]
    /// pub(super) struct MyFeature(pub(super) SubFeature1, SubFeature2);
    ///
    /// my_feature(Key::Index(0)) == Ok(<SubFeature1 reference>);
    ///
    /// // Because the second field has an access different to the MyFeature
    /// // type access.
    /// my_feature(Key::Index(1)) == Err(MissingFeature);
    ///
    /// // Because index 2 is outside of the feature index.
    /// my_feature(Key::Index(2)) == Err(MissingFeature);
    /// ```
    fn feature(&self, key: Key) -> AnalysisResult<&dyn AbstractFeature>;

    /// Returns all valid keys of the [feature](Self::feature) function.
    fn feature_keys(&self) -> &'static [&'static Key];
}

/// An initializer of the [Feature].
///
/// Via this object, the Analyzer recognizes
/// [attributes](crate::analysis::Attr) of the feature structure.
///
/// You cannot construct this object manually. The Analyzer creates
/// an Initializer during the syntax tree initialization stage of its inner
/// algorithm and calls corresponding nodes' [Grammar::init] function
/// with the mutable reference to the Initializer instance.
///
/// The init function propagates the reference to all [Features](Feature)
/// of the node semantics using the [Feature::init] function down to the node's
/// attributes. When the init function is called on
/// the [attribute](crate::analysis::Attr) object, the attribute registers
/// itself in the Analyzer's semantic graph via the Initializer instance.
pub struct Initializer<
    'a,
    N: Grammar,
    H: TaskHandle = TriggerHandle,
    S: SyncBuildHasher = RandomState,
> {
    pub(super) id: Id,
    pub(super) database: Weak<dyn AbstractDatabase>,
    pub(super) records: &'a mut DocRecords<N, H, S>,
    pub(super) inserts: bool,
}

/// A marker-[feature](Feature) of the syntax tree nodes with empty
/// [semantics](Semantics).
///
/// You are encouraged to use this object when the [Grammar](Grammar)'s node
/// variant does not have any semantics features:
///
/// ```ignore
/// #[derive(Node)]
/// enum MyNode {
///     #[rule(...)]
///     VariantWithoutSemantics {
///         #[semantics]
///         semantics: Semantics<VoidSemantics<MyNode>>,
///     }
/// }
/// ```
pub struct VoidFeature<N: Grammar>(PhantomData<N>);

impl<N: Grammar> Default for VoidFeature<N> {
    #[inline(always)]
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<N: Grammar> AbstractFeature for VoidFeature<N> {
    #[inline(always)]
    fn attr_ref(&self) -> &AttrRef {
        &NIL_ATTR_REF
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

impl<N: Grammar> Feature for VoidFeature<N> {
    type Node = N;

    #[inline(always)]
    fn new(_node_ref: NodeRef) -> Self {
        Self::default()
    }

    #[inline(always)]
    fn init<H: TaskHandle, S: SyncBuildHasher>(
        &mut self,
        _initializer: &mut Initializer<Self::Node, H, S>,
    ) {
    }

    #[inline(always)]
    fn invalidate<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        _invalidator: &mut Invalidator<Self::Node, H, S>,
    ) {
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> Identifiable for Initializer<'a, N, H, S> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> Initializer<'a, N, H, S> {
    #[inline(always)]
    pub(super) fn register_attribute<C: Computable<Node = N> + Eq>(
        &mut self,
        node_ref: NodeRef,
    ) -> (Weak<dyn AbstractDatabase>, Entry) {
        self.inserts = true;

        (
            self.database.clone(),
            self.records
                .attrs
                .insert(AttrRecord::new(AttrRecordData::new::<C>(node_ref))),
        )
    }

    #[inline(always)]
    pub(super) fn register_slot<T: Default + Send + Sync + 'static>(
        &mut self,
    ) -> (Weak<dyn AbstractDatabase>, Entry) {
        self.inserts = true;

        (
            self.database.clone(),
            self.records
                .slots
                .insert(SlotRecord::new(SlotRecordData::new::<T>())),
        )
    }
}

/// An invalidator of the [Feature].
///
/// Via this object, the Analyzer makes
/// [attributes](crate::analysis::Attr)' current values invalid in
/// the cache of the Analyzer's semantics graph.
///
/// You cannot construct this object manually. The Analyzer creates
/// an Invalidator during the syntax tree [scope](Grammar::is_scope)
/// invalidation stage of its inner algorithm and calls corresponding
/// nodes' [Grammar::invalidate] function with the mutable reference to
/// the Invalidator instance.
///
/// The invalidate function propagates the reference to the [Features](Feature)
/// of the node semantics using the [Feature::invalidate] function down to
/// the node's attributes. When the invalidate function is called on
/// the [attribute](crate::analysis::Attr) object, the attribute reports to
/// the Analyzer via the Initializer instance that the attribute's cached value
/// is subject to recomputation.
pub struct Invalidator<
    'a,
    N: Grammar,
    H: TaskHandle = TriggerHandle,
    S: SyncBuildHasher = RandomState,
> {
    pub(super) id: Id,
    pub(super) records: &'a mut Repo<AttrRecord<N, H, S>>,
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> Identifiable for Invalidator<'a, N, H, S> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> Invalidator<'a, N, H, S> {
    #[inline(always)]
    pub(super) fn invalidate_attribute(&mut self, entry: &Entry) {
        let Some(record) = self.records.get(entry) else {
            return;
        };

        record.invalidate();
    }
}

/// An entry-point object to the syntax tree node semantics.
///
/// This is a wrapper of the node's variant user-defined semantic [Feature]
/// denoted by the `F` parameter.
///
/// The `F` parameter could be any type that implements the Feature trait,
/// but in practice it is supposed to be either a user-defined struct type with
/// [Attr](crate::analysis::Attr) fields that build up node semantics, or
/// a [VoidFeature] if the node does not have specific semantics.
///
/// To define the `F` type, you are encouraged to use
/// the [Feature](lady_deirdre_derive::Feature) derive macro on the struct type:
///
/// ```ignore
/// #[derive(Node)]
/// enum MyNode {
///     #[rule(...)]
///     Variant {
///         #[semantics]
///         semantics: Semantics<MyNodeVariantSemantics>
///     }
/// }
///
/// #[derive(Feature)]
/// #[node(MyNode)]
/// struct MyNodeVariantSemantics {
///     // Just a normal attribute.
///     attr_1: Attr<SomeAttribute>,
///
///     // This attribute will be invalidated when the Analyzer's algorithm
///     // calls the Grammar::invalidate function, which propagates
///     // the invalidation event through the Semantics::invalidate call down
///     // to the MyNodeVariantSemantics feature fields annotated with
///     // the `#[scoped]` macro attribute.
///     #[scoped]
///     scoped_attr: Attr<ScopedAttribute>,
///
///     // The SubFeature should also implement the Feature trait.
///     // When annotated with the `#[scoped]` attribute, the invalidation event
///     // will be propagated to the `sub_feature`, as well as its fields.
///     sub_feature: SubFeature,
/// }
/// ```
///
/// The Semantics by itself implements a Feature trait, and as a semantic
/// feature, the object exists in one of two state:
///
///  - The [Feature::new] default constructor creates Semantics in
///    **uninitialized** state. In this state, Semantics does not allocate
///    memory for the underlying `F` Feature.
///
///  - When the Semantics object is **initialized** (through
///    the [Feature::init] function), it allocates heap memory with
///    the initialized `F` feature and the [ScopeAttr] instance.
///    In the initialized state, the [AbstractFeature] implementation of
///    Semantics delegates all trait calls to the `F` implementation.
///
/// Note that calls to the [AbstractFeature] functions of an uninitialized
/// Semantics object yields an [UninitSemantics](AnalysisError::UninitSemantics)
/// error.
///
/// Also, subsequent [initialization](Feature::init) of an already initialized
/// Semantics does nothing.
pub struct Semantics<F: Feature> {
    inner: Box<SemanticsInner<F>>,
}

impl<F: Feature> AbstractFeature for Semantics<F> {
    #[inline(always)]
    fn attr_ref(&self) -> &AttrRef {
        let Ok(inner) = self.get() else {
            return &NIL_ATTR_REF;
        };

        inner.attr_ref()
    }

    #[inline(always)]
    fn slot_ref(&self) -> &SlotRef {
        let Ok(inner) = self.get() else {
            return &NIL_SLOT_REF;
        };

        inner.slot_ref()
    }

    #[inline(always)]
    fn feature(&self, key: Key) -> AnalysisResult<&dyn AbstractFeature> {
        self.get()?.feature(key)
    }

    #[inline(always)]
    fn feature_keys(&self) -> &'static [&'static Key] {
        let Ok(inner) = self.get() else {
            return &[];
        };

        inner.feature_keys()
    }
}

impl<F: Feature> Feature for Semantics<F> {
    type Node = F::Node;

    fn new(node_ref: NodeRef) -> Self {
        Self {
            inner: Box::new(SemanticsInner::Uninit(node_ref)),
        }
    }

    fn init<H: TaskHandle, S: SyncBuildHasher>(
        &mut self,
        initializer: &mut Initializer<Self::Node, H, S>,
    ) {
        let SemanticsInner::Uninit(node_ref) = self.inner.deref() else {
            return;
        };

        let node_ref = *node_ref;

        let mut feature = F::new(node_ref);
        let mut scope_attr = ScopeAttr::new(node_ref);

        feature.init(initializer);
        scope_attr.init(initializer);

        *self.inner = SemanticsInner::Init {
            feature,
            scope_attr,
        };
    }

    fn invalidate<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        invalidator: &mut Invalidator<Self::Node, H, S>,
    ) {
        let SemanticsInner::Init { feature, .. } = self.inner.deref() else {
            return;
        };

        feature.invalidate(invalidator);
    }
}

impl<F: Feature> Semantics<F> {
    /// Provides access to the wrapped and initialized `F` [Feature].
    ///
    /// If Semantics is not initialized yet, this function returns
    /// an [UninitSemantics](AnalysisError::UninitSemantics) error.
    #[inline(always)]
    pub fn get(&self) -> AnalysisResult<&F> {
        let SemanticsInner::Init { feature, .. } = self.inner.deref() else {
            return Err(AnalysisError::UninitSemantics);
        };

        Ok(feature)
    }

    /// Provides access to the [ScopeAttr] of this Semantics.
    ///
    /// If Semantics is not initialized yet, this function returns
    /// an [UninitSemantics](AnalysisError::UninitSemantics) error.
    #[inline(always)]
    pub fn scope_attr(&self) -> AnalysisResult<&ScopeAttr<F::Node>> {
        let SemanticsInner::Init { scope_attr, .. } = self.inner.deref() else {
            return Err(AnalysisError::UninitSemantics);
        };

        Ok(scope_attr)
    }
}

enum SemanticsInner<F: Feature> {
    Uninit(NodeRef),

    Init {
        feature: F,
        scope_attr: ScopeAttr<F::Node>,
    },
}
