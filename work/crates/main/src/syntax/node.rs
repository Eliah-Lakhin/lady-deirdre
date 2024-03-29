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

extern crate lady_deirdre_derive;

pub use lady_deirdre_derive::Node;

use crate::{
    arena::{Id, Identifiable, Ref},
    lexis::{Token, TokenCursor},
    std::*,
    syntax::{ClusterRef, SyntaxBuffer, SyntaxError, SyntaxRule, SyntaxSession, SyntaxTree},
};

/// A trait that specifies syntax tree node kind and provides a syntax grammar parser.
///
/// An API user implements this trait to specify Programming Language syntax grammar and the
/// type of the syntax tree node.
///
/// This trait is supposed to be implemented on the Rust enum type with variants representing
/// tree node kinds, but this is not a strict requirement. From the functional sense the main
/// purpose of the Node implementation is to provide a syntax parser that will re-parse sequences of
/// [Tokens](crate::lexis::Token) by interacting with arbitrary
/// [SyntaxSession](crate::syntax::SyntaxSession) interface that, in turn, manages parsing process.
///
/// An API user is encouraged to implement this trait using helper
/// [Node](::lady_deirdre_derive::Node) macro-derive on enum types by specifying syntax
/// grammar directly on enum variants through the macros attributes.
///
/// ```rust
/// use lady_deirdre::{
///     syntax::{Node, SyntaxError, SyntaxTree},
///     lexis::{SimpleToken, TokenRef},
///     Document,
/// };
///
/// #[derive(Node, PartialEq, Debug)]
/// #[token(SimpleToken)]
/// #[error(SyntaxError)]
/// #[skip($Whitespace)]
/// enum NumbersInParens {
///     #[root]
///     #[rule($ParenOpen & (numbers: $Number)*{$Symbol} & $ParenClose)]
///     Root {
///         numbers: Vec<TokenRef>,
///     },
/// }
///
/// let doc = Document::<NumbersInParens>::from("(3, 4, 5)");
///
/// let root = doc.root().deref(&doc).unwrap();
///
/// match root {
///     NumbersInParens::Root { numbers } => {
///         assert_eq!(
///             numbers.iter().map(|num| num.string(&doc).unwrap()).collect::<Vec<_>>(),
///             vec!["3", "4", "5"],
///         );
///     },
/// }
/// ```
///
/// An API user can implement the Node trait manually too. For example, using 3rd party parser
/// libraries. See [`Node::new`](crate::syntax::Node::new) function specification for details.
pub trait Node: Sized + 'static {
    /// Describes programming language's lexical grammar.
    type Token: Token;

    /// Describes syntax/semantic error type of this programming language grammar.
    type Error: From<SyntaxError> + Sized + 'static;

    /// Parses a branch of the syntax tree from the sequence of [Tokens](crate::lexis::Token) using
    /// specified parse `rule`, and returns an instance of the top node of the branch.
    ///
    /// This is a low-level API function.
    ///
    /// An API user encouraged to use [Node](::lady_deirdre_derive::Node) macro-derive to
    /// implement this trait automatically based on a set of LL(1) grammar rules,
    /// but you can implement it manually too.
    ///
    /// You need to call this function manually only if you want to implement an extension API to
    /// this crate. In this case you should also prepare a custom implementation of the
    /// SyntaxSession trait. See [SyntaxSession](crate::syntax::SyntaxSession) documentation for
    /// details.
    ///
    /// **Algorithm Specification:**
    ///   - The Algorithm lay behind this implementation is a
    ///     [Top-down Parser](https://en.wikipedia.org/wiki/Top-down_parsing) that parses
    ///     a context-free language of [LL grammar class](https://en.wikipedia.org/wiki/LL_grammar)
    ///     with potentially unlimited lookahead. Note, that due to unlimited lookahead
    ///     characteristic it could be a wide class of recursive-descending grammars including
    ///     [PEG grammars](https://en.wikipedia.org/wiki/Parsing_expression_grammar).
    ///   - The Algorithm reads as many tokens from the input sequence as needed using `session`'s
    ///     [TokenCursor](crate::lexis::TokenCursor) lookahead operations to recognize
    ///     appropriate parse `rule`.
    ///   - The Algorithm [advances](crate::lexis::Tokens::advance) TokenCursor to as many tokens
    ///     as needed to exactly match parsed `rule`.
    ///   - To descend into a parsing subrule the Algorithm calls `session`'s
    ///     [`descend`](crate::syntax::SyntaxSession::descend) function that consumes subrule's
    ///     [kind](crate::syntax::SyntaxRule) and returns a [`weak reference`](NodeRef) into the
    ///     rule's parsed Node.
    ///   - The Algorithm never calls [`descend`](crate::syntax::SyntaxSession::descend) function
    ///     with [ROOT_RULE](crate::syntax::ROOT_RULE). The Root Rule is not a recursive rule
    ///     by design.
    ///   - The Specification does not limit the way the Algorithm maps `rule` values to
    ///     specific parsing function under the hood. This mapping is fully encapsulated by the
    ///     Algorithm internals. In other words the "external" caller of the function `new` does not
    ///     have to be aware of the mapping between the `rule` values and the types of produced
    ///     nodes. The only exception from this is a [ROOT_RULE](crate::syntax::ROOT_RULE)
    ///     value. If the "external" caller invokes `new` function with the ROOT_RULE parameter, the
    ///     Algorithm guaranteed to enter the entire syntax tree parsing procedure.
    ///   - When the function `new` invoked, the Algorithm guaranteed to complete parsing procedure
    ///     regardless of input sequence, and to return a valid instance of [Node]. If the input
    ///     sequence contains syntax errors, the Algorithm recovers these error in a way that is
    ///     not specified. In this case the Algorithm could call `session`'s
    ///     [error](crate::syntax::SyntaxSession::error) function to register syntax error.
    ///
    /// ```rust
    /// use lady_deirdre::{
    ///     syntax::{Node, NodeRef, SyntaxSession, SyntaxRule, SyntaxError, SyntaxTree, ROOT_RULE},
    ///     lexis::{SimpleToken, TokenCursor},
    ///     Document,
    /// };
    ///
    /// // A syntax of embedded parentheses: `(foo (bar) baz)`.
    /// enum Parens {
    ///    Root { inner: Vec<NodeRef> },
    ///    Parens { inner: Vec<NodeRef> },
    ///    Other,
    /// };
    ///  
    /// const PARENS_RULE: SyntaxRule = &1;
    /// const OTHER_RULE: SyntaxRule = &2;
    ///
    /// impl Node for Parens {
    ///     type Token = SimpleToken;
    ///     type Error = SyntaxError;
    ///
    ///     fn new<'code>(
    ///         rule: SyntaxRule,
    ///         session: &mut impl SyntaxSession<'code, Node = Self>,
    ///     ) -> Self {
    ///         // Rule dispatcher that delegates parsing control flow to specialized parse
    ///         // functions.
    ///
    ///         if rule == ROOT_RULE {
    ///             return Self::parse_root(session);
    ///         }
    ///
    ///         if rule == PARENS_RULE {
    ///             return Self::parse_parens(session);
    ///         }
    ///
    ///         // Otherwise the `rule` is an `OTHER_RULE`.
    ///
    ///         Self::parse_other(session)
    ///     }
    ///
    /// }
    ///
    /// impl Parens {
    ///     fn parse_root<'code>(session: &mut impl SyntaxSession<'code, Node = Self>) -> Self {
    ///         let mut inner = vec![];
    ///
    ///         loop {
    ///             // Analysing of the next incoming token.
    ///             match session.token(0) {
    ///                 Some(&SimpleToken::ParenOpen) => {
    ///                     inner.push(session.descend(PARENS_RULE));
    ///                 }
    ///
    ///                 Some(_) => {
    ///                     inner.push(session.descend(OTHER_RULE));
    ///                 }
    ///
    ///                 None => break,
    ///             }
    ///         }
    ///
    ///         Self::Root { inner }
    ///     }
    ///
    ///     // Parsing a pair of parenthesis(`(...)`).
    ///     fn parse_parens<'code>(session: &mut impl SyntaxSession<'code, Node = Self>) -> Self {
    ///         let mut inner = vec![];
    ///
    ///         // The first token is open parenthesis("("). Consuming it.
    ///         session.advance();
    ///
    ///         loop {
    ///             // Analysing of the next incoming token.
    ///             match session.token(0) {
    ///                 Some(&SimpleToken::ParenOpen) => {
    ///                     inner.push(session.descend(PARENS_RULE));
    ///                 }
    ///
    ///                 // Close parenthesis(")") found. Parsing process finished successfully.
    ///                 Some(&SimpleToken::ParenClose) => {
    ///                     // Consuming this token.
    ///                     session.advance();
    ///
    ///                     return Self::Parens { inner };
    ///                 }
    ///
    ///                 Some(_) => {
    ///                     inner.push(session.descend(OTHER_RULE));
    ///                 }
    ///
    ///                 None => break,
    ///             }
    ///         }
    ///
    ///         // Parse process has failed. We didn't find closing parenthesis.
    ///
    ///         // Registering a syntax error.
    ///         let span = session.site_ref(0)..session.site_ref(0);
    ///         session.error(SyntaxError::UnexpectedEndOfInput {
    ///             span,
    ///             context: "Parse Parens",
    ///         });
    ///
    ///         // Returning what we have parsed so far.
    ///         Self::Parens { inner }
    ///     }
    ///
    ///     // Parsing any sequence of tokens except parenthesis(`foo bar`).
    ///     fn parse_other<'code>(session: &mut impl SyntaxSession<'code, Node = Self>) -> Self {
    ///         // The first token is not a parenthesis token. Consuming it.
    ///         session.advance();
    ///
    ///         loop {
    ///             // Analysing of the next incoming token.
    ///             match session.token(0) {
    ///                 Some(&SimpleToken::ParenOpen) | Some(&SimpleToken::ParenClose) | None => {
    ///                     break;
    ///                 }
    ///
    ///                 Some(_) => {
    ///                     // The next token is not a parenthesis token. Consuming it.
    ///                     session.advance();
    ///                 },
    ///             }
    ///         }
    ///
    ///         Self::Other
    ///     }
    /// }
    ///
    /// let doc = Document::<Parens>::from("foo (bar (baz) (aaa) ) bbb");
    ///
    /// // The input text has been parsed without errors.
    /// assert_eq!(doc.errors().count(), 0);
    /// ```
    fn new<'code>(rule: SyntaxRule, session: &mut impl SyntaxSession<'code, Node = Self>) -> Self;

    /// A helper function to immediately parse a subsequent of tokens in non-incremental way.
    ///
    /// ```rust
    /// use lady_deirdre::{
    ///     lexis::{SimpleToken, Token, SourceCode},
    ///     syntax::{SimpleNode, Node, SyntaxTree},
    /// };
    ///
    /// let tokens = SimpleToken::parse("(foo bar)");
    ///
    /// let sub_sequence = tokens.cursor(0..5); // A cursor into the "(foo bar" substring.
    ///
    /// let syntax = SimpleNode::parse(sub_sequence);
    ///
    /// // Close parenthesis is missing in this subsequence, so the syntax tree of the subsequence
    /// // has syntax errors.
    /// assert!(syntax.errors().count() > 0);
    /// ```
    #[inline(always)]
    fn parse<'code>(cursor: impl TokenCursor<'code, Token = Self::Token>) -> SyntaxBuffer<Self> {
        SyntaxBuffer::new(cursor)
    }
}

