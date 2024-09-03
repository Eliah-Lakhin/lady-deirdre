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

use std::{marker::PhantomData, mem::replace};

use crate::{
    arena::{Entry, EntryIndex, Id, Identifiable},
    lexis::{Length, Site, SiteRef, TokenCount, TokenCursor, TokenRef},
    report::ld_unreachable,
    syntax::{ErrorRef, Node, NodeRef, NodeRule, SyntaxError},
};

/// A communication channel of the syntax tree parsing process.
///
/// Lady Deirdre distinguishes two independent sides of the syntax parsing process:
///
/// - The parsing environment side (e.g., [Document](crate::units::Document))
///   that manages the parsing input (tokens), and manages the parsing output
///   (syntax tree).
///
/// - The parsing algorithm implementation side (via the [Node::parse] function).
///
/// Both sides unaware of each other, and they use SyntaxSession
/// for intercommunication.
///
///  1. Whenever the parsing environment side wants to parse a particular node
///     of the syntax tree, it gives control flow to the parser by providing
///     it's own implementation of the SyntaxSession to the [Node::parse]
///     function.
///
///  2. The parse function reads particular tokens of the token input stream
///     and advances the token cursor using SyntaxSession's functions.
///
///  3. If the parse algorithm needs to further descend into the syntax tree,
///     it calls [SyntaxSession::descend] function returning control flow back
///     to the parsing environment side.
///
///  4. The parsing environment side decides whether to give control flow to
///     the Node's parse function again, continuing recursive descending,
///     or returning a result from its own cache.
///
/// Since both sides of the parsing process are isolated from each other,
/// it opens up a lot of implementation configurations.
///
/// ## As the user of the SyntaxSession instances, you can
///
///   - Implement a custom parser that inherently suit the needs
///     of both one-time parsing of the entire file and the incremental
///     reparsing process.
///
///   - Adopt a 3rd party parsing library to Lady Deirdre by adopting
///     its interface to the SyntaxSession interface.
///
///   - Implement a parsing algorithm of almost any programming
///     language as long as its grammar does not surpass a class of
///     the context free grammars with unlimited lookahead.
///
/// In particular, the [Node derive macro](lady_deirdre_derive::Node)
/// implements Node's parse function for LL(1) grammars, but you are free to
/// implement your own parsers and alternative recursive-descending parsing
/// libraries utilizing the SyntaxSession interface.
///
/// ## As the author of the SyntaxSession implementations
///
/// You have control over arbitrary programming language parser steps
/// without the need to know the details of the language's grammar.
///
/// Therefore, you can implement a wide set of the parsing environments with
/// different capabilities.
///
/// In particular, Lady Deirdre provides the following parsing environments
/// through custom implementations of the SyntaxSession trait:
///
///  - The immutable [Document](crate::units::Document) and
///    the [ImmutableSyntaxTree](crate::syntax::ImmutableSyntaxTree) objects
///    provide one-time parsing capabilities, always returning control-flow
///    back to the Node's parse function when the function is trying to descend.
///    The performance characteristics of this approach are close to ordinary
///    non-incremental parsers.
///
///  - The mutable Document and the [MutableUnit](crate::units::MutableUnit)
///    have their own reusable caches of the syntax tree nodes that they utilize
///    during incremental reparsing by returning previously parsed node cache
///    whenever possible when the Node's parse function is trying to descend.
///
///  - [Node::debug] function under the hood uses special implementation of the
///    SyntaxSession trait that does not store any nodes in the syntax tree
///    but instead just prints parser's actions to the terminal for debugging
///    purposes.
///
///  - [ParseTree](crate::syntax::ParseTree) tracks parser's interactions
///    with the SyntaxSession to reconstruct concrete parsing trees.
///
/// ## Parsing algorithm considerations
///
/// When implementing the [Node::parse] function, several considerations should
/// be taken into account:
///
///  1. The parsing algorithm should handle a **context-free grammar**.
///     Each rule of the grammar should be parseable from any **arbitrary**
///     token sequence, and the algorithm should not rely on parsing rules
///     applied previously.
///
///  2. The [Node::parse] function must be infallible regardless of the input
///     token sequence or the `rule` parameter. If the token sequence provided
///     to the parser by the SyntaxSession does not fit the parsing rule at any
///     step (including the first step) of the rule’s algorithm, **the algorithm
///     must attempt to recover from syntax errors** by skipping or ignoring
///     parts of the input tokens and by reporting syntax errors using the
///     [SyntaxSession::failure] function.
///
///  3. In the end, the [Node::parse] function must advance the SyntaxSession
///     token cursor by consuming **at least one token** from the input
///     sequence, and it must return an instance of the [Node] that corresponds
///     to the requested rule.
///
/// ## Parse rules
///
/// Both [Node::parse] and the [SyntaxSession::descend] functions have a
/// `rule` parameter of the [NodeRule] type, which is an arbitrary number used
/// to distinguish between concrete parsing rules.
///
/// In the context of parsing rules:
///
///  1. Rule `0` ([ROOT_RULE](crate::syntax::ROOT_RULE)), denoting the root node
///     of the syntax tree, should be used only once per the entire parsing
///     process, as there can only be one root in the syntax tree.
///     Therefore, the [Node::parse] function itself **should never call**
///     [descend](SyntaxSession::descend) or [enter](SyntaxSession::enter)
///     functions with the rule `0`.
///
///  2. Rule `u16::MAX` ([NON_RULE](crate::syntax::NON_RULE)) is a reserved rule
///     that denotes an invalid rule, indicating a rule that does not parse
///     anything. Therefore, it should never be supplied to either
///     the [Node::parse] function or any of the SyntaxSession functions that
///     accept a NodeRule type.
///
///  3. Any other rule number in a range of `1..u16::MAX` is a valid rule
///     uniquely denotes a parsing rule of a particular syntax tree node.
///
///  4. The SyntaxSession trait is unaware of the mapping between these numbers
///     and the node types upfront. They are determined by each implementer of
///     the programming language parsers. However, the SyntaxSession can rely on
///     the fact that **passing the same rule number to the [Node::parse]
///     function would produce a node of the same type**.
///
/// ## Cache control
///
/// The parser algorithm typically descends into the sub-rules of the currently
/// parsed rule by calling the [SyntaxSession::descend] function, which gives
/// control flow to the SyntaxSession and leaves the decision about the node’s
/// caching to the SyntaxSession implementation.
///
/// The nodes computed this way are called _primary nodes_.
///
/// Alternatively, the algorithm could compute a sub-rule in place, without
/// returning control flow back to the SyntaxSession.
///
/// These nodes are called _secondary nodes_. As the secondary nodes
/// are computed in places, they cannot be cached by the SyntaxSession.
///
/// Whenever the parser starts in-place parsing of the secondary node it should
/// call the [SyntaxSession::enter] function specifying the node's parsing rule.
///
/// When the the parsing finishes, the parser should call the
/// [SyntaxSession::leave] function specifying an instance of the [Node] as
/// a product of this sub-rule.
///
/// ## Nodes and rules nesting
///
/// The SyntaxSession tracks rules and their products ([Nodes](Node)) nesting.
///
/// It is the parser's implementor responsibility to balance the
/// [enter](SyntaxSession::enter) and [leave](SyntaxSession::leave) functions
/// properly when parsing the secondary nodes in place, always leaving the nodes
/// entered before.
///
/// When the algorithm uses the [descend](SyntaxSession::descend) function,
/// the nesting process is controlled by the SyntaxSession implementation.
///
/// [SyntaxSession::node_ref] returns a [NodeRef] of the node currently being
/// parsed. Using this function, you can fetch the node's reference while it is
/// being parsed.
///
/// [SyntaxSession::parent_ref] returns a NodeRef reference of the parent rule's
/// node.
///
/// The parsing algorithm cannot access either of the syntax tree node
/// instances during the parsing process, but it can use these NodeRef
/// references to set up the [parent_ref](crate::syntax::AbstractNode::parent_ref)
/// and the [node_ref](crate::syntax::AbstractNode::node_ref) values of
/// the resulting node instance.
///
/// These values of the Node instance establish back references from the child
/// nodes to their parents and are useful for the resulting syntax tree
/// ascending traverse.
///
/// ## Left recursion
///
/// To handle left recursion, the parser could utilize either lookahead
/// capabilities of the SyntaxSession or to use the node lifting feature.
///
/// For example, to parse infix expressions such as `a + b`, `a * b`, or
/// just `a`, where the binary operator is not known upfront or could absent,
/// you can descend into the operand parsing rule first, receiving its
/// [NodeRef] reference.
///
/// Then, if the parser encounters an operator token, you can
/// [enter](SyntaxSession::enter) the corresponding binary operation rule and
/// immediately call the [lift](SyntaxSession::lift) function, providing
/// the operand's NodeRef. Then, parse the rest of the expression normally.
///
/// The lift function would "transplant" the operand's node parsed outside of
/// the operator's rule to the context of the operator's rule, rearranging
/// operand's node nesting.
///
/// ## Tokens access
///
/// The SyntaxSession trait is a super-trait of the [TokenCursor] trait.
///
/// Functions of the TokenCursor grant access to the current state of the token
/// sequence being parsed.
///
/// The [TokenCursor::advance] and the [TokenCursor::skip] functions advance
/// the SyntaxSession parsing cursor, consuming corresponding tokens.
///
/// The access functions of the TokenCursor grant potentially unlimited
/// lookahead capabilities.
///
/// The SyntaxSession could track the lookahead distance used by the parsing
/// algorithm. The lookahead distance could limit the underlying parsing
/// environment caching capabilities.
///
/// Therefore, the parser's implementor **should prefer to limit the lookahead**
/// whenever possible.
///
/// ## Safety and Panic
///
/// The SyntaxSession trait and all of it's functions **are safe**.
///
/// Violations of the above specification **are not** undefined behavior, but
/// the failure to follow the specified contract could lead to bugs and panics
/// depending on the implementation.
pub trait SyntaxSession<'code>: TokenCursor<'code, Token = <Self::Node as Node>::Token> {
    /// Specifies a type of the Node that is currently being parsed.
    type Node: Node;

