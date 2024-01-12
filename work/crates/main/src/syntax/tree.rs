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
    arena::{Entry, Identifiable},
    lexis::TokenRef,
    std::*,
    syntax::{AbstractNode, ErrorRef, Node, NodeRef, RefKind},
};

/// A low-level interface to access and inspect syntax structure of the compilation unit.
///
/// SyntaxTree by convenient should be implemented for the compilation unit management object such
/// as [Document](crate::Document) and [SyntaxBuffer](crate::syntax::ImmutableSyntaxTree) objects that
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

    type NodeIterator<'tree>: Iterator<Item = NodeRef> + FusedIterator + 'tree
    where
        Self: 'tree;

    type ErrorIterator<'tree>: Iterator<Item = ErrorRef> + FusedIterator + 'tree
    where
        Self: 'tree;

    #[inline(always)]
    fn root(&self) -> &Self::Node
    where
        Self: Sized,
    {
        match self.root_node_ref().deref(self) {
            Some(node) => node,
            None => panic!("Syntax tree without root."),
        }
    }

    fn root_node_ref(&self) -> NodeRef;

    #[inline(always)]
    fn nodes(&self) -> NodeIter<'_, Self>
    where
        Self: Sized,
    {
        NodeIter {
            tree: self,
            inner: self.node_refs(),
        }
    }

    fn node_refs(&self) -> Self::NodeIterator<'_>;

    #[inline(always)]
    fn errors(&self) -> ErrorIter<'_, Self>
    where
        Self: Sized,
    {
        ErrorIter {
            tree: self,
            inner: self.error_refs(),
        }
    }

    fn error_refs(&self) -> Self::ErrorIterator<'_>;

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

            for child in node.children_iter() {
                match child.kind() {
                    RefKind::Token => visitor.visit_token(child.as_token_ref()),
                    RefKind::Node => self.traverse_subtree(child.as_node_ref(), visitor),
                }
            }
        }

        visitor.leave_node(top)
    }

    fn has_node(&self, entry: &Entry) -> bool;

    fn get_node(&self, entry: &Entry) -> Option<&Self::Node>;

    fn get_node_mut(&mut self, entry: &Entry) -> Option<&mut Self::Node>;

    fn has_error(&self, entry: &Entry) -> bool;

    fn get_error(&self, entry: &Entry) -> Option<&<Self::Node as Node>::Error>;
}

pub struct NodeIter<'tree, T: SyntaxTree> {
    tree: &'tree T,
    inner: <T as SyntaxTree>::NodeIterator<'tree>,
}

impl<'tree, T: SyntaxTree> Iterator for NodeIter<'tree, T> {
    type Item = &'tree <T as SyntaxTree>::Node;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.inner.next()?;

            let Some(node) = self.tree.get_node(&next.entry) else {
                continue;
            };

            return Some(node);
        }
    }
}

impl<'tree, T: SyntaxTree> FusedIterator for NodeIter<'tree, T> {}

pub struct ErrorIter<'tree, T: SyntaxTree> {
    tree: &'tree T,
    inner: <T as SyntaxTree>::ErrorIterator<'tree>,
}

impl<'tree, T: SyntaxTree> Iterator for ErrorIter<'tree, T> {
    type Item = &'tree <<T as SyntaxTree>::Node as Node>::Error;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.inner.next()?;

            let Some(error) = self.tree.get_error(&next.entry) else {
                continue;
            };

            return Some(error);
        }
    }
}

impl<'tree, T: SyntaxTree> FusedIterator for ErrorIter<'tree, T> {}

pub trait Visitor {
    fn visit_token(&mut self, token_ref: &TokenRef);

    fn enter_node(&mut self, node_ref: &NodeRef) -> bool;

    fn leave_node(&mut self, node_ref: &NodeRef);
}