/// A weak reference of the [Node] and its metadata inside the syntax structure of the compilation
/// unit.
///
/// This objects represents a long-lived lifetime independent and type independent cheap to
/// [Copy](::std::marker::Copy) safe weak reference into the syntax tree.
///
/// NodeRef is capable to survive source code incremental changes happening aside of the referred
/// Node.
///
/// An API user normally does not need to inspect NodeRef inner fields manually or to construct
/// a NodeRef manually unless you are working on the Crate API Extension.
///
/// For details on the Weak references framework design see [Arena](crate::arena) module
/// documentation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NodeRef {
    /// An [identifier](crate::arena::Id) of the [SyntaxTree](crate::syntax::SyntaxTree) instance
    /// this weakly referred [Node] belongs to.
    pub id: Id,

    /// An internal weak reference of the node's [Cluster](crate::syntax::Cluster) of the
    /// [SyntaxTree](crate::syntax::SyntaxTree) instance.
    pub cluster_ref: Ref,

    /// An internal weak reference of the Node object in the
    /// [Cluster](crate::syntax::Cluster).
    ///
    /// If `node_ref` is a [`Ref::Primary`](crate::arena::Ref::Primary) variant, the NodeRef object
    /// refers [`Cluster::primary`](crate::syntax::Cluster::primary) object. Otherwise `node_ref` is
    /// a [`Ref::Repository`] variant that refers an object from the
    /// [`Cluster::nodes`](crate::syntax::Cluster::nodes) repository.
    pub node_ref: Ref,
}

