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
    arena::{Entry, Id, Identifiable},
    lexis::{Site, SiteSpan, SourceCode, Token, TokenBuffer, TokenRef, NIL_TOKEN_REF},
    std::*,
    syntax::{
        Capture,
        CapturesIter,
        ChildrenIter,
        DebugObserver,
        ImmutableSyntaxTree,
        Key,
        NodeRule,
        ParseError,
        PolyRef,
        PolyVariant,
        RefKind,
        SyntaxSession,
        SyntaxTree,
        NON_RULE,
    },
    units::CompilationUnit,
};

pub static NIL_NODE_REF: NodeRef = NodeRef::nil();

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
///     syntax::{Node, ParseError, SyntaxTree},
///     lexis::{SimpleToken, TokenRef},
///     units::Document,
/// };
///
/// #[derive(Node, PartialEq, Debug)]
/// #[token(SimpleToken)]
/// #[error(ParseError)]
/// #[trivia($Whitespace)]
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
/// let root = doc.root_node_ref().deref(&doc).unwrap();
///
/// match root {
///     NumbersInParens::Root { numbers } => {
///         assert_eq!(
///             numbers.iter().map(|num| num.string(&doc).unwrap()).collect::<Vec<_>>(),
///             vec!["3", "4", "5"],
///         );
///     }
/// }
/// ```
///
/// An API user can implement the Node trait manually too. For example, using 3rd party parser
/// libraries. See [`Node::new`](crate::syntax::Node::parse) function specification for details.
pub trait Node: AbstractNode + Sized {
    /// Describes programming language's lexical grammar.
    type Token: Token;

