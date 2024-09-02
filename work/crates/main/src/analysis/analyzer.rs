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
    collections::HashMap,
    hash::RandomState,
    sync::{Arc, Weak},
    time::Duration,
};

use crate::{
    analysis::{
        database::{Database, DocRecords},
        entry::DocEntry,
        manager::{TaskKind, TaskManager},
        AnalysisResult,
        AnalysisTask,
        Event,
        ExclusiveTask,
        Feature,
        Grammar,
        Initializer,
        MutationTask,
        Revision,
        TaskHandle,
        TaskPriority,
        TriggerHandle,
    },
    arena::Id,
    sync::{SyncBuildHasher, Table},
    syntax::NodeRef,
};

/// An initial configuration of the [Analyzer].
///
/// This structure is non-exhaustive; new configuration options may be added
/// in future minor versions of this crate.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub struct AnalyzerConfig {
    /// When set to true, the Analyzer will be optimized to store just one
    /// single [Document](crate::units::Document).
    ///
    /// This option does not prevent the Analyzer from storing more than
    /// one document, but the cross-document operations will be
    /// less efficient.
    ///
    /// The default value is false.
    pub single_document: bool,

    /// Specifies the lower bound of the analysis operations (such as
    /// attributes [reading](crate::analysis::Attr::snapshot)) timeout.
    ///
    /// The upper bound of the timeout is not specified.
    /// In practice, the Analyzer gives more time for the analysis operation
    /// to complete, but it guarantees that the analysis operation will not fail
    /// with the [Timeout](crate::analysis::AnalysisError::Timeout) error for
    /// at least `analysis_timeout` amount of time.
    ///
    /// Note, however, that this value is ignored under the wasm targets.
    /// Under the wasm targets, the `analysis_timeout` value is treated as zero.
    ///
    /// The default value is 1 second under the development builds
    /// (when the `debug_assertions` feature is enabled), and 5 seconds in
    /// production builds.
    ///
    /// The development build timeout is intentionally shorter because
    /// the timeout event may indicate an existence of a cycle inside the
    /// semantic graph.
    pub analysis_timeout: Duration,
}

impl Default for AnalyzerConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyzerConfig {
    /// Returns new configuration object with all fields set to their defaults.
    #[inline(always)]
    pub const fn new() -> Self {
        let attributes_timeout;

        #[cfg(debug_assertions)]
        {
            attributes_timeout = 1000;
        }

        #[cfg(not(debug_assertions))]
        {
            attributes_timeout = 5000;
        }

        Self {
            single_document: false,
            analysis_timeout: Duration::from_millis(attributes_timeout),
        }
    }
}

