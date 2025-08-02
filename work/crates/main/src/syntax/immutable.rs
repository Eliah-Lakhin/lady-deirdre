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
    fmt::{Debug, Formatter},
    iter::FusedIterator,
    marker::PhantomData,
    ops::Range,
};

use crate::{
    arena::{Entry, EntryIndex, Id, Identifiable, SubId},
    lexis::TokenCursor,
    report::system_panic,
    syntax::{
        observer::ObservableSyntaxSession,
        session::ImmutableSyntaxSession,
        ErrorRef,
        Node,
        NodeRef,
        Observer,
        SyntaxError,
        SyntaxSession,
        SyntaxTree,
        ROOT_RULE,
    },
};

/// A canonical implementation of the [SyntaxTree] trait.
///
/// Parses a syntax grammar only and does not provides any reparse
/// capabilities.
///
/// Use this interface when you need to build a syntax tree from already
/// existing stream of tokens or a part of it.
pub struct ImmutableSyntaxTree<N: Node> {
    id: SubId,
    nodes: Vec<Option<N>>,
    errors: Vec<SyntaxError>,
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
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter
            .debug_struct("SyntaxTree")
            .field("id", &self.id())
            .finish_non_exhaustive()
    }
}

impl<N: Node> Identifiable for ImmutableSyntaxTree<N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id.id()
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
            id: self.id.id(),
            entry: Entry {
                index: 0,
                version: 0,
            },
        }
    }

    #[inline(always)]
    fn node_refs(&self) -> Self::NodeIterator<'_> {
        NodeIter {
            id: self.id.id(),
            inner: 0..self.nodes.len(),
        }
    }

    #[inline(always)]
    fn error_refs(&self) -> Self::ErrorIterator<'_> {
        ErrorIter {
            id: self.id.id(),
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
    fn get_error(&self, entry: &Entry) -> Option<&SyntaxError> {
        if entry.version > 0 {
            return None;
        }

        self.errors.get(entry.index)
    }
}

impl<N: Node> ImmutableSyntaxTree<N> {
    /// An ImmutableSyntaxTree constructor.
    ///
    /// The `token_cursor` parameter of type [TokenCursor] provides access
    /// to the token stream that needs to be parsed.
    ///
    /// You can get this cursor from a [TokenBuffer](crate::lexis::TokenBuffer),
    /// [TokenStream](crate::lexis::TokenStream), or any compilation unit type
    /// (e.g., [Document](crate::units::Document)).
    ///
    /// See [SourceCode::cursor](crate::lexis::SourceCode::cursor) function for
    /// details.
    #[inline(always)]
    pub fn parse<'code>(token_cursor: impl TokenCursor<'code, Token = <N as Node>::Token>) -> Self {
        Self::parse_with_id(SubId::new(), token_cursor)
    }

    /// An extended [constructor](Self::parse), where the additional parameter
    /// `observer` specifies an [Observer] object into which the syntax parser
    /// will report parsing steps.
    ///
    /// One use case of the observer is to debug your parser. In particular,
    /// the [DebugObserver](crate::syntax::DebugObserver) prints each parsing
    /// step performed by the syntax parser to the stdout.
    #[inline(always)]
    pub fn parse_with_observer<'code>(
        token_cursor: impl TokenCursor<'code, Token = <N as Node>::Token>,
        observer: &mut impl Observer<Node = N>,
    ) -> Self {
        Self::parse_with_id_and_observer(SubId::new(), token_cursor, observer)
    }

    pub(crate) fn parse_with_id<'code, 'observer>(
        id: SubId,
        token_cursor: impl TokenCursor<'code, Token = <N as Node>::Token>,
    ) -> Self {
        let mut session = ImmutableSyntaxSession {
            id: id.id(),
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
        id: SubId,
        token_cursor: impl TokenCursor<'code, Token = <N as Node>::Token>,
        observer: &'observer mut impl Observer<Node = N>,
    ) -> Self {
        let mut session = ObservableSyntaxSession {
            id: id.id(),
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