    /// Describes syntax/semantic error type of this programming language grammar.
    type Error: From<ParseError> + Send + Sync + Sized + 'static;

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
    ///     [kind](crate::syntax::NodeRule) and returns a [`weak reference`](NodeRef) into the
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
    ///     [error](crate::syntax::SyntaxSession::failure) function to register syntax error.
    ///
    /// ```rust
    /// use lady_deirdre::{
    ///     syntax::{
    ///         Node,
    ///         NodeRef,
    ///         SyntaxSession,
    ///         NodeRule,
    ///         ParseError,
    ///         SyntaxTree,
    ///         NodeSet,
    ///         ROOT_RULE,
    ///         EMPTY_NODE_SET,
    ///         RecoveryResult,
    ///     },
    ///     lexis::{SimpleToken, TokenCursor, TokenSet, EMPTY_TOKEN_SET},
    ///     units::Document,
    ///     analysis::{FeatureInitializer, FeatureInvalidator},
    ///     sync::SyncBuildHasher,
    /// };
    /// use lady_deirdre::syntax::{AbstractNode, Capture, Key};
    ///
    /// // A syntax of embedded parentheses: `(foo (bar) baz)`.
    /// enum Parens {
    ///    Root { inner: Vec<NodeRef> },
    ///    Parens { inner: Vec<NodeRef> },
    ///    Other,
    /// };
    ///  
    /// const PARENS_RULE: NodeRule = 1;
    /// const OTHER_RULE: NodeRule = 2;
    ///
    /// impl AbstractNode for Parens {
    ///     fn rule(&self) -> NodeRule {
    ///         match self {
    ///             Self::Root {..} => ROOT_RULE,
    ///             Self::Parens {..} => PARENS_RULE,
    ///             Self::Other {..} => OTHER_RULE,
    ///         }
    ///     }
    ///
    ///     fn name(&self) -> Option<&'static str> {
    ///         Self::rule_name(self.rule())
    ///     }
    ///
    ///     fn describe(&self, verbose: bool) -> Option<&'static str> {
    ///         Self::rule_description(self.rule(), verbose)
    ///     }
    ///
    ///     fn node_ref(&self) -> NodeRef {
    ///         NodeRef::nil()
    ///     }
    ///
    ///     fn parent_ref(&self) -> NodeRef {
    ///         NodeRef::nil()
    ///     }
    ///
    ///     fn set_parent_ref(&mut self, parent_ref: NodeRef) {}
    ///
    ///     fn capture(&self, key: Key) -> Option<Capture> {
    ///         None
    ///     }
    ///
    ///     fn capture_keys(&self) -> &'static [Key<'static>] {
    ///         &[]
    ///     }
    ///
    ///     fn rule_name(rule: NodeRule) -> Option<&'static str> where Self: Sized {
    ///         match rule {
    ///             PARENS_RULE => Some("Parens"),
    ///             OTHER_RULE => Some("Other"),
    ///             _ => None,
    ///         }
    ///     }
    ///
    ///     fn rule_description(rule: NodeRule, verbose: bool) -> Option<&'static str> where Self: Sized {
    ///         match rule {
    ///             PARENS_RULE => Some("Parens"),
    ///             OTHER_RULE => Some("Other"),
    ///             _ => None,
    ///         }
    ///     }
    /// }
    ///
    /// impl Node for Parens {
    ///     type Token = SimpleToken;
    ///     type Error = ParseError;
    ///
    ///     fn parse<'code>(
    ///         session: &mut impl SyntaxSession<'code, Node = Self>,
    ///         rule: NodeRule,
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
    /// }
    ///
    /// impl Parens {
    ///     fn parse_root<'code>(session: &mut impl SyntaxSession<'code, Node = Self>) -> Self {
    ///         let mut inner = vec![];
    ///
    ///         loop {
    ///             // Analysing of the next incoming token.
    ///             match session.token(0) {
    ///                 SimpleToken::ParenOpen => {
    ///                     inner.push(session.descend(PARENS_RULE));
    ///                 }
    ///
    ///                 SimpleToken::EOI => break,
    ///
    ///                 _ => {
    ///                     inner.push(session.descend(OTHER_RULE));
    ///                 }
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
    ///                 SimpleToken::ParenOpen => {
    ///                     inner.push(session.descend(PARENS_RULE));
    ///                 }
    ///
    ///                 // Close parenthesis(")") found. Parsing process finished successfully.
    ///                 SimpleToken::ParenClose => {
    ///                     // Consuming this token.
    ///                     session.advance();
    ///
    ///                     return Self::Parens { inner };
    ///                 }
    ///
    ///                 SimpleToken::EOI => break,
    ///
    ///                 _ => {
    ///                     inner.push(session.descend(OTHER_RULE));
    ///                 }
    ///             }
    ///         }
    ///
    ///         // Parse process has failed. We didn't find closing parenthesis.
    ///
    ///         // Registering a syntax error.
    ///         let span = session.site_ref(0)..session.site_ref(0);
    ///         session.failure(ParseError {
    ///             span,
    ///             context: PARENS_RULE,
    ///             recovery: RecoveryResult::UnexpectedEOI,
    ///             expected_tokens: &EMPTY_TOKEN_SET,
    ///             expected_nodes: &EMPTY_NODE_SET,
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
    ///                 SimpleToken::ParenOpen | SimpleToken::ParenClose | SimpleToken::EOI => {
    ///                     break;
    ///                 }
    ///
    ///                 _ => {
    ///                     // The next token is not a parenthesis token. Consuming it.
    ///                     session.advance();
    ///                 }
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
    fn parse<'code>(session: &mut impl SyntaxSession<'code, Node = Self>, rule: NodeRule) -> Self;

    fn debug(text: impl AsRef<str>) {
        let tokens = TokenBuffer::<Self::Token>::from(text);

        ImmutableSyntaxTree::<Self>::parse_with_observer(
            tokens.cursor(..),
            &mut DebugObserver::default(),
        );
    }
}

