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
    lexis::{Length, Site, SiteRef, TokenCount, TokenCursor, TokenRef},
    report::debug_unreachable,
    std::*,
    syntax::{ErrorRef, Node, NodeRef, NodeRule, Observer},
};

/// An interface to the source code syntax parsing/re-parsing session.
///
/// This is a low-level API.
///
/// Syntax parsing architecture decoupled into two independent components:
///   - The Syntax Tree Manager that organizes a syntax structure storage, and that provides access
///     operations to the syntax structure objects. This component implements a
///     [SyntaxTree](crate::syntax::SyntaxTree) trait.
///   - The Syntax Parser of particular programming language. This component is unaware about the
///     syntax structure memory management process, and about the source of parsing.
///
/// Both components of this architecture are unaware about each other, and they use a
/// [SyntaxSession] trait as an input/output "thin" interaction interface.
///
/// The Syntax Tree Manager passes a mutable reference to SyntaxSession object to the
/// [`Node::new`](crate::syntax::Node::parse) function to initiate syntax parsing procedure in
/// specified context. And, in turn, the `Node::new` function uses this object to read
/// [Tokens](crate::lexis::Token) from the input sequence, and to drive the parsing process.
///
/// You can implement this trait as well as the [SyntaxTree](crate::syntax::SyntaxTree) trait to
/// create a custom syntax tree manager of the compilation unit that would be able to work with
/// existing syntax grammar definitions seamlessly.
///
/// As long as the the [Node](crate::syntax::Node) trait implementation follows
/// [`Algorithm Specification`](crate::syntax::Node::parse), the
/// intercommunication between the Syntax Parser and the Syntax Tree Manager works correctly too.
///
/// The SyntaxSession inherits [TokenCursor](crate::lexis::TokenCursor) trait that provides
/// input [Token](crate::lexis::Token) sequence read operations to be parsed by the Syntax Parser.
pub trait SyntaxSession<'code>: TokenCursor<'code, Token = <Self::Node as Node>::Token> {
    /// Specifies programming language grammar.
    type Node: Node;

    /// Performs descend operation into the syntax grammar subrule from the current
    /// [TokenCursor](crate::lexis::TokenCursor) inner [Site](crate::lexis::Site).
    ///
    /// Depending on implementation this function may recursively invoke
    /// [`Node::new`](crate::syntax::Node::parse) function under the hood to process specified `rule`,
    /// or get previously parsed value from the Syntax Tree Manager internal cache.
    ///
    /// The function returns a [`weak reference`](crate::syntax::NodeRef) into the parsed Node.
    ///
    /// The `Node::new` algorithm should prefer to call this function to recursively descend into
    /// the syntax grammar rules instead of the direct recursive invocation of the `Node::new`.
    ///
    /// By the [`Algorithm Specification`](crate::syntax::Node::parse) the `Node::new` function should
    /// avoid of calling of this function with the [ROOT_RULE](crate::syntax::ROOT_RULE) value.
    fn descend(&mut self, rule: NodeRule) -> NodeRef;

    fn enter(&mut self, rule: NodeRule) -> NodeRef;

    fn leave(&mut self, node: Self::Node) -> NodeRef;

    fn lift(&mut self, node_ref: &NodeRef);

    fn node_ref(&self) -> NodeRef;

    fn parent_ref(&self) -> NodeRef;

    /// Registers a syntax parse error.
    ///
    /// If the Syntax Parser encounters grammatically incorrect input sequence, it should recover
    /// this error and register all syntax errors objects of the currently parsed
    /// [RuleIndex](crate::syntax::NodeRule) using this function.
    ///
    /// The function returns a [`weak reference`](crate::syntax::ErrorRef) into registered error.
    fn failure(&mut self, error: impl Into<<Self::Node as Node>::Error>) -> ErrorRef;
}

pub(super) struct ImmutableSyntaxSession<
    'code,
    'observer,
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
    O: Observer<Node = N>,
> {
    pub(super) id: Id,
    pub(super) context: Vec<EntryIndex>,
    pub(super) nodes: Vec<Option<N>>,
    pub(super) errors: Vec<N::Error>,
    pub(super) failing: bool,
    pub(super) token_cursor: C,
    pub(super) observer: &'observer mut O,
    pub(super) _phantom: PhantomData<&'code ()>,
}