/// An entry point of the compiler.
///
/// The Analyzer manages a set of [documents](crate::units::Document) that
/// represent "files" within the compilation project, and the semantic graph
/// that represents a state of the semantic model of the compiled project.
///
/// The Analyzer is responsible for parsing and incremental parsing of
/// the managed documents, and for synchronizing the syntax and lexis of
/// the documents with the semantic state. Documents' syntax and lexis parsing
/// happens instantly, whereas semantics synchronization happens on demand:
/// whenever you request a particular semantic feature, the subset of the
/// semantic graph required to infer this feature gets in sync with the recent
/// changes in the document's source code.
///
/// The Analyzer is a core component of the language server or a programming
/// language (incremental) compiler. It is worth keeping it in shared memory,
/// such as static, for convenient access to its API.
///
/// The state of the Analyzer consists of shared memory allocations. You gain
/// access to its state through the system of "tasks", objects that grant
/// access to particular kinds of operations.
///
/// The `N` generic parameter of the Analyzer denotes the type of
/// the programming language grammar of the managed documents, which includes
/// lexis, syntax, and semantics definitions.
///
/// The `H` parameter, which is a [TaskHandle], is the type of handle used
/// to gracefully shut down Analyzer's tasks earlier. This parameter set to
/// a [TriggerHandle] by default.
///
/// The `S` parameter specifies a hashing algorithm of the Analyzer's internal
/// hash maps and hash sets. The default type of `S` is the standard
/// [RandomState], but you are encouraged to replace it with a cheaper hashing
/// algorithm.
///
/// ## Semantic Model
///
/// The semantic model is a set of arbitrary user defined objects that abstracts
/// out the syntax level of the language to a more useful representation for
/// the final front-end analysis.
///
/// For example, in Rust the statement `let x = 10;` is a variable introduction
/// that could be expressed in terms of the syntax tree, but the inferred type
/// of the variable `x` (usize) would be a part of the semantic model state.
///
/// The semantic model objects form a system of dependencies. In the example
/// above the type of the variable `x` depends on the type of the variable
/// initialization expression `10`. The structure of dependencies could form
/// a complex system. For instance, the variable could be initialized with
/// an expression that calls a function introduced in a distinct file
/// (document), such that the type of the variable would depend on the return
/// type of that function, which, in turn, may depend on a number of other
/// semantic objects.
///
/// ## Semantic Graph
///
/// The semantic model state exposed through a system of
/// [attribute](crate::analysis::Attr) values that typically represent small
/// subsets of the semantic model, **localized to particular syntax tree nodes**.
///
/// Each attribute is parametrized with
/// a user-defined [Computable](crate::analysis::Computable) function that
/// aims to infer the value of the attribute based on the syntax trees and
/// other attributes.
///
/// The system of attributes forms a directed graph, called
/// **the Semantic Graph**, where the vertices of the graph are the attribute
/// values, the edges represent dependencies between attributes.
///
/// This graph is dynamic and is built on the fly. When the analyzer calls
/// a particular computable function of an attribute, and the function attempts
/// to read another attribute, the reading event establishes a dependency
/// between the attributes, which becomes an edge in the semantic graph.
///
/// Lady Deirdre establishes the following requirements for
/// the user-defined Semantic Model:
///
///  - The system of attributes **should not have recursive dependencies**.
///    An attribute cannot read its own value either directly or indirectly
///    through reading of other attributes' values. In other words,
///    the Semantic Graph must be
///    a [directed acyclic graph](https://en.wikipedia.org/wiki/Directed_acyclic_graph).
///
///  - **Computable functions must be pure functions without side effects**.
///    The output of the function must deterministically depend on its inputs.
///
///  - The attribute value types should implement [Eq] and [Clone] traits.
///    If the inputs of the attribute don't change, the output values of
///    the computable function calls must be equal.
///
/// The Analyzer's inner algorithm relies on the above assumptions when
/// rebuilding parts of the semantic graph at synchronization points between
/// the syntax tree and the semantic model.
///
/// ## Scopes
///
/// The [Grammar] of the programming language establishes a partition of
/// the syntax tree nodes into so-called _scoped subtrees_.
///
/// For example, in Java you can partition a module file into Java Class
/// introduction scopes, Class members scopes, and method body scopes.
///
/// The root node of the scoped subtree is called a [scope](Grammar::is_scope)
/// node.
///
/// Note that the inner scoped subtree inside the outer scoped subtree is
/// not a part of the outer scope nodes set. **The inner scope nodes inside the
/// scope subtree are the leaves of the scoped subtree**. In the example above,
/// the Java Class member node set is not a part of the Java Class introduction
/// scope.
///
/// The scope nodes are the synchronization entry points of the semantic graph,
/// or, in other words, the inputs of the semantic model.
///
/// From the _scoped attributes_ of the scope nodes, you can inspect
/// the structure of the syntax tree nodes of the scoped subtree to create
/// initial semantic model objects.
///
/// For example, the scoped attribute of the Java method body could create sets
/// of the variable namespaces and collect the statements reachability
/// metadata. Other non-scoped attributes could infer more specific semantic
/// facts about their nodes based on the metadata collected by the scoped nodes.
///
/// The scoped attributes are the only attributes allowed to inspect
/// the overall structure of the syntax tree within their scoped subtrees.
/// Non-scoped attributes should never perform structural inspection of
/// the syntax tree for the inner structure of the nodes they have
/// **direct access** to.
///
/// Direct access means the local node of the attribute or the nodes to which
/// the scoped attributes point in their values.
///
/// For example, during namespace analysis, the scoped attributes could make
/// a hash map between variable names (strings) and
/// the [NodeRefs](crate::syntax::NodeRef) of the variable initialization
/// expressions. The non-scoped node would be allowed to
/// [dereference](crate::syntax::NodeRef::deref) the NodeRefs from that map
/// to directly read the inner structure of the initialization expression nodes
/// (including their attributes), but it will not be allowed to inspect
/// the ancestors or descendants of the node via syntax tree traversal.
///
/// Any kind of attribute can **reveal its scope node** using the special
/// built-in [ScopeAttr](crate::analysis::ScopeAttr) available on the local
/// node of the attribute.
///
/// ## Synchronization
///
/// In contrast to the lexical and syntax parsing and incremental re-parsing
/// of the documents, which occur instantly, **semantic analysis
/// is demand-driven**.
///
/// The semantic graph is computed and recomputed whenever you
/// [read](crate::analysis::Attr::snapshot) particular attributes of the graph.
///
/// The Analyzer attempts to validate and recompute the least subset of
/// the graph to return the requested attribute's value, preferring to utilize
/// its inner cache whenever possible. The inner algorithm of the semantic graph
/// validation follows an approach similar to the one used in
/// [Salsa](https://github.com/salsa-rs/salsa). The correctness of the algorithm
/// depends on the requirements listed in the [Semantic Graph](#semantic-graph)
/// section.
///
/// The synchronization between the Syntax Tree and the Semantic Graph occurs
/// during document mutations when the end user
/// [writes](crate::analysis::MutationAccess::write_to_doc) text into
/// the document source code:
///
///  1. The Analyzer reparses the syntax tree of the document
///     and tracks the syntax tree nodes that has been affected during
///     the incremental reparsing.
///
///  2. Then, the Analyzer restores the relations between the affected nodes and
///     their ancestor scopes.
///
///  3. Finally, the Analyzer invalidates the scoped attributes (the inputs of
///     the semantic graph) of the scoped nodes in the Semantic Graph.
///
/// Since the document **mutation procedure only invalidates the scoped
/// attributes** of the semantic graph, non-scoped attributes should never
/// rely on changes in the syntax tree structure because they could miss
/// changes in the syntax tree structure during reparsing.
///
/// The mutation procedure is usually a fast process because
/// the [Document](crate::units::Document) object is specifically optimized
/// to perform fast incremental reparsing, and the Analyzer
/// **does not** recompute the semantic graph during mutations; it only
/// labels its scope attribute values inside the semantic graph cache as
/// invalid.
///
/// The corresponding subsets of the semantic graph will be computed or
/// recomputed only when you read corresponding attributes of these subsets.
///
/// In practice, a part of the semantic graph could coexist in an outdated and
/// untouched state most of the time if you don't observe its values.
/// For example, in a text editor, the end user usually observes only one screen
/// of the source code text at a time. Therefore, the language server backed by
/// Lady Deirdre would update only a part of the semantic model required
/// to render the current screen.
///
/// ## Attributes Invalidation
///
/// The value of the attribute becomes invalid and is subject to recomputation
/// in the following cases:
///
///  - The [computable](crate::analysis::Computable) function reads another
///    attribute, and the value of that attribute's has changed.
///
///  - The attribute is a scoped attribute of the scoped node, and the scoped
///    subtree of the syntax tree has changed during the incremental reparsing.
///
///  - The computable function
///    [subscribes](crate::analysis::AttrContext::subscribe) to
///    the Analyzer-wide [event](Event), and this event has been triggered.
///
///  - The computable function is
///    [reads a class of nodes](crate::analysis::AttrContext::read_class),
///    and the class has changed during the incremental reparsing.
///
///  - The [invalidate](crate::analysis::AttrRef::invalidate) function of
///    the [AttrRef](crate::analysis::AttrRef) object that points to this
///    attribute was called explicitly.
///
/// Otherwise, the Analyzer will consider that the attribute’s value
/// is up to date.
///
/// ## Syntax Tree Index
///
/// Nodes classification is the mechanism of indexing syntax tree nodes,
/// under which the Analyzer maintains a document-wide index of syntax tree nodes
/// of specific user-defined classes.
///
/// For example, you can define an index of identifiers with specified names
/// across the source code or an index of all Java classes across the document's
/// source code.
///
/// To enable node indexing, you should implement
/// the [Classifier](crate::analysis::Classifier) trait on the function-like
/// type that would return a set of classes of the requested node.
/// Then you should specify the classifier type in the [Grammar] definition:
///
/// ```ignore
/// #[derive(Node)]
/// #[classifier(MyClassifier)]
/// struct MyNode {
///     // ...
/// }
///
/// struct MyClassifier;
///
/// impl Classifier for MyClassifier {
///     type Node = MyNode;
///     type Class = MyClass;
///
///     fn classify<S: SyncBuildHasher>(
///         doc: &Document<Self::Node>,
///         node_ref: &NodeRef,
///     ) -> HashSet<Self::Class, S> {
///         // Returns a set of classes to which the requested node belongs.
///     }
/// }
///
/// #[derive(Clone, Eq, Hash)]
/// enum MyClass {
///     SomeClass,
///     AnotherClass(String),
/// }
/// ```
///
/// You can request the node set that belong to specified class both
/// [inside](crate::analysis::AttrContext::read_class)
/// and [outside](crate::analysis::AbstractTask::snapshot_class) of
/// the attribute's [computable](crate::analysis::Computable) function.
///
/// When accessing the node index inside the computable function,
/// the attribute will be dependent on the changes in the requested classes.
///
/// The Analyzer maintains node index automatically during document mutations.
/// Therefore, the node classification function should be relatively simple.
///
/// ## Events
///
/// The Analyzer provides a mechanism for mass invalidation of semantic graph
/// attributes through the system of [events](Event).
///
/// The attribute's [computable](crate::analysis::Computable) function can
/// subscribe to specific event, and when the event
/// is [triggered](crate::analysis::MutationAccess::trigger_event),
/// the corresponding subscriber values will become invalid in the semantic
/// graph (subject to recomputation).
///
/// There are a few built-in events, such as documents adding, removing, or
/// updating events, but most of the event numeric space (starting from
/// the [CUSTOM_EVENT_START_RANGE](crate::analysis::CUSTOM_EVENT_START_RANGE)
/// number) is left at your discretion.
///
/// The events mechanism is particularly useful to notify attributes that
/// rely on the external environment (attributes with side effects) about
/// changes in the external environment state. For instance, you can
/// subscribe the attributes of one Analyzer to changes in the state of Another
/// analyzer that manages a distinct subset of the compiled project.
///
/// ## Semantic Design Considerations
///
///  1. [Reading](crate::analysis::Attr::snapshot) of the values of attributes
///     requires cloning of the values (when the attribute is being read
///     outside of another attribute's [computable](crate::analysis::Computable)
///     function). Therefore, it is recommended to wrap the attribute value
///     types into cheap-to-clone containers such as [Arc] or
///     [Shared](crate::sync::Shared) whenever appropriate.
///
///  2. In particular, you can use
///     the [SharedComputable](crate::analysis::SharedComputable) helper trait,
///     which makes `Shared<T>` computable for an arbitrary computable type `T`.
///
///  3. The scoped attributes (the inputs of the semantic graph synchronization)
///     typically tend to have large volumes of metadata inferred from
///     the scoped subtrees. Performance-wise, it is recommended to spread
///     the metadata into small pieces using middleware attributes, such that
///     the end non-scoped attributes would depend on the middlewares rather
///     than the initial input attributes containing possibly excessive data for
///     their job.
///
/// ## Grammar Setup
///
/// To set up the programming language [Grammar] (denoted via the `N`
/// generic parameter of the Analyzer) you are encouraged to use
/// the [Node](lady_deirdre_derive::Node) derive macro on enum type that specify
/// the syntax grammar component of the language, and the entry points from
/// the node instances to the semantic graph attributes.
///
/// ```ignore
/// #[derive(Node)]
/// enum MyNode {
///     #[rule(...)]
///     #[scope]
///     SomeScopedNode {
///         #[semantics]
///         semantics: Semantics<SomeScopedNodeSemantics>,
///     },
///
///     #[rule(...)]
///     NonScopedNode {
///         #[semantics]
///         semantics: Semantics<NonScopedNodeSemantics>,
///     },
///
///     #[rule(...)]
///     ANodeWithoutSemantics {
///         #[semantics]
///         semantics: Semantics<VoidFeature<MyNode>>,
///     },
/// }
///
/// let analyzer: Analyzer<MyNode> = Analyzer::new(AnalyzerConfig::new());
/// ```
///
/// First, you need to annotate some of the parsable node variants with the
/// `#[scope]` macro attributes to denote these nodes
/// as [scoped](Grammar::is_scope) (as entry points of the semantic graph
/// synchronization with the syntax tree).
///
/// Then, in each node variant, you should define a `#[semantics]` field of type
/// [Semantics](crate::analysis::Semantics) parametrized with
/// the [Feature](crate::analysis::Feature) object that would consist of
/// the node attributes. If the node does not have any associated attributes,
/// you can use the [VoidFeature](crate::analysis::VoidFeature) zero-sized
/// type, but you have to define the `#[semantics]` in the parsable node
/// variant anyway.
///
/// To define the semantic feature object of a particular node variant you can
/// use the [Feature](lady_deirdre_derive::Feature) derive macro on the struct
/// type:
///
/// ```ignore
/// #[derive(Feature)]
/// #[node(MyNode)]
/// struct SomeScopedNodeSemantics {
///     #[scoped]
///     scoped_attribute: Attr<ComputableValue1>,
///
///     non_scoped_attribute: Attr<ComputableValue2>,
///
///     sub_feature: SomeSubFeature,
/// }
///
/// #[derive(Feature)]
/// #[node(MyNode)]
/// struct NonScopedNodeSemantics {
///     non_scoped_attribute: Attr<ComputableValue3>,
/// }
/// ```
///
/// Finally, you should implement the computable functions for the attribute
/// values of the semantic graph:
///
/// ```ignore
/// #[derive(Clone, PartialEq, Eq)]
/// struct ComputableValue1 {
///     foo: usize,
/// }
///
/// impl Computable for ComputableValue1 {
///     type Node = ScriptNode;
///
///     fn compute<H: TaskHandle, S: SyncBuildHasher>(
///         context: &mut AttrContext<Self::Node, H, S>,
///     ) -> AnalysisResult<Self> {
///         // A NodeRef reference to the local node of this attribute.
///         let node_ref = context.node_ref();
///
///         // An access to the syntax tree of the local node.
///         let doc_read = context.read_doc(node_ref.id).unwrap_abnormal()?;
///
///         // ...
///
///         Ok(Self { ... })
///     }
/// }
/// ```
///
/// ## Cycles Detection
///
/// Cyclic dependencies between semantic graph attributes are errors in
/// the semantic model design.
///
/// The Analyzer is unable to detect these issues statically. Cyclic references
/// may only appear during the execution of
/// [computable](crate::analysis::Computable) functions. When the Analyzer
/// encounters a self-referential configuration, it will eventually yield
/// a [Timeout](crate::analysis::AnalysisError::Timeout) error from
/// the corresponding attribute reading operation.
///
/// It is recommended to unwrap these errors with panic in place so that
/// the cyclic issues will be encountered as early as possible in the code where
/// they appear. In particular,
/// the [unwrap_abnormal](crate::analysis::AnalysisResultEx::unwrap_abnormal)
/// function unwraps Timeout issues in debug mode (when the `debug_assertions`
/// feature is enabled).
///
/// As a rule of thumb, it is also recommended to put debug or trace log
/// messages in each computable function implementation to enable tracing
/// of the attributes dependencies that lead to the cycle.
///
/// ## Analyzer's State Access
///
/// The Analyzer object is specifically designed to work in a multi-threaded
/// environment, allowing independent working threads to perform operations on
/// the Analyzer’s data without blocking.
///
/// This extends to semantic graph parallel computations, where if independent
/// threads request merely independent attribute values, the Analyzer can
/// perform computations on the semantic graph without blocking (or almost
/// without blocking).
///
/// However, semantic computations required that the syntax state of
/// the documents be locked for write during the computations. In other words,
/// the Analyzer does not allow requests to the semantic graph while another
/// thread is writing to the source code text of the managed documents.
///
/// In accordance with this limitation, the Analyzer introduces three kinds
/// of working modes: analysis mode, mutation mode, and exclusive mode.
///
/// In **analysis mode**, you can read semantic attribute values from
/// multiple threads, but you cannot mutate the documents.
///
/// In **mutation mode**, you can create, delete, and edit multiple
/// documents from the independent threads (without blocking as long as
/// the independent threads edit independent documents), and you can also
/// trigger Analyzer's-wide events.
///
/// In **exclusive mode**, you can sequentially perform any kind of
/// operations on the Analyzer's data, but from a single thread only. This mode
/// is especially useful for single-threaded compilers and
/// multi-threaded compilers when the single job thread needs to ensure
/// that a particular set of operations will be performed in a specified order
/// without interruptions.
///
/// Note, that under any of the Analyzer's mode, you can read the document's
/// lexical and syntax structures, and you can read the node classes too.
///
/// As an API user, you acquire access to the Analyzer's data in the specified
/// mode using the corresponding Analyzer's methods:
/// [analyze](Analyzer::analyze), [mutate](Analyzer::mutate),
/// and [exclusive](Analyzer::exclusive) (and their non-blocking variants with
/// `try_` prefix).
///
/// These functions return corresponding "task" objects that grant access to
/// the Analyzer's data.
///
/// You can think of the task as "RAII guards" to the analyzer's data,
/// but with complex acquiring and reclamation rules.
///
/// In particular, each task has a priority number and the associated
/// [task handle](TaskHandle) for graceful shutting down (or temporary
/// interruption) of the task's thread job.
///
/// Via the task handle the Analyzer and other job threads can signalize
/// the task's job thread to free the task object to give access to other types
/// of tasks.
///
/// The [task priority](TaskPriority) number is used by the Analyzer's internal
/// job queue to establish the task access acquisition priority (tasks with
/// higher priority compete over tasks with lower priority numbers).
///
/// Also, currently active tasks with lower priority acquiring task with higher
/// priority will be signaled for interruption if the acquiring task requires
/// switching the Analyzer to another mode.
///
/// The analyze, mutate, and exclusive functions can block the current thread
/// that calls them if the requested task access cannot be granted yet.
///
/// Their non-blocking alternatives (prefixed with `try_`) will yield
/// an [interruption](crate::analysis::AnalysisError::Interrupted) error
/// if the specified access cannot be granted instantly.
pub struct Analyzer<N: Grammar, H: TaskHandle = TriggerHandle, S: SyncBuildHasher = RandomState> {
    pub(super) docs: Table<Id, DocEntry<N, S>, S>,
    pub(super) common: N::CommonSemantics,
    pub(super) events: Table<Id, HashMap<Event, Revision>, S>,
    pub(super) db: Arc<Database<N, H, S>>,
    pub(super) tasks: TaskManager<H, S>,
}

impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> Default for Analyzer<N, H, S> {
    #[inline(always)]
    fn default() -> Self {
        Self::new(AnalyzerConfig::default())
    }
}

impl<N: Grammar, H: TaskHandle, S: SyncBuildHasher> Analyzer<N, H, S> {
    /// Creates a new Analyzer with the specified `config` options.
    ///
    /// Initially, the Analyzer does not hold any document.
    pub fn new(config: AnalyzerConfig) -> Self {
        let docs = match config.single_document {
            true => Table::with_capacity_and_hasher_and_shards(1, S::default(), 1),
            false => Table::new(),
        };

        let events = match config.single_document {
            true => Table::with_capacity_and_hasher_and_shards(1, S::default(), 1),
            false => Table::with_capacity_and_hasher_and_shards(1, S::default(), 1),
        };

        let db = Arc::new(Database::new(&config));

        let mut common = <N::CommonSemantics as Feature>::new(NodeRef::nil());

        {
            let mut records = DocRecords::new();

            let mut initializer: Initializer<'_, N, H, S> = Initializer {
                id: Id::nil(),
                database: Arc::downgrade(&db) as Weak<_>,
                records: &mut records,
                inserts: false,
            };

            common.init(&mut initializer);

            if initializer.inserts {
                db.records.insert(Id::nil(), records);
            }
        }

