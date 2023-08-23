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
    arena::{Entry, Id, Identifiable, RepositoryIterator},
    std::*,
    syntax::{ClusterRef, Node, NodeRef, SyntaxTree},
};

pub trait TreeContent: SyntaxTree {
    type NodeIterator<'tree>: Identifiable + Iterator<Item = &'tree Self::Node> + FusedIterator
    where
        Self: 'tree;

    type ErrorIterator<'tree>: Identifiable
        + Iterator<Item = &'tree <Self::Node as Node>::Error>
        + FusedIterator
    where
        Self: 'tree;

    fn root_node_ref(&self) -> NodeRef;

    fn root_cluster_ref(&self) -> ClusterRef;

    fn nodes(&self) -> Self::NodeIterator<'_>;

    fn errors(&self) -> Self::ErrorIterator<'_>;
}

impl<T: SyntaxTree> TreeContent for T {
    type NodeIterator<'tree> = NodeIterator<'tree, T> where Self: 'tree;
    type ErrorIterator<'tree> = ErrorIterator<'tree, T> where Self: 'tree;

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
    fn nodes(&self) -> Self::NodeIterator<'_> {
        NodeIterator {
            tree: self,
            inner: NodeIteratorInner::Root,
        }
    }

    #[inline(always)]
    fn errors(&self) -> Self::ErrorIterator<'_> {
        let cluster_ref = self.root_cluster_ref();

        let cluster = match cluster_ref.deref(self) {
            Some(cluster) => cluster,
            None => panic!("Root cluster dereference failure."),
        };

        ErrorIterator {
            tree: self,
            cluster_ref,
            current: (&cluster.errors).into_iter(),
        }
    }
}

pub struct NodeIterator<'tree, T: SyntaxTree> {
    tree: &'tree T,
    inner: NodeIteratorInner<'tree, T>,
}

impl<'tree, T: SyntaxTree> Identifiable for NodeIterator<'tree, T> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.tree.id()
    }
}

impl<'tree, T: SyntaxTree> Iterator for NodeIterator<'tree, T> {
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

impl<'tree, T: SyntaxTree> FusedIterator for NodeIterator<'tree, T> {}

enum NodeIteratorInner<'tree, T: SyntaxTree> {
    Root,

    NonRoot {
        cluster_ref: ClusterRef,
        current: RepositoryIterator<'tree, T::Node>,
    },
}

pub struct ErrorIterator<'tree, T: SyntaxTree> {
    tree: &'tree T,
    cluster_ref: ClusterRef,
    current: RepositoryIterator<'tree, <T::Node as Node>::Error>,
}

impl<'tree, T: SyntaxTree> Identifiable for ErrorIterator<'tree, T> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.tree.id()
    }
}

impl<'tree, T: SyntaxTree> Iterator for ErrorIterator<'tree, T> {
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

impl<'tree, T: SyntaxTree> FusedIterator for ErrorIterator<'tree, T> {}
