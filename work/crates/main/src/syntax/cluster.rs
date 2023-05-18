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
    arena::{Id, Identifiable, Ref, Repository},
    lexis::{SiteRef, SiteRefSpan},
    std::*,
    syntax::{ErrorRef, Node, NodeRef, SyntaxTree},
};

/// An ownership object of a part of the syntax structure data.
///
/// This a lower-level API that organizes syntax structure persistence. An API user usually does not
/// need to interact with Cluster directly or to inspect its fields. For a higher-level access use
/// [NodeRef](crate::syntax::NodeRef), [ErrorRef](crate::syntax::ErrorRef), or
/// [ClusterRef](crate::syntax::ClusterRef).
///
/// Syntax structure consists of a set of instance of [Node](crate::syntax::Node) objects and the
/// syntax/semantic error objects belong to these nodes. These objects could be split into groups
/// called Clusters. It is up to the syntax structure manager's design to decide on how to spread
/// these instances between clusters, and about the number of clusters per a single compilation
/// unit. In general, Cluster serves as a unit of caching of the syntax structure of the
/// compilation unit. It is assumed that if an incremental reparser obsoletes a single Node of the
/// syntax tree, it obsoletes the entire Cluster this Node belongs to altogether.
///
/// For example, since the [SyntaxBuffer](crate::syntax::SyntaxBuffer) does not provide any
/// incremental reparsing capabilities, it uses only a single Cluster to store all of the Nodes and
/// the syntax/semantic error objects of the syntax tree. Whereas the [Document](crate::Document)
/// object, being an incrementally managed compiler with reparsing operations, splits nodes and
/// syntax/semantic errors between many clusters more granularly.
///
/// If you are developing an incrementally compiled system, in general you should not relay on
/// particular granularity of the system of clusters. Your system should expect that any node or an
/// error object could obsolete at any time. The syntax structure manager does not have to provide
/// particular splitting guarantees.
///
/// Note that regardless of incremental capabilities of a compilation unit manager, the Cluster
/// object is a mutable object, as well as all of the mutable operations of related weak
/// references(such as [ClusterRef](crate::syntax::ClusterRef), [NodeRef](crate::syntax::NodeRef),
/// and the [ErrorRef](crate::syntax::ErrorRef)).
///
/// The object consists of two [Repositories](crate::arena::Repository) that store nodes and errors
/// in arbitrary order that considered to be "secondary" objects to this Cluster, and one single
/// "primary" Node instance.
pub struct Cluster<N: Node> {
    /// A single "selected" node of the cluster that considered to be a primary descriptive node of
    /// this cluster data.
    ///
    /// All other nodes of the Cluster considered to be helper nodes that together with the Primary
    /// one build up a part of the syntax tree.
    ///
    /// There are no particular rules on how to select this node, but it is assumed that the
    /// [`secondary nodes`](Cluster::nodes) are logically closely related to the Primary one.
    ///
    /// By convention this node is referred within the [`Ref::Primary`](crate::arena::Ref::Primary)
    /// low-level reference.
    pub primary: N,

    /// A set of the "secondary" nodes that together with the [primary](Cluster::primary) build up
    /// a part of the syntax tree.
    ///
    /// By convention this node is referred within the
    /// [`Ref::Repository`](crate::arena::Ref::Repository) low-level reference.
    pub nodes: Repository<N>,

    /// A set of syntax and semantic errors logically related to the nodes of this cluster.
    ///
    /// By convention this node is referred within the
    /// [`Ref::Repository`](crate::arena::Ref::Repository) low-level reference.
    pub errors: Repository<N::Error>,
}

impl<N: Node + Debug> Debug for Cluster<N> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("Cluster")
            .field("primary", &self.primary)
            .field("nodes", &self.nodes)
            .field("errors", &self.errors)
            .finish_non_exhaustive()
    }
}

