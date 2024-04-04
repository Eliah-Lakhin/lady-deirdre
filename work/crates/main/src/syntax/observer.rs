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
    lexis::{Length, Site, SiteRef, Token, TokenCount, TokenCursor, TokenRef},
    report::{debug_unreachable, system_panic},
    std::*,
    syntax::{ErrorRef, Node, NodeRef, NodeRule, SyntaxSession, SyntaxTree, ROOT_RULE},
};

pub trait Observer {
    type Node: Node;

    fn read_token(&mut self, token: <Self::Node as Node>::Token, token_ref: TokenRef);

    fn enter_rule(&mut self, rule: NodeRule, node_ref: NodeRef);

    fn leave_rule(&mut self, rule: NodeRule, node_ref: NodeRef);

    fn lift_node(&mut self, node_ref: NodeRef);

    fn parse_error(&mut self, error_ref: ErrorRef);
}

pub struct DebugObserver<N: Node> {
    depth: usize,
    _phantom: PhantomData<N>,
}

impl<N: Node> Default for DebugObserver<N> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            depth: 0,
            _phantom: PhantomData,
        }
    }
}

impl<N: Node> Observer for DebugObserver<N> {
    type Node = N;

    fn read_token(&mut self, token: <Self::Node as Node>::Token, _token_ref: TokenRef) {
        let indent = self.indent();
        let name = token.name().unwrap_or("?");

        println!("{indent} ${name}");
    }

    fn enter_rule(&mut self, rule: NodeRule, _node_ref: NodeRef) {
        let indent = self.indent();
        let name = N::rule_name(rule).unwrap_or("?");

        println!("{indent} {name} {{");

        self.depth += 1;
    }

    fn leave_rule(&mut self, rule: NodeRule, _node_ref: NodeRef) {
        self.depth = self.depth.checked_sub(1).unwrap_or_default();

        let indent = self.indent();
        let name = N::rule_name(rule).unwrap_or("?");

        println!("{indent} }} {name}");
    }

    fn lift_node(&mut self, _node_ref: NodeRef) {
        let indent = self.indent();
        println!("{indent} --- lift ---",);
    }

    fn parse_error(&mut self, _error_ref: ErrorRef) {
        let indent = self.indent();
        println!("{indent} --- error ---",);
    }
}

impl<N: Node> DebugObserver<N> {
    #[inline(always)]
    fn indent(&self) -> String {
        "    ".repeat(self.depth)
    }
}

#[repr(transparent)]
pub struct VoidObserver<N: Node>(PhantomData<N>);

impl<N: Node> Default for VoidObserver<N> {
    #[inline(always)]
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<N: Node> Observer for VoidObserver<N> {
    type Node = N;

    #[inline(always)]
    fn read_token(&mut self, _token: <Self::Node as Node>::Token, _token_ref: TokenRef) {}

    #[inline(always)]
    fn enter_rule(&mut self, _rule: NodeRule, _node_ref: NodeRef) {}

    #[inline(always)]
    fn leave_rule(&mut self, _rule: NodeRule, _node_ref: NodeRef) {}

    #[inline(always)]
    fn lift_node(&mut self, _node_ref: NodeRef) {}

    #[inline(always)]
    fn parse_error(&mut self, _error_ref: ErrorRef) {}
}

pub(super) struct ObservableSyntaxSession<
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

impl<'code, 'observer, N, C, O> Identifiable for ObservableSyntaxSession<'code, 'observer, N, C, O>
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
    for ObservableSyntaxSession<'code, 'observer, N, C, O>
where
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
    O: Observer<Node = N>,
{
    type Token = <N as Node>::Token;

    #[inline(always)]
    fn advance(&mut self) -> bool {
        let token = self.token(0);
        let token_ref = self.token_ref(0);
        self.observer.read_token(token, token_ref);

        let advanced = self.token_cursor.advance();

        self.failing = self.failing && !advanced;

        advanced
    }

    #[inline(always)]
    fn skip(&mut self, mut distance: TokenCount) {
        let start = self.token_cursor.site(0);

        while distance > 0 {
            if !self.advance() {
                break;
            }

            distance -= 1;
        }

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
    for ObservableSyntaxSession<'code, 'observer, N, C, O>
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
        let index = self.nodes.len();

        self.nodes.push(None);

        self.context.push(index);

        let node_ref = NodeRef {
            id: self.id,
            entry: Entry { index, version: 0 },
        };

        self.observer.enter_rule(rule, node_ref);

        node_ref
    }

    #[inline]
    fn leave(&mut self, node: Self::Node) -> NodeRef {
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

        let rule = node.rule();

        if replace(item, Some(node)).is_some() {
            unsafe { debug_unreachable!("Bad context index.") }
        }

        let node_ref = NodeRef {
            id: self.id,
            entry: Entry { index, version: 0 },
        };

        self.observer.leave_rule(rule, node_ref);

        node_ref
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

        self.observer.lift_node(*node_ref);
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

        self.failing = true;

        let index = self.errors.len();

        self.errors.push(error.into());

        let error_ref = ErrorRef {
            id: self.id,
            entry: Entry { index, version: 0 },
        };

        self.observer.parse_error(error_ref);

        error_ref
    }
}
