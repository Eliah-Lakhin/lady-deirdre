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
    arena::{Entry, EntryIndex, Id, Identifiable},
    lexis::TokenCursor,
    report::system_panic,
    std::*,
    syntax::{
        observer::{ObservableSyntaxSession, VoidObserver},
        session::ImmutableSyntaxSession,
        ErrorRef,
        Node,
        NodeRef,
        Observer,
        SyntaxSession,
        SyntaxTree,
        ROOT_RULE,
    },
};

/// A non-incrementally managed syntax structure of a compilation unit.
///
/// SyntaxBuffer is a simple implementation of the [SyntaxTree](crate::syntax::SyntaxTree) interface
/// that runs a syntax grammar parser over the sequence of tokens just once to produce and to store
/// a syntax structure of a compilation unit. In contrast to [Document](crate::Document),
/// SyntaxBuffer does not provide source code mutation operations(incremental re-parsing
/// operations). However the syntax structure stored by this object is still a mutable structure by
/// itself, an API user can mutate its nodes manually using [Cluster](crate::syntax::Cluster) and
/// similar mutation operations.
///
/// The syntax grammar of the programming language and the syntax structure type specified by the
/// SyntaxBuffer's generic parameter of [Node](crate::syntax::Node) type.
///
/// To crate a SyntaxBuffer use [`Node::parse`](crate::syntax::Node::parse) function.
///
/// ```rust
/// use lady_deirdre::{
///     lexis::{TokenBuffer, SimpleToken, SourceCode, Token},
///     syntax::{ImmutableSyntaxTree, SimpleNode, SyntaxTree, NodeRef, Node},
/// };
///
/// let token_buffer = TokenBuffer::parse("foo({bar}[baz])");
/// let syntax_buffer = ImmutableSyntaxTree::parse(token_buffer.cursor(..));
///
/// fn format(tree: &impl SyntaxTree<Node = SimpleNode>, node: &NodeRef) -> String {
///     let node = node.deref(tree).unwrap();
///
///     let inner = node
///         .inner()
///         .iter()
///         .map(|inner_node_ref: &NodeRef| format(tree, inner_node_ref))
///         .collect::<Vec<_>>()
///         .join("");
///
///     match node {
///         SimpleNode::Root { .. } => inner,
///         SimpleNode::Parenthesis { .. } => format!("({})", inner),
///         SimpleNode::Brackets { .. } => format!("[{}]", inner),
///         SimpleNode::Braces { .. } => format!("{{{}}}", inner),
///     }
/// }
///
/// assert_eq!("({}[])", format(&syntax_buffer, &syntax_buffer.root_node_ref()));
/// ```
pub struct ImmutableSyntaxTree<N: Node> {
    id: Id,
    nodes: Vec<Option<N>>,
    errors: Vec<N::Error>,
}

impl<N: Node> PartialEq for ImmutableSyntaxTree<N> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl<N: Node> Eq for ImmutableSyntaxTree<N> {}

impl<N: Node> Debug for ImmutableSyntaxTree<N> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter
            .debug_struct("SyntaxTree")
            .field("id", &self.id())
            .finish_non_exhaustive()
    }
}

impl<N: Node> Drop for ImmutableSyntaxTree<N> {
    fn drop(&mut self) {
        self.id.clear_name();
    }
}

impl<N: Node> Identifiable for ImmutableSyntaxTree<N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<N: Node> SyntaxTree for ImmutableSyntaxTree<N> {
    type Node = N;

    type NodeIterator<'tree> = NodeIter;

    type ErrorIterator<'tree> = ErrorIter;

    #[inline(always)]
    fn root_node_ref(&self) -> NodeRef {
        #[cfg(debug_assertions)]
        if self.nodes.is_empty() {
            system_panic!("Empty syntax tree.");
        }

        NodeRef {
            id: self.id,
            entry: Entry {
                index: 0,
                version: 0,
            },
        }
    }

