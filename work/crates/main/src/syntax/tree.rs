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

use std::iter::FusedIterator;

use crate::{
    arena::{Entry, Identifiable},
    lexis::TokenRef,
    syntax::{AbstractNode, ErrorRef, Node, NodeRef, RefKind, SyntaxError},
};

/// An object that provides access to the syntax structure of
/// a compilation unit.
///
/// The syntax structure consists of a set of [nodes](Node) forming an abstract
/// syntax tree, and a set of syntax [errors](SyntaxError) that may occur during
/// parsing or incremental reparsing.
///
/// In Lady Deirdre, instances of these objects are owned by the compilation
/// units, rather than by the syntax tree nodes. This ownership structure allows
/// compilation units to have full control over the syntax structure,
/// especially for the purpose of incremental reparsing.
///
/// Syntax tree nodes reference their children, parents, and other nodes
/// through a system of versioned indices ([Entry]).
///
/// This trait provides a low-level interface to borrow instance of syntax nodes
/// and the syntax errors by index from the compilation unit.
///
/// Higher-level referential objects, such as [NodeRef] and [ErrorRef], offer
/// a more convenient interface for borrowing these objects from the SyntaxTree.
///
/// Additionally, the SyntaxTree interface provides higher-level functions
/// to retrieve the syntax tree root node, iterate through all nodes and errors
/// currently managed by the compilation unit, and perform a depth-first
/// traversal of the syntax tree.
///
/// Typically, manual implementation of this trait is unnecessary unless
/// creating a new type of compilation unit manager.
///
/// To create a wrapper of an existing compilation unit,
/// a [Syntax](crate::units::Syntax) facade-interface can be utilized,
/// which auto-implements this trait through delegation.
pub trait SyntaxTree: Identifiable {
    /// Specifies the type of the tree node.
    ///
    /// [Node::Token] inherently specifies the lexical grammar of the language.
    ///
    /// [Node::parse] inherently specifies the syntax parser of the language.
    type Node: Node;

    /// Specifies the type of the iterator that walks through all node
    /// [NodeRef] references currently managed by this SyntaxTree instance.
    type NodeIterator<'tree>: Iterator<Item = NodeRef> + FusedIterator + 'tree
    where
        Self: 'tree;

    /// Specifies the type of the iterator that walks through all syntax error
    /// [ErrorRef] references currently managed by this SyntaxTree instance.
    type ErrorIterator<'tree>: Iterator<Item = ErrorRef> + FusedIterator + 'tree
    where
        Self: 'tree;

    /// Provides access to the root node of the syntax tree.
    ///
    /// **Panic**
    ///
    /// Depending on the implementation, this function may panic if
    /// the SyntaxTree does not have a root node.
    ///
    /// However, all objects within this crate that implement
    /// the SyntaxTree trait always have the root node regardless of the input.
    /// Therefore, they would **never** panic.
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

    /// Returns a [NodeRef] reference to the root node of this syntax tree.
    fn root_node_ref(&self) -> NodeRef;

    /// Returns an iterator over all nodes currently managed by this syntax tree.
    ///
    /// The order of iteration is not specified.
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

    /// Returns an iterator of the [NodeRef] references over all nodes currently
    /// managed by this syntax tree.
    ///
    /// The order of iteration is not specified.
    fn node_refs(&self) -> Self::NodeIterator<'_>;

    /// Returns an iterator over all syntax errors currently managed by
    /// this syntax tree.
    ///
    /// The order of iteration is not specified.
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

    /// Returns an iterator of the [ErrorRef] references over all syntax errors
    /// currently managed by this syntax tree.
    ///
    /// The order of iteration is not specified.
    fn error_refs(&self) -> Self::ErrorIterator<'_>;