/// A weak reference of the [Cluster] inside the syntax tree.
///
/// This objects represents a long-lived lifetime independent and type independent cheap to
/// [Copy](::std::marker::Copy) safe weak reference into the source code syntax structure.
///
/// ```rust
/// use lady_deirdre::{
///     Document,
///     syntax::{SimpleNode, SyntaxTree, NodeRef, Cluster, TreeContent}
/// };
///
/// let doc = Document::<SimpleNode>::from("[]{}()");
///
/// let braces_node_ref = &doc.root_node_ref().deref(&doc).unwrap().inner()[1];
/// let braces_cluster_ref = braces_node_ref.cluster();
/// let braces_cluster = braces_cluster_ref.deref(&doc).unwrap();
///
/// assert_eq!(&braces_cluster.primary, braces_node_ref.deref(&doc).unwrap());
/// ```
///
/// An API user normally does not need to inspect ClusterRef inner fields manually or to construct
/// a ClusterRef manually unless you are working on the Crate API Extension.
///
/// For details on the Weak references framework design see [Arena](crate::arena) module
/// documentation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ClusterRef {
    /// An [identifier](crate::arena::Id) of the [SyntaxTree](crate::syntax::SyntaxTree) instance
    /// this weakly referred Cluster belongs to.
    pub id: Id,

    /// An internal weak reference of the cluster into the [SyntaxTree](crate::syntax::SyntaxTree)
    /// instance.
    ///
    /// This low-level [Ref](crate::arena::Ref) object used by the ClusterRef under the hood to
    /// fetch particular values from the SyntaxTree dereferencing functions(e.g.
    /// [`SyntaxTree::get_cluster`](crate::syntax::SyntaxTree::get_cluster)).
    pub cluster_ref: Ref,
}

impl Debug for ClusterRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self.is_nil() {
            false => formatter.write_fmt(format_args!("ClusterRef({:?})", self.id())),
            true => formatter.write_str("ClusterRef(Nil)"),
        }
    }
}