pub trait AbstractNode: Send + Sync + 'static {
    fn rule(&self) -> NodeRule;

    fn name(&self) -> Option<&'static str>;

    fn describe(&self, verbose: bool) -> Option<&'static str>;

    fn node_ref(&self) -> NodeRef;

    fn parent_ref(&self) -> NodeRef;

    fn set_parent_ref(&mut self, parent_ref: NodeRef);

    fn capture(&self, key: Key) -> Option<Capture>;

    #[inline(always)]
    fn first_capture(&self) -> Option<Capture> {
        self.capture(Key::Index(0))
    }

    #[inline(always)]
    fn last_capture(&self) -> Option<Capture> {
        self.capture(Key::Index(self.captures_len().checked_sub(1)?))
    }

    fn capture_keys(&self) -> &'static [Key<'static>];

    #[inline(always)]
    fn captures_len(&self) -> usize {
        self.capture_keys().len()
    }

    #[inline(always)]
    fn captures_iter(&self) -> CapturesIter<Self>
    where
        Self: Sized,
    {
        CapturesIter::new(self)
    }

    #[inline(always)]
    fn children_iter(&self) -> ChildrenIter<Self>
    where
        Self: Sized,
    {
        ChildrenIter::new(self)
    }

    fn prev_child_node(&self, current: &NodeRef) -> Option<&NodeRef>
    where
        Self: Sized,
    {
        let mut nodes = self
            .children_iter()
            .rev()
            .filter(|child| child.kind().is_node())
            .map(|child| child.as_node_ref());

        loop {
            let probe = nodes.next()?;

            if probe == current {
                return nodes.next();
            }
        }
    }

    fn next_child_node(&self, current: &NodeRef) -> Option<&NodeRef>
    where
        Self: Sized,
    {
        let mut nodes = self
            .children_iter()
            .filter(|child| child.kind().is_node())
            .map(|child| child.as_node_ref());

        loop {
            let probe = nodes.next()?;

            if probe == current {
                return nodes.next();
            }
        }
    }

    fn span(&self, unit: &impl CompilationUnit) -> Option<SiteSpan>
    where
        Self: Sized,
    {
        let start = self.start(unit)?;
        let end = self.end(unit)?;

        Some(start..end)
    }

    fn start(&self, unit: &impl CompilationUnit) -> Option<Site>
    where
        Self: Sized,
    {
        for child in self.captures_iter() {
            match child.start(unit) {
                None => continue,
                Some(site) => return Some(site),
            }
        }

        None
    }

    fn end(&self, unit: &impl CompilationUnit) -> Option<Site>
    where
        Self: Sized,
    {
        for child in self.captures_iter().rev() {
            match child.end(unit) {
                None => continue,
                Some(site) => return Some(site),
            }
        }

        None
    }

    fn rule_name(rule: NodeRule) -> Option<&'static str>
    where
        Self: Sized;

    fn rule_description(rule: NodeRule, verbose: bool) -> Option<&'static str>
    where
        Self: Sized;
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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeRef {
    /// An [identifier](crate::arena::Id) of the [SyntaxTree](crate::syntax::SyntaxTree) instance
    /// this weakly referred [Node] belongs to.
    pub id: Id,

    /// An internal weak reference of the Node object in the
    /// [Cluster](crate::syntax::Cluster).
    ///
    /// If `node_ref` is a [`Ref::Primary`](crate::arena::Entry::Primary) variant, the NodeRef object
    /// refers [`Cluster::primary`](crate::syntax::Cluster::primary) object. Otherwise `node_ref` is
    /// a [`Entry::Repo`] variant that refers an object from the
    /// [`Cluster::nodes`](crate::syntax::Cluster::nodes) repository.
    pub entry: Entry,
}

impl Debug for NodeRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        match self.is_nil() {
            false => formatter.write_fmt(format_args!(
                "NodeRef(id: {:?}, entry: {:?})",
                self.id, self.entry,
            )),
            true => formatter.write_str("NodeRef(Nil)"),
        }
    }
}