impl Debug for NodeRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self.is_nil() {
            false => formatter.write_fmt(format_args!("NodeRef({:?})", self.id())),
            true => formatter.write_str("NodeRef(Nil)"),
        }
    }
}

impl Identifiable for NodeRef {
    #[inline(always)]
    fn id(&self) -> &Id {
        &self.id
    }
}

impl NodeRef {
    /// Returns an invalid instance of the NodeRef.
    ///
    /// This instance never resolves to valid [Node].
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: *Id::nil(),
            cluster_ref: Ref::Nil,
            node_ref: Ref::Nil,
        }
    }

    /// Returns `true` if this instance will never resolve to valid [Node].
    ///
    /// It is guaranteed that `NodeRef::nil().is_nil()` is always `true`, but in general if
    /// this function returns `false` it is not guaranteed that provided instance is a valid
    /// reference.
    ///
    /// To determine reference validity per specified [SyntaxTree](crate::syntax::SyntaxTree)
    /// instance use [is_valid_ref](NodeRef::is_valid_ref) function instead.
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.id.is_nil() || self.cluster_ref.is_nil() || self.node_ref.is_nil()
    }

    /// Immutably dereferences weakly referred [Node] of specified
    /// [SyntaxTree](crate::syntax::SyntaxTree).
    ///
    /// Returns [None] if this NodeRef is not valid reference for specified `tree` instance.
    ///
    /// Use [is_valid_ref](NodeRef::is_valid_ref) to check NodeRef validity.
    ///
    /// This function uses [`SyntaxTree::get_cluster`](crate::syntax::SyntaxTree::get_cluster)
    /// function under the hood.
    #[inline(always)]
    pub fn deref<'tree, N: Node>(
        &self,
        tree: &'tree impl SyntaxTree<Node = N>,
    ) -> Option<&'tree N> {
        if &self.id != tree.id() {
            return None;
        }

        match tree.get_cluster(&self.cluster_ref) {
            Some(cluster) => match &self.node_ref {
                Ref::Primary => Some(&cluster.primary),

                _ => cluster.nodes.get(&self.node_ref),
            },

            _ => None,
        }
    }

    /// Mutably dereferences weakly referred [Node] of specified
    /// [SyntaxTree](crate::syntax::SyntaxTree).
    ///
    /// Returns [None] if this NodeRef is not valid reference for specified `tree` instance.
    ///
    /// Use [is_valid_ref](NodeRef::is_valid_ref) to check NodeRef validity.
    ///
    /// This function uses
    /// [`SyntaxTree::get_cluster_mut`](crate::syntax::SyntaxTree::get_cluster_mut) function under
    /// the hood.
    #[inline(always)]
    pub fn deref_mut<'tree, N: Node>(
        &self,
        tree: &'tree mut impl SyntaxTree<Node = N>,
    ) -> Option<&'tree mut N> {
        if &self.id != tree.id() {
            return None;
        }

        match tree.get_cluster_mut(&self.cluster_ref) {
            None => None,
            Some(data) => match &self.node_ref {
                Ref::Primary => Some(&mut data.primary),

                _ => data.nodes.get_mut(&self.node_ref),
            },
        }
    }

    /// Creates a weak reference of the [Cluster](crate::syntax::Cluster) of referred [Node].
    #[inline(always)]
    pub fn cluster(&self) -> ClusterRef {
        ClusterRef {
            id: self.id,
            cluster_ref: self.cluster_ref,
        }
    }

    /// Removes an instance of the [Node] from the [SyntaxTree](crate::syntax::SyntaxTree)
    /// that is weakly referred by this reference.
    ///
    /// Returns [Some] value of the Node if this weak reference is a valid reference of
    /// existing node inside `tree` instance. Otherwise returns [None].
    ///
    /// Use [is_valid_ref](NodeRef::is_valid_ref) to check NodeRef validity.
    ///
    /// This function uses
    /// [`SyntaxTree::get_cluster_mut`](crate::syntax::SyntaxTree::get_cluster_mut) function under
    /// the hood.
    #[inline(always)]
    pub fn unlink<N: Node>(&self, tree: &mut impl SyntaxTree<Node = N>) -> Option<N> {
        if &self.id != tree.id() {
            return None;
        }

        match tree.get_cluster_mut(&self.cluster_ref) {
            None => None,
            Some(data) => data.nodes.remove(&self.node_ref),
        }
    }

    /// Returns `true` if and only if weakly referred Node belongs to specified
    /// [SyntaxTree](crate::syntax::SyntaxTree), and referred Node exists in this SyntaxTree
    /// instance.
    ///
    /// If this function returns `true`, all dereference function would return meaningful [Some]
    /// values, otherwise these functions return [None].
    ///
    /// This function uses [`SyntaxTree::get_cluster`](crate::syntax::SyntaxTree::get_cluster)
    /// function under the hood.
    #[inline(always)]
    pub fn is_valid_ref(&self, tree: &impl SyntaxTree) -> bool {
        if &self.id != tree.id() {
            return false;
        }

        match tree.get_cluster(&self.cluster_ref) {
            None => false,
            Some(cluster) => cluster.nodes.contains(&self.node_ref),
        }
    }
}