    /// Instructs the parsing environment to parse the parsing rule denoted by
    /// the `rule` parameter starting from the current token.
    ///
    /// The valid values of the `rule` are any values within the [NodeRule]
    /// range except the [ROOT_RULE](crate::syntax::ROOT_RULE) and
    /// the [NON_RULE](crate::syntax::NON_RULE).
    ///
    /// Returns a [NodeRef] reference of the node inside the
    /// [SyntaxTree](crate::syntax::SyntaxTree) that would be parsed by this
    /// rule and consumes the sequence of tokens required to apply the rule.
    fn descend(&mut self, rule: NodeRule) -> NodeRef;

    /// Begins parsing of the parsing rule denoted by the `rule` parameter
    /// from the current token.
    ///
    /// The valid values of the `rule` are any values within the [NodeRule]
    /// range except the [ROOT_RULE](crate::syntax::ROOT_RULE) and
    /// the [NON_RULE](crate::syntax::NON_RULE).
    ///
    /// Returns a [NodeRef] reference of the node inside the
    /// [SyntaxTree](crate::syntax::SyntaxTree) that will be placed to
    /// the syntax tree when the rule parsing finishes.
    ///
    /// Each enter function must be paired with
    /// the [leave](SyntaxSession::leave) function that finises the rule.
    ///
    /// After entering into the rule parser, the parsing algorithm must consume
    /// at least one token directly or indirectly by entering another sub-rule.
    fn enter(&mut self, rule: NodeRule) -> NodeRef;