    /// Performs a depth-first traverse of the syntax tree starting from
    /// the root node.
    ///
    /// The `visitor` object will be called on each node entering and
    /// leaving events, as well as the token entering event.
    ///
    /// The traverse algorithm will not visit descending nodes of the tree
    /// branch if the [Visitor::enter_node] returns false. Thus, the visitor
    /// can control the descending process.
    ///
    /// The algorithm relies on the [AbstractNode::children_iter] function
    /// to determine the node's children to descend to, which in turn relies on
    /// the node's captures description.
    ///
    /// In other words, the traverser will only visit the nodes for which you
    /// have specified `#[child]` attribute:
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     #[rule(...)]
    ///     SomeVariant {
    ///         #[child]
    ///         child_1: NodeRef, // will be visited
    ///         #[child]
    ///         child_2: NodeRef, // will be visited
    ///         not_a_child: NodeRef, // will not be visited
    ///     }
    /// }
    /// ```
    #[inline(always)]
    fn traverse_tree(&self, visitor: &mut impl Visitor)
    where
        Self: Sized,
    {
        self.traverse_subtree(&self.root_node_ref(), visitor);
    }

    /// Performs a depth-first traverse of a branch of the syntax tree.
    ///
    /// The `top` parameter specifies a reference into the top node of
    /// the branch.
    ///
    /// For details, see [SyntaxTree::traverse_tree].
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

    /// Checks if the node referred to by the versioned index exists in this
    /// syntax tree.
    fn has_node(&self, entry: &Entry) -> bool;

    /// Provides immutable access to the node referred to by the versioned index.
    ///
    /// If the index parameter `entry` is not valid, returns None.
    fn get_node(&self, entry: &Entry) -> Option<&Self::Node>;

    /// Provides mutable access to the node referred to by the versioned index.
    ///
    /// If the index parameter `entry` is not valid, returns None.
    fn get_node_mut(&mut self, entry: &Entry) -> Option<&mut Self::Node>;

    /// Checks if the syntax error referred to by the versioned index exists in
    /// this syntax tree.
    fn has_error(&self, entry: &Entry) -> bool;

    /// Provides access to the syntax error referred to by the versioned index.
    ///
    /// If the index parameter `entry` is not valid, returns None.
    fn get_error(&self, entry: &Entry) -> Option<&SyntaxError>;
}

/// An iterator over all nodes of the [SyntaxTree].
///
/// This object is created by the [SyntaxTree::nodes] function.
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

/// An iterator over all syntax errors of the [SyntaxTree].
///
/// This object is created by the [SyntaxTree::errors] function.
pub struct ErrorIter<'tree, T: SyntaxTree> {
    tree: &'tree T,
    inner: <T as SyntaxTree>::ErrorIterator<'tree>,
}

impl<'tree, T: SyntaxTree> Iterator for ErrorIter<'tree, T> {
    type Item = &'tree SyntaxError;

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

/// Visits individual nodes and tokens during the syntax tree
/// depth-first traversing.
///
/// See [SyntaxTree::traverse_tree] for details.
pub trait Visitor {
    /// Triggers when the traverser visits a token child of a node.
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     #[rule(...)]
    ///     SomeVariant {
    ///         #[child]
    ///         node_child: NodeRef,
    ///         #[child]
    ///         token_child: TokenRef, // <- when this type of children is visited
    ///     }
    /// }
    /// ```
    fn visit_token(&mut self, token_ref: &TokenRef);

    /// Triggers when the traverser enters a node.
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     #[rule(...)]
    ///     SomeVariant {
    ///         #[child]
    ///         node_child: NodeRef, // <- when this type of children is visited
    ///         #[child]
    ///         token_child: TokenRef,
    ///     }
    /// }
    /// ```
    ///
    /// Returning false prevents the traverser further descending into
    /// the node's sub-branch.
    fn enter_node(&mut self, node_ref: &NodeRef) -> bool;

    /// Triggers when the traverser leaves a node.
    ///
    /// In practice, this function observes depth-first traversing
    /// in reverse order.
    fn leave_node(&mut self, node_ref: &NodeRef);
}