impl Identifiable for NodeRef {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl Default for NodeRef {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl PolyRef for NodeRef {
    #[inline(always)]
    fn kind(&self) -> RefKind {
        RefKind::Node
    }

    #[inline(always)]
    fn is_nil(&self) -> bool {
        self.id.is_nil() || self.entry.is_nil()
    }

    #[inline(always)]
    fn as_variant(&self) -> PolyVariant {
        PolyVariant::Node(*self)
    }

    #[inline(always)]
    fn as_token_ref(&self) -> &TokenRef {
        &NIL_TOKEN_REF
    }

    #[inline(always)]
    fn as_node_ref(&self) -> &NodeRef {
        self
    }

    #[inline(always)]
    fn span(&self, unit: &impl CompilationUnit) -> Option<SiteSpan> {
        self.deref(unit)?.span(unit)
    }
}

impl NodeRef {
    /// Returns an invalid instance of the NodeRef.
    ///
    /// This instance never resolves to valid [Node].
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            entry: Entry::nil(),
        }
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
        if self.id != tree.id() {
            return None;
        }

        tree.get_node(&self.entry)
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
        if self.id != tree.id() {
            return None;
        }

        tree.get_node_mut(&self.entry)
    }

    #[inline(always)]
    pub fn rule(&self, tree: &impl SyntaxTree) -> NodeRule {
        self.deref(tree).map(AbstractNode::rule).unwrap_or(NON_RULE)
    }

    #[inline(always)]
    pub fn name<N: Node>(&self, tree: &impl SyntaxTree<Node = N>) -> Option<&'static str> {
        self.deref(tree).map(AbstractNode::name).flatten()
    }

    #[inline(always)]
    pub fn describe<N: Node>(
        &self,
        tree: &impl SyntaxTree<Node = N>,
        verbose: bool,
    ) -> Option<&'static str> {
        self.deref(tree)
            .map(|node| node.describe(verbose))
            .flatten()
    }

    #[inline(always)]
    pub fn parent(&self, tree: &impl SyntaxTree) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        node.parent_ref()
    }

    pub fn first_child(&self, tree: &impl SyntaxTree) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        node.children_iter()
            .filter(|child| child.kind().is_node())
            .map(|child| child.as_node_ref())
            .next()
            .copied()
            .unwrap_or_default()
    }

    pub fn last_child(&self, tree: &impl SyntaxTree) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        node.children_iter()
            .rev()
            .filter(|child| child.kind().is_node())
            .map(|child| child.as_node_ref())
            .next()
            .copied()
            .unwrap_or_default()
    }

    pub fn get_child<'a>(&self, tree: &impl SyntaxTree, key: impl Into<Key<'a>>) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        let Some(child) = node.capture(key.into()) else {
            return NodeRef::nil();
        };

        let Some(first) = child.first() else {
            return NodeRef::nil();
        };

        *first.as_node_ref()
    }

    pub fn get_token(&self, tree: &impl SyntaxTree, key: &'static str) -> TokenRef {
        let Some(node) = self.deref(tree) else {
            return TokenRef::nil();
        };

        let Some(child) = node.capture(key.into()) else {
            return TokenRef::nil();
        };

        let Some(first) = child.first() else {
            return TokenRef::nil();
        };

        *first.as_token_ref()
    }

    pub fn prev_sibling(&self, tree: &impl SyntaxTree) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        let Some(parent) = node.parent_ref().deref(tree) else {
            return NodeRef::nil();
        };

        let Some(sibling) = parent.prev_child_node(self) else {
            return NodeRef::nil();
        };

        *sibling
    }

    pub fn next_sibling(&self, tree: &impl SyntaxTree) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        let Some(parent) = node.parent_ref().deref(tree) else {
            return NodeRef::nil();
        };

        let Some(sibling) = parent.next_child_node(self) else {
            return NodeRef::nil();
        };

        *sibling
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
        if self.id != tree.id() {
            return false;
        }

        tree.has_node(&self.entry)
    }
}