        let tasks = TaskManager::new();

        Self {
            docs,
            common,
            events,
            db,
            tasks,
        }
    }

    /// Requests access to the semantic analysis operations.
    ///
    /// The function returns a task object that will keep the Analyzer in
    /// the "analysis" mode until the last instance of the AnalysisTask is held.
    ///
    /// The `handle` parameter is an instance of the [TaskHandle] through which
    /// the task could be signaled for gracefully shutdown (interruption).
    ///
    /// The `priority` parameter specifies request priority over other tasks.
    ///
    /// The Analyzer's task manager tends to prioritize access granting to
    /// tasks with higher priority numbers and signals already activated
    /// non-analysis tasks to shut down if these tasks have a lower priority
    /// than the requested one.
    ///
    /// This function will block the current thread if the requested access
    /// could not be granted instantly (the Analyzer is in non-analysis mode,
    /// and there are unfinished non-analysis tasks).
    ///
    /// The function returns
    /// an [Interrupted](crate::analysis::AnalysisError::Interrupted) error if
    /// the Analyzer's task manager decides to cancel the task before
    /// access is granted.
    ///
    /// See the [Analyzer's State Access](#analyzers-state-access) section of
    /// the specification for details about the tasks management.
    pub fn analyze<'a>(
        &'a self,
        handle: &'a H,
        priority: TaskPriority,
    ) -> AnalysisResult<AnalysisTask<'a, N, H, S>> {
        let id = self
            .tasks
            .acquire_task(TaskKind::Analysis, handle, priority, true)?;

        Ok(AnalysisTask::new(id, self, handle))
    }

    /// Requests access to the semantic analysis operations **without blocking**.
    ///
    /// This is a non-blocking version of the [analyze](Self::analyze)
    /// function that will return
    /// an [Interrupted](crate::analysis::AnalysisError::Interrupted) error
    /// if the requested access could not be granted instantly.
    pub fn try_analyze<'a>(
        &'a self,
        handle: &'a H,
        priority: TaskPriority,
    ) -> AnalysisResult<AnalysisTask<'a, N, H, S>> {
        let id = self
            .tasks
            .acquire_task(TaskKind::Analysis, handle, priority, false)?;

        Ok(AnalysisTask::new(id, self, handle))
    }

    /// Requests access to the mutation operations (documents creation,
    /// deletion, write operations, and events triggering).
    ///
    /// The function returns a task object that will keep the Analyzer in
    /// the "mutation" mode until the last instance of the MutationTask is held.
    ///
    /// The `handle` parameter is an instance of the [TaskHandle] through which
    /// the task could be signaled for gracefully shutdown (interruption).
    ///
    /// The `priority` parameter specifies request priority over other tasks.
    ///
    /// The Analyzer's task manager tends to prioritize access granting to
    /// tasks with higher priority numbers and signals already activated
    /// non-mutation tasks to shut down if these tasks have a lower priority
    /// than the requested one.
    ///
    /// This function will block the current thread if the requested access
    /// could not be granted instantly (the Analyzer is in non-mutation mode,
    /// and there are unfinished non-mutation tasks).
    ///
    /// The function returns
    /// an [Interrupted](crate::analysis::AnalysisError::Interrupted) error if
    /// the Analyzer's task manager decides to cancel the task before
    /// access is granted.
    ///
    /// See the [Analyzer's State Access](#analyzers-state-access) section of
    /// the specification for details about the tasks management.
    pub fn mutate<'a>(
        &'a self,
        handle: &'a H,
        priority: TaskPriority,
    ) -> AnalysisResult<MutationTask<'a, N, H, S>> {
        let id = self
            .tasks
            .acquire_task(TaskKind::Mutation, handle, priority, true)?;

        Ok(MutationTask::new(id, self, handle))
    }

    /// Requests access to the mutation operations (documents creation,
    /// deletion, write operations, and events triggering) **without blocking**.
    ///
    /// This is a non-blocking version of the [mutate](Self::mutate)
    /// function that will return
    /// an [Interrupted](crate::analysis::AnalysisError::Interrupted) error
    /// if the requested access could not be granted instantly.
    pub fn try_mutate<'a>(
        &'a self,
        handle: &'a H,
        priority: TaskPriority,
    ) -> AnalysisResult<MutationTask<'a, N, H, S>> {
        let id = self
            .tasks
            .acquire_task(TaskKind::Mutation, handle, priority, false)?;

        Ok(MutationTask::new(id, self, handle))
    }

    /// Requests exclusive access to all kind of operations from a single thread.
    ///
    /// The function returns a task object that will keep the Analyzer in
    /// the "exclusive" mode until the instance of the ExclusiveTask is held.
    ///
    /// The `handle` parameter is an instance of the [TaskHandle] through which
    /// the task could be signaled for gracefully shutdown (interruption).
    ///
    /// The `priority` parameter specifies request priority over other tasks.
    ///
    /// The Analyzer's task manager tends to prioritize access granting to
    /// tasks with higher priority numbers and signals already activated
    /// tasks to shut down if these tasks have a lower priority than
    /// the requested one.
    ///
    /// This function will block the current thread if the requested access
    /// could not be granted instantly (there are other active tasks of any
    /// kind).
    ///
    /// The function returns
    /// an [Interrupted](crate::analysis::AnalysisError::Interrupted) error if
    /// the Analyzer's task manager decides to cancel the task before
    /// access is granted.
    ///
    /// See the [Analyzer's State Access](#analyzers-state-access) section of
    /// the specification for details about the tasks management.
    pub fn exclusive<'a>(
        &'a self,
        handle: &'a H,
        priority: TaskPriority,
    ) -> AnalysisResult<ExclusiveTask<'a, N, H, S>> {
        let id = self
            .tasks
            .acquire_task(TaskKind::Exclusive, handle, priority, true)?;

        Ok(ExclusiveTask::new(id, self, handle))
    }

    /// Requests exclusive access to all kind of operations from a single
    /// thread **without blocking**.
    ///
    /// This is a non-blocking version of the [exclusive](Self::exclusive)
    /// function that will return
    /// an [Interrupted](crate::analysis::AnalysisError::Interrupted) error
    /// if the requested access could not be granted instantly.
    pub fn try_exclusive<'a>(
        &'a self,
        handle: &'a H,
        priority: TaskPriority,
    ) -> AnalysisResult<ExclusiveTask<'a, N, H, S>> {
        let id = self
            .tasks
            .acquire_task(TaskKind::Exclusive, handle, priority, false)?;

        Ok(ExclusiveTask::new(id, self, handle))
    }

    /// Sets the minimal allowed value of tasks' priority.
    ///
    /// All current active tasks with priority less than the `threshold` will be
    /// signaled for graceful shutdown.
    ///
    /// All tasks in the waiting queue with priority less than the `threshold`
    /// will be cancelled, and their access request functions will return
    /// an [Interrupted](crate::analysis::AnalysisError::Interrupted) error.
    ///
    /// All future task requests with priority less than the `threshold` will
    /// be rejected, and their access request functions will keep returning
    /// an Interrupted error instantly.
    ///
    /// By default, the Analyzer has a zero cancellation threshold, meaning that
    /// it does not reject the tasks based solely on the task priority value.
    ///
    /// The cancellation threshold could be reset to a zero (or any other) value
    /// later using the same function.
    ///
    /// The set_access_level function is particularly useful to shut down
    /// the entire set of the compiler thread jobs by setting the threshold
    /// value to the [TaskPriority::MAX] value.
    #[inline(always)]
    pub fn set_access_level(&self, threshold: TaskPriority) {
        self.tasks.set_access_level(threshold);
    }

    /// Returns the current threshold of the tasks' priority.
    ///
    /// The function returns the latest value set by
    /// the [set_access_level](Self::set_access_level) function.
    ///
    /// The default value is zero.
    #[inline(always)]
    pub fn get_access_level(&self) -> TaskPriority {
        self.tasks.get_access_level()
    }
}