impl<'code, 'observer, N, C, O> Identifiable for ImmutableSyntaxSession<'code, 'observer, N, C, O>
where
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
    O: Observer<Node = N>,
{
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'code, 'observer, N, C, O> TokenCursor<'code>
    for ImmutableSyntaxSession<'code, 'observer, N, C, O>
where
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
    O: Observer<Node = N>,
{
    type Token = <N as Node>::Token;

    #[inline(always)]
    fn advance(&mut self) -> bool {
        let advanced = self.token_cursor.advance();

        self.failing = self.failing && !advanced;

        advanced
    }

    #[inline(always)]
    fn skip(&mut self, distance: TokenCount) {
        let start = self.token_cursor.site(0);

        self.token_cursor.skip(distance);

        self.failing = self.failing && start == self.token_cursor.site(0);
    }

    #[inline(always)]
    fn token(&mut self, distance: TokenCount) -> Self::Token {
        self.token_cursor.token(distance)
    }

    #[inline(always)]
    fn site(&mut self, distance: TokenCount) -> Option<Site> {
        self.token_cursor.site(distance)
    }

    #[inline(always)]
    fn length(&mut self, distance: TokenCount) -> Option<Length> {
        self.token_cursor.length(distance)
    }

    #[inline(always)]
    fn string(&mut self, distance: TokenCount) -> Option<&'code str> {
        self.token_cursor.string(distance)
    }

    #[inline(always)]
    fn token_ref(&mut self, distance: TokenCount) -> TokenRef {
        self.token_cursor.token_ref(distance)
    }

    #[inline(always)]
    fn site_ref(&mut self, distance: TokenCount) -> SiteRef {
        self.token_cursor.site_ref(distance)
    }

    #[inline(always)]
    fn end_site_ref(&mut self) -> SiteRef {
        self.token_cursor.end_site_ref()
    }
}

impl<'code, 'observer, N, C, O> SyntaxSession<'code>
    for ImmutableSyntaxSession<'code, 'observer, N, C, O>
where
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
    O: Observer<Node = N>,
{
    type Node = N;

    fn descend(&mut self, rule: NodeRule) -> NodeRef {
        let _ = self.enter(rule);

        let node = N::parse(self, rule);

        self.leave(node)
    }

    #[inline]
    fn enter(&mut self, rule: NodeRule) -> NodeRef {
        self.observer.enter_rule(rule);

        let index = self.nodes.len();

        self.nodes.push(None);

        self.context.push(index);

        NodeRef {
            id: self.id,
            entry: Entry { index, version: 0 },
        }
    }

    #[inline]
    fn leave(&mut self, node: Self::Node) -> NodeRef {
        self.observer.leave_rule(node.rule(), &node);

        let Some(index) = self.context.pop() else {
            #[cfg(debug_assertions)]
            {
                panic!("Nesting imbalance.");
            }

            #[cfg(not(debug_assertions))]
            {
                return NodeRef::nil();
            }
        };

        let Some(item) = self.nodes.get_mut(index) else {
            unsafe { debug_unreachable!("Bad context index.") }
        };

        if replace(item, Some(node)).is_some() {
            unsafe { debug_unreachable!("Bad context index.") }
        }

        NodeRef {
            id: self.id,
            entry: Entry { index, version: 0 },
        }
    }

    #[inline]
    fn lift(&mut self, node_ref: &NodeRef) {
        if self.id != node_ref.id {
            #[cfg(debug_assertions)]
            {
                panic!("Cannot lift a node that does not belong to this compilation session.");
            }

            #[cfg(not(debug_assertions))]
            {
                return;
            }
        }

        let parent_ref = self.node_ref();

        let Some(Some(node)) = self.nodes.get_mut(node_ref.entry.index) else {
            #[cfg(debug_assertions)]
            {
                panic!("Cannot lift a node that does not belong to this compilation session.");
            }

            #[cfg(not(debug_assertions))]
            {
                return;
            }
        };

        node.set_parent_ref(parent_ref);
    }

    #[inline(always)]
    fn node_ref(&self) -> NodeRef {
        let Some(index) = self.context.last() else {
            #[cfg(debug_assertions)]
            {
                panic!("Nesting imbalance.");
            }

            #[cfg(not(debug_assertions))]
            {
                return NodeRef::nil();
            }
        };

        NodeRef {
            id: self.id,
            entry: Entry {
                index: *index,
                version: 0,
            },
        }
    }

    #[inline(always)]
    fn parent_ref(&self) -> NodeRef {
        let Some(depth) = self.context.len().checked_sub(2) else {
            return NodeRef::nil();
        };

        let index = *unsafe { self.context.get_unchecked(depth) };

        NodeRef {
            id: self.id,
            entry: Entry { index, version: 0 },
        }
    }

    #[inline(always)]
    fn failure(&mut self, error: impl Into<<Self::Node as Node>::Error>) -> ErrorRef {
        if self.failing {
            return ErrorRef::nil();
        }

        self.observer.parse_error();

        self.failing = true;

        let index = self.errors.len();

        self.errors.push(error.into());

        return ErrorRef {
            id: self.id,
            entry: Entry { index, version: 0 },
        };
    }
}