impl Identifiable for ClusterRef {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl ClusterRef {
    /// Returns an invalid instance of the ClusterRef.
    ///
    /// This instance never resolves to valid [Cluster].
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            cluster_ref: Ref::Nil,
        }
    }

    /// Returns `true` if this instance will never resolve to a valid [Cluster].
    ///
    /// It is guaranteed that `ClusterRef::nil().is_nil()` is always `true`, but in general
    /// if this function returns `false` it is not guaranteed that provided instance is a valid
    /// reference.
    ///
    /// To determine reference validity per specified [SyntaxTree](crate::syntax::SyntaxTree)
    /// instance use [is_valid_ref](crate::syntax::ClusterRef::is_valid_ref) function instead.
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.id.is_nil() || self.cluster_ref.is_nil()
    }

    /// Immutably dereferences weakly referred [Cluster] of specified
    /// [SyntaxTree](crate::syntax::SyntaxTree).
    ///
    /// Returns [None] if this ClusterRef is not valid reference for specified `tree` instance.
    ///
    /// Use [is_valid_ref](crate::syntax::ClusterRef::is_valid_ref) to check ClusterRef validity.
    ///
    /// This function uses [`SyntaxTree::get_cluster`](crate::syntax::SyntaxTree::get_cluster)
    /// function under the hood.
    #[inline(always)]
    pub fn deref<'tree, N: Node>(
        &self,
        tree: &'tree impl SyntaxTree<Node = N>,
    ) -> Option<&'tree Cluster<N>> {
        if self.id != tree.id() {
            return None;
        }

        tree.get_cluster(&self.cluster_ref)
    }

    /// Mutably dereferences weakly referred [Cluster] of specified
    /// [SyntaxTree](crate::syntax::SyntaxTree).
    ///
    /// Returns [None] if this ClusterRef is not valid reference for specified `tree` instance.
    ///
    /// Use [is_valid_ref](crate::syntax::ClusterRef::is_valid_ref) to check ClusterRef validity.
    ///
    /// This function uses
    /// [`SyntaxTree::get_cluster_mut`](crate::syntax::SyntaxTree::get_cluster_mut) function under
    /// the hood.
    #[inline(always)]
    pub fn deref_mut<'tree, N: Node>(
        &self,
        tree: &'tree mut impl SyntaxTree<Node = N>,
    ) -> Option<&'tree mut Cluster<N>> {
        if self.id != tree.id() {
            return None;
        }

        tree.get_cluster_mut(&self.cluster_ref)
    }

    #[inline(always)]
    pub fn span(&self, tree: &impl SyntaxTree) -> SiteRefSpan {
        if self.id != tree.id() {
            return SiteRef::nil()..SiteRef::nil();
        }

        tree.get_cluster_span(&self.cluster_ref)
    }

    #[inline(always)]
    pub fn next(&self, tree: &impl SyntaxTree) -> Self {
        if self.id != tree.id() {
            return Self::nil();
        }

        match tree.get_next_cluster(&self.cluster_ref) {
            Ref::Nil => Self::nil(),

            other => ClusterRef {
                id: self.id,
                cluster_ref: other,
            },
        }
    }

    #[inline(always)]
    pub fn previous(&self, tree: &impl SyntaxTree) -> Self {
        if self.id != tree.id() {
            return Self::nil();
        }

        match tree.get_previous_cluster(&self.cluster_ref) {
            Ref::Nil => Self::nil(),

            other => ClusterRef {
                id: self.id,
                cluster_ref: other,
            },
        }
    }

    #[inline(always)]
    pub fn take<N: Node>(&self, tree: &mut impl SyntaxTree<Node = N>) -> Option<Cluster<N>> {
        if self.id != tree.id() {
            return None;
        }

        tree.remove_cluster(&self.cluster_ref)
    }

    /// Adds new `node` into the weakly referred [Cluster] of specified `tree` instance.
    ///
    /// This function consumes `node` value, and adds it to the
    /// [`Cluster::nodes`](crate::syntax::Cluster::nodes) secondary nodes repository.
    ///
    /// Returns valid [NodeRef](crate::syntax::NodeRef) if this ClusterRef weak reference is a valid
    /// reference into specified `tree` instance. Otherwise returns invalid NodeRef. Use
    /// [is_valid_ref](crate::syntax::ClusterRef::is_valid_ref) to check ClusterRef validity
    /// beforehand.
    ///
    /// This function uses
    /// [`SyntaxTree::get_cluster_mut`](crate::syntax::SyntaxTree::get_cluster_mut) function under
    /// the hood.
    ///
    /// Note that added node(or any other secondary node of the cluster) could be later removed from
    /// the cluster using [`NodeRef::unlink`](crate::syntax::NodeRef::unlink) function.
    #[inline]
    pub fn link_node<N: Node>(&self, tree: &mut impl SyntaxTree<Node = N>, node: N) -> NodeRef {
        if self.id != tree.id() {
            return NodeRef::nil();
        }

        let cluster = match tree.get_cluster_mut(&self.cluster_ref) {
            Some(cluster) => cluster,

            None => return NodeRef::nil(),
        };

        let node_ref = cluster.nodes.insert(node);

        NodeRef {
            id: self.id,
            cluster_ref: self.cluster_ref,
            node_ref,
        }
    }

    /// Adds new `error` into the weakly referred [Cluster] of specified `tree` instance.
    ///
    /// This function consumes `error` value, and adds it to the
    /// [`Cluster::error`](crate::syntax::Cluster::error) syntax/semantic errors repository.
    ///
    /// Returns valid [ErrorRef](crate::syntax::ErrorRef) if this ClusterRef weak reference is a
    /// valid reference into specified `tree` instance. Otherwise returns invalid ErrorRef. Use
    /// [is_valid_ref](crate::syntax::ClusterRef::is_valid_ref) to check ClusterRef validity
    /// beforehand.
    ///
    /// This function uses
    /// [`SyntaxTree::get_cluster_mut`](crate::syntax::SyntaxTree::get_cluster_mut) function under
    /// the hood.
    ///
    /// Note that added error could be later removed from
    /// the cluster using [`ErrorRef::unlink`](crate::syntax::ErrorRef::unlink) function.
    #[inline]
    pub fn link_error<N: Node>(
        &self,
        tree: &mut impl SyntaxTree<Node = N>,
        error: N::Error,
    ) -> ErrorRef {
        if self.id != tree.id() {
            return ErrorRef::nil();
        }

        let cluster = match tree.get_cluster_mut(&self.cluster_ref) {
            Some(cluster) => cluster,

            None => return ErrorRef::nil(),
        };

        let error_ref = cluster.errors.insert(error);

        ErrorRef {
            id: self.id,
            cluster_ref: self.cluster_ref,
            error_ref,
        }
    }

    /// Returns `true` if and only if referred weak Cluster reference belongs to specified
    /// [SyntaxTree](crate::syntax::SyntaxTree), and referred Cluster exists in this SyntaxTree
    /// instance.
    ///
    /// This function uses [`SyntaxTree::contains`](crate::syntax::SyntaxTree::contains_cluster)
    /// function under the hood.
    #[inline(always)]
    pub fn is_valid_ref(&self, tree: &impl SyntaxTree) -> bool {
        if self.id != tree.id() {
            return false;
        }

        tree.contains_cluster(&self.cluster_ref)
    }
}