    #[inline(always)]
    fn node_refs(&self) -> Self::NodeIterator<'_> {
        NodeIter {
            id: self.id,
            inner: 0..self.nodes.len(),
        }
    }

    #[inline(always)]
    fn error_refs(&self) -> Self::ErrorIterator<'_> {
        ErrorIter {
            id: self.id,
            inner: 0..self.errors.len(),
        }
    }

    #[inline(always)]
    fn has_node(&self, entry: &Entry) -> bool {
        if entry.version > 0 {
            return false;
        }

        entry.index < self.nodes.len()
    }

    #[inline(always)]
    fn get_node(&self, entry: &Entry) -> Option<&Self::Node> {
        if entry.version > 0 {
            return None;
        }

        self.nodes.get(entry.index)?.as_ref()
    }

    #[inline(always)]
    fn get_node_mut(&mut self, entry: &Entry) -> Option<&mut Self::Node> {
        if entry.version > 0 {
            return None;
        }

        self.nodes.get_mut(entry.index)?.as_mut()
    }

    #[inline(always)]
    fn has_error(&self, entry: &Entry) -> bool {
        if entry.version > 0 {
            return false;
        }

        entry.index < self.errors.len()
    }

    #[inline(always)]
    fn get_error(&self, entry: &Entry) -> Option<&<Self::Node as Node>::Error> {
        if entry.version > 0 {
            return None;
        }

        self.errors.get(entry.index)
    }
}

impl<N: Node> ImmutableSyntaxTree<N> {
    #[inline(always)]
    pub fn parse<'code>(token_cursor: impl TokenCursor<'code, Token = <N as Node>::Token>) -> Self {
        Self::parse_with_id(Id::new(), token_cursor)
    }

    #[inline(always)]
    pub fn parse_with_observer<'code>(
        token_cursor: impl TokenCursor<'code, Token = <N as Node>::Token>,
        observer: &mut impl Observer<Node = N>,
    ) -> Self {
        Self::parse_with_id_and_observer(Id::new(), token_cursor, observer)
    }

    pub(crate) fn parse_with_id<'code, 'observer>(
        id: Id,
        token_cursor: impl TokenCursor<'code, Token = <N as Node>::Token>,
    ) -> Self {
        let mut session = ImmutableSyntaxSession {
            id,
            context: Vec::new(),
            nodes: Vec::new(),
            errors: Vec::new(),
            failing: false,
            token_cursor,
            _phantom: PhantomData,
        };

        let _ = session.descend(ROOT_RULE);

        Self {
            id,
            nodes: session.nodes,
            errors: session.errors,
        }
    }

    pub(crate) fn parse_with_id_and_observer<'code, 'observer>(
        id: Id,
        token_cursor: impl TokenCursor<'code, Token = <N as Node>::Token>,
        observer: &'observer mut impl Observer<Node = N>,
    ) -> Self {
        let mut session = ObservableSyntaxSession {
            id,
            context: Vec::new(),
            nodes: Vec::new(),
            errors: Vec::new(),
            failing: false,
            token_cursor,
            observer,
            _phantom: PhantomData,
        };

        let _ = session.descend(ROOT_RULE);

        Self {
            id,
            nodes: session.nodes,
            errors: session.errors,
        }
    }
}

pub struct NodeIter {
    id: Id,
    inner: Range<EntryIndex>,
}

impl Iterator for NodeIter {
    type Item = NodeRef;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.inner.next()?;

        Some(NodeRef {
            id: self.id,
            entry: Entry { index, version: 0 },
        })
    }
}

impl FusedIterator for NodeIter {}

pub struct ErrorIter {
    id: Id,
    inner: Range<EntryIndex>,
}

impl Iterator for ErrorIter {
    type Item = ErrorRef;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.inner.next()?;

        Some(ErrorRef {
            id: self.id,
            entry: Entry { index, version: 0 },
        })
    }
}

impl FusedIterator for ErrorIter {}
