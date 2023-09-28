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
    arena::{Entry, EntryIndex, Id, Identifiable, Repository},
    lexis::{Length, Site, SiteRef, TokenCount, TokenCursor, TokenRef},
    std::*,
    syntax::{ErrorRef, Node, NodeRef, NodeRule, ROOT_RULE},
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

    fn enter_node(&mut self) -> NodeRef;

    fn leave_node(&mut self, node: Self::Node) -> NodeRef;

    fn node(&mut self, node: Self::Node) -> NodeRef;

    fn lift_sibling(&mut self, sibling_ref: &NodeRef);

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

pub(super) struct SequentialSyntaxSession<
    'code,
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
> {
    pub(super) id: Id,
    pub(super) context: Vec<EntryIndex>,
    pub(super) primary: Option<N>,
    pub(super) nodes: Repository<N>,
    pub(super) errors: Repository<N::Error>,
    pub(super) failing: bool,
    pub(super) token_cursor: C,
    pub(super) _code_lifetime: PhantomData<&'code ()>,
}

impl<'code, N, C> Identifiable for SequentialSyntaxSession<'code, N, C>
where
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
{
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'code, N, C> TokenCursor<'code> for SequentialSyntaxSession<'code, N, C>
where
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
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

impl<'code, N, C> SyntaxSession<'code> for SequentialSyntaxSession<'code, N, C>
where
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
{
    type Node = N;

    fn descend(&mut self, rule: NodeRule) -> NodeRef {
        let index = self.nodes.reserve();

        self.context.push(index);

        let node = N::parse(self, rule);

        #[allow(unused)]
        let last = self.context.pop();

        #[cfg(debug_assertions)]
        if last != Some(index) {
            panic!("Inheritance imbalance.");
        }

        unsafe { self.nodes.set_unchecked(index, node) };

        NodeRef {
            id: self.id,
            cluster_entry: Entry::Primary,
            node_entry: unsafe { self.nodes.entry_of(index) },
        }
    }

    #[inline]
    fn enter_node(&mut self) -> NodeRef {
        let index = self.nodes.reserve();

        self.context.push(index);

        NodeRef {
            id: self.id,
            cluster_entry: Entry::Primary,
            node_entry: unsafe { self.nodes.entry_of(index) },
        }
    }

    #[inline]
    fn leave_node(&mut self, node: Self::Node) -> NodeRef {
        let index = match self.context.pop() {
            None => panic!("Inheritance imbalance."),
            Some(index) => index,
        };

        unsafe { self.nodes.set_unchecked(index, node) };

        NodeRef {
            id: self.id,
            cluster_entry: Entry::Primary,
            node_entry: unsafe { self.nodes.entry_of(index) },
        }
    }

    #[inline(always)]
    fn node(&mut self, node: Self::Node) -> NodeRef {
        NodeRef {
            id: self.id,
            cluster_entry: Entry::Primary,
            node_entry: self.nodes.insert(node),
        }
    }

    #[inline]
    fn lift_sibling(&mut self, sibling_ref: &NodeRef) {
        #[cfg(debug_assertions)]
        if sibling_ref.id != self.id {
            panic!("An attempt to lift external Node.");
        }

        #[cfg(debug_assertions)]
        if sibling_ref.cluster_entry != Entry::Primary {
            panic!("An attempt to lift non-sibling Node.");
        }

        let node_ref = self.node_ref();

        if let Some(node) = self.nodes.get_mut(&sibling_ref.node_entry) {
            node.set_parent_ref(node_ref);
            return;
        }

        panic!("An attempt to lift non-sibling Node.");
    }

    #[inline(always)]
    fn node_ref(&self) -> NodeRef {
        match self.context.last() {
            None => NodeRef {
                id: self.id,
                cluster_entry: Entry::Primary,
                node_entry: Entry::Primary,
            },

            Some(index) => NodeRef {
                id: self.id,
                cluster_entry: Entry::Primary,
                node_entry: unsafe { self.nodes.entry_of(*index) },
            },
        }
    }

    #[inline(always)]
    fn parent_ref(&self) -> NodeRef {
        match self.context.len() {
            0 => NodeRef::nil(),
            1 => NodeRef {
                id: self.id,
                cluster_entry: Entry::Primary,
                node_entry: Entry::Primary,
            },
            _ => {
                let index = *unsafe { self.context.get_unchecked(self.context.len() - 2) };

                NodeRef {
                    id: self.id,
                    cluster_entry: Entry::Primary,
                    node_entry: unsafe { self.nodes.entry_of(index) },
                }
            }
        }
    }

    #[inline(always)]
    fn failure(&mut self, error: impl Into<<Self::Node as Node>::Error>) -> ErrorRef {
        if !self.failing {
            self.failing = true;

            return ErrorRef {
                id: self.id,
                cluster_entry: Entry::Primary,
                error_entry: self.errors.insert(error.into()),
            };
        }

        return ErrorRef::nil();
    }
}

impl<'code, N, C> SequentialSyntaxSession<'code, N, C>
where
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
{
    pub(super) fn enter_root(&mut self) {
        let node = N::parse(self, ROOT_RULE);

        #[cfg(debug_assertions)]
        if !self.context.is_empty() {
            panic!("Inheritance imbalance.");
        }

        self.primary = Some(node);
    }
}
