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
    arena::{Entry, Identifiable, RepositoryIterator},
    lexis::TokenRef,
    std::*,
    syntax::{Cluster, ClusterRef, Node, NodeRef, RefKind},
};

/// A low-level interface to access and inspect syntax structure of the compilation unit.
///
/// SyntaxTree by convenient should be implemented for the compilation unit management object such
/// as [Document](crate::Document) and [SyntaxBuffer](crate::syntax::SyntaxBuffer) objects that
/// supposed to manage code's syntax grammar structure.
///
/// This trait:
///   1. Specifies syntax grammar through the [Node](crate::syntax::SyntaxTree::Node) associative
///      type.
///   2. Provides a [root](crate::syntax::SyntaxTree::root_node_ref) function to obtain a weak reference to
///      the root node of the syntax tree. An API uses utilizes this function to enter into the
///      the syntax tree structure, and uses received reference to further inspect and traverse this
///      syntax structure.
///   3. Provides an [errors](crate::syntax::SyntaxTree::errors) function to obtain an
///      [iterator](crate::syntax::SyntaxTree::ErrorIterator) over all syntax and semantic errors
///      associated with this compilation unit.
///   4. Provides low-level interface to resolve higher-level weak references(such as
///      [ClusterRef](crate::syntax::ClusterRef), [NodeRef](crate::syntax::NodeRef), or
///      [ErrorRef](crate::syntax::ErrorRef)).
///
/// In practice an API user interacts with a small subset of this functionality directly.
///
/// To implement an extension library to this Crate with the source code management of alternative
/// designs, you can implement this trait over these objects. In this case these new objects will be
/// able to interact with existing [Node](crate::syntax::Node) implementations, and the weak
/// references belong to them will work transparently with other conventional weak references.
pub trait SyntaxTree: Identifiable {
    /// Specifies programming language lexical grammar.
    ///
    /// See [Node](crate::syntax::Node) for details.
    type Node: Node;

    #[inline(always)]
    fn root_node_ref(&self) -> NodeRef {
        NodeRef {
            id: self.id(),
            cluster_entry: Entry::Primary,
            node_entry: Entry::Primary,
        }
    }

    #[inline(always)]
    fn root_cluster_ref(&self) -> ClusterRef {
        ClusterRef {
            id: self.id(),
            cluster_entry: Entry::Primary,
        }
    }

    #[inline(always)]
    fn nodes(&self) -> NodeIter<'_, Self>
    where
        Self: Sized,
    {
        NodeIter {
            tree: self,
            inner: NodeIteratorInner::Root,
        }
    }

    #[inline(always)]
    fn errors(&self) -> ErrorIter<'_, Self>
    where
        Self: Sized,
    {
        let cluster_ref = self.root_cluster_ref();

        let cluster = match cluster_ref.deref(self) {
            Some(cluster) => cluster,
            None => panic!("Root cluster dereference failure."),
        };

        ErrorIter {
            tree: self,
            cluster_ref,
            current: (&cluster.errors).into_iter(),
        }
    }

    #[inline(always)]
    fn traverse_tree(&self, visitor: &mut impl Visitor)
    where
        Self: Sized,
    {
        self.traverse_subtree(&self.root_node_ref(), visitor);
    }

    fn traverse_subtree(&self, top: &NodeRef, visitor: &mut impl Visitor)
    where
        Self: Sized,
    {
        if visitor.enter_node(top) {
            let node: &Self::Node = match top.deref(self) {
                Some(node) => node,
                None => return,
            };

            let children = node.children();

            for child in children.flatten() {
                match child.kind() {
                    RefKind::Token => visitor.visit_token(child.as_token_ref()),
                    RefKind::Node => self.traverse_subtree(child.as_node_ref(), visitor),
                }
            }
        }

        visitor.leave_node(top)
    }

    /// Returns `true` if the [`Node Cluster`](crate::syntax::ClusterRef) referred by specified
    /// low-level `cluster_ref` weak reference exists in this syntax tree instance.
    ///
    /// This is a low-level API used by the higher-level [ClusterRef](crate::syntax::ClusterRef),
    /// [NodeRef](crate::syntax::NodeRef) and [ErrorRef](crate::syntax::ErrorRef) weak references
    /// under the hood. An API user normally don't need to call this function directly.
    fn has_cluster(&self, cluster_entry: &Entry) -> bool;