    /// Completes parsing of the rule started previously by
    /// the [enter](SyntaxSession::enter) function.
    ///
    /// The `node` parameter specifies rule's parsing result.
    ///
    /// Returns a [NodeRef] reference of the `node` inside
    /// the [SyntaxTree](crate::syntax::SyntaxTree).
    fn leave(&mut self, node: Self::Node) -> NodeRef;

    /// Reinterprets the previously parsed sibling node of the current node
    /// as the current node's child.
    ///
    /// See the [Left recursion](SyntaxSession#left-recursion) section of
    /// the parsing process specification for details.
    fn lift(&mut self, node_ref: &NodeRef);

    /// Returns the [NodeRef] reference of the node inside
    /// the [SyntaxTree](crate::syntax::SyntaxTree) being parsed by the current
    /// parsing rule.
    fn node_ref(&self) -> NodeRef;

    /// Returns the [NodeRef] reference of the node inside
    /// the [SyntaxTree](crate::syntax::SyntaxTree) being parsed by the parental
    /// parsing rule.
    ///
    /// If the current rule is the root, this function returns [NodeRef::nil].
    fn parent_ref(&self) -> NodeRef;

    /// Reports a syntax error occur during the syntax recovery.
    ///
    /// Returns an [ErrorRef] reference of the error object inside
    /// the [SyntaxTree](crate::syntax::SyntaxTree).
    ///
    /// The SyntaxSession implementation may decide to ignore the provided error
    /// object. In this case, the failure function returns [ErrorRef::nil].
    fn failure(&mut self, error: SyntaxError) -> ErrorRef;
}

pub(super) struct ImmutableSyntaxSession<
    'code,
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
> {
    pub(super) id: Id,
    pub(super) context: Vec<EntryIndex>,
    pub(super) nodes: Vec<Option<N>>,
    pub(super) errors: Vec<SyntaxError>,
    pub(super) failing: bool,
    pub(super) token_cursor: C,
    pub(super) _phantom: PhantomData<&'code ()>,
}

impl<'code, N, C> Identifiable for ImmutableSyntaxSession<'code, N, C>
where
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
{
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'code, N, C> TokenCursor<'code> for ImmutableSyntaxSession<'code, N, C>
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

impl<'code, N, C> SyntaxSession<'code> for ImmutableSyntaxSession<'code, N, C>
where
    N: Node,
    C: TokenCursor<'code, Token = <N as Node>::Token>,
{
    type Node = N;

    fn descend(&mut self, rule: NodeRule) -> NodeRef {
        let _ = self.enter(rule);

        let node = N::parse(self, rule);

        self.leave(node)
    }

    #[inline]
    fn enter(&mut self, _rule: NodeRule) -> NodeRef {
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
            unsafe { ld_unreachable!("Bad context index.") }
        };

        if replace(item, Some(node)).is_some() {
            unsafe { ld_unreachable!("Bad context index.") }
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
    fn failure(&mut self, error: SyntaxError) -> ErrorRef {
        if self.failing {
            return ErrorRef::nil();
        }

        self.failing = true;

        let index = self.errors.len();

        self.errors.push(error);

        ErrorRef {
            id: self.id,
            entry: Entry { index, version: 0 },
        }
    }
}