    /// Immutably dereferences a [Cluster](crate::syntax::Cluster) instance by specified low-level
    /// `cluster_ref` weak reference.
    ///
    /// Returns [None] if referred Cluster does not exist in this instance.
    ///
    /// This is a low-level API used by the higher-level [ClusterRef](crate::syntax::ClusterRef),
    /// [NodeRef](crate::syntax::NodeRef) and [ErrorRef](crate::syntax::ErrorRef) weak references
    /// under the hood. An API user normally don't need to call this function directly.
    fn get_cluster(&self, cluster_entry: &Entry) -> Option<&Cluster<Self::Node>>;

    /// Mutably dereferences a [Cluster](crate::syntax::Cluster) instance by specified low-level
    /// `cluster_ref` weak reference.
    ///
    /// Returns [None] if referred Cluster does not exist in this instance.
    ///
    /// This is a low-level API used by the higher-level [ClusterRef](crate::syntax::ClusterRef),
    /// [NodeRef](crate::syntax::NodeRef) and [ErrorRef](crate::syntax::ErrorRef) weak references
    /// under the hood. An API user normally don't need to call this function directly.
    fn get_cluster_mut(&mut self, cluster_entry: &Entry) -> Option<&mut Cluster<Self::Node>>;

    fn get_previous_cluster(&self, cluster_entry: &Entry) -> Entry;

    fn get_next_cluster(&self, cluster_entry: &Entry) -> Entry;

    fn remove_cluster(&mut self, cluster_entry: &Entry) -> Option<Cluster<Self::Node>>;
}

pub struct NodeIter<'tree, T: SyntaxTree> {
    tree: &'tree T,
    inner: NodeIteratorInner<'tree, T>,
}

impl<'tree, T: SyntaxTree> Iterator for NodeIter<'tree, T> {
    type Item = &'tree T::Node;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            NodeIteratorInner::Root => {
                let cluster_ref = self.tree.root_cluster_ref();

                let cluster = match cluster_ref.deref(self.tree) {
                    Some(cluster) => cluster,
                    None => panic!("Root cluster dereference failure."),
                };

                self.inner = NodeIteratorInner::NonRoot {
                    cluster_ref: self.tree.root_cluster_ref(),
                    current: (&cluster.nodes).into_iter(),
                };

                Some(&cluster.primary)
            }

            NodeIteratorInner::NonRoot {
                cluster_ref,
                current,
            } => {
                if let Some(node) = current.next() {
                    return Some(node);
                }

                let cluster_ref = cluster_ref.next(self.tree);

                let cluster = match cluster_ref.deref(self.tree) {
                    Some(cluster) => cluster,
                    None => return None,
                };

                self.inner = NodeIteratorInner::NonRoot {
                    cluster_ref,
                    current: (&cluster.nodes).into_iter(),
                };

                Some(&cluster.primary)
            }
        }
    }
}

impl<'tree, T: SyntaxTree> FusedIterator for NodeIter<'tree, T> {}

enum NodeIteratorInner<'tree, T: SyntaxTree> {
    Root,

    NonRoot {
        cluster_ref: ClusterRef,
        current: RepositoryIterator<'tree, T::Node>,
    },
}

pub struct ErrorIter<'tree, T: SyntaxTree> {
    tree: &'tree T,
    cluster_ref: ClusterRef,
    current: RepositoryIterator<'tree, <T::Node as Node>::Error>,
}

impl<'tree, T: SyntaxTree> Iterator for ErrorIter<'tree, T> {
    type Item = &'tree <T::Node as Node>::Error;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(error) = self.current.next() {
                return Some(error);
            }

            self.cluster_ref = self.cluster_ref.next(self.tree);

            let cluster = match self.cluster_ref.deref(self.tree) {
                Some(cluster) => cluster,
                None => return None,
            };

            self.current = (&cluster.errors).into_iter();
        }
    }
}

impl<'tree, T: SyntaxTree> FusedIterator for ErrorIter<'tree, T> {}

pub trait Visitor {
    fn visit_token(&mut self, token_ref: &TokenRef);

    fn enter_node(&mut self, node_ref: &NodeRef) -> bool;

    fn leave_node(&mut self, node_ref: &NodeRef);
}
