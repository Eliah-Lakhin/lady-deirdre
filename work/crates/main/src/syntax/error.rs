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
    arena::{Id, Identifiable, Ref},
    compiler::CompilationUnit,
    lexis::{SiteRefSpan, ToSpan, Token, TokenSet},
    std::*,
    syntax::{ClusterRef, Node, RuleIndex, RuleSet, SyntaxTree, ROOT_RULE},
};

/// A base syntax parse error object.
///
/// All custom syntax/semantic errors must be [From](::std::convert::From) this object.
///
/// SyntaxError implements [Display](::std::fmt::Display) trait to provide default syntax error
/// formatter, but an API user is encouraged to implement custom formatter to better represent
/// semantic of particular programming language.
///
/// ```rust
/// use lady_deirdre::syntax::ParseError;
///
/// enum CustomError {
///     SyntaxError(ParseError),
///     SemanticError(&'static str),
/// }
///
/// impl From<ParseError> for CustomError {
///     fn from(err: ParseError) -> Self {
///         Self::SyntaxError(err)
///     }
/// }
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseError {
    /// A [site](crate::lexis::Site) reference span of where the rule has failed.
    pub span: SiteRefSpan,

    /// A name of the rule that has failed.
    pub context: RuleIndex,

    /// A set of tokens that the parser was expected.
    ///
    /// Possibly empty set.
    pub expected_tokens: &'static TokenSet,

    /// A set of named rules that the parser was expected to be descend to.
    ///
    /// Possibly empty set.
    pub expected_rules: &'static RuleSet,
}

impl ParseError {
    #[inline(always)]
    pub fn title<N: Node>(&self) -> impl Display + '_ {
        struct Title<'error, N> {
            error: &'error ParseError,
            _node: PhantomData<N>,
        }

        impl<'error, N: Node> Display for Title<'error, N> {
            #[inline(always)]
            fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                match N::describe(self.error.context) {
                    Some(context) => {
                        formatter.write_fmt(format_args!("Syntax error in {context}."))
                    }
                    None => formatter.write_str("Syntax error."),
                }
            }
        }

        Title {
            error: self,
            _node: PhantomData::<N>,
        }
    }

    #[inline(always)]
    pub fn describe<N: Node>(&self) -> impl Display + '_ {
        const SHORT_LENGTH: usize = 80;

        struct Describe<'error, N> {
            error: &'error ParseError,
            _node: PhantomData<N>,
        }

        impl<'error, N: Node> Display for Describe<'error, N> {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                #[derive(PartialEq, Eq, PartialOrd, Ord)]
                enum RuleOrToken {
                    Rule(&'static str),
                    Token(&'static str),
                }

                impl Display for RuleOrToken {
                    #[inline(always)]
                    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                        match self {
                            Self::Rule(string) => formatter.write_str(string),
                            Self::Token(string) => formatter.write_fmt(format_args!("'{string}'")),
                        }
                    }
                }

                let mut expected_rules = (&self.error.expected_rules)
                    .into_iter()
                    .map(|rule| N::describe(rule))
                    .flatten()
                    .map(RuleOrToken::Rule)
                    .collect::<Vec<_>>();

                let mut expected_tokens = self
                    .error
                    .expected_tokens
                    .into_iter()
                    .map(|rule| <N as Node>::Token::describe(rule))
                    .flatten()
                    .map(RuleOrToken::Token)
                    .collect::<Vec<_>>();

                let total = expected_rules.len() + expected_tokens.len();

                if total == 0 {
                    return match formatter.alternate() {
                        true => match self.error.context == ROOT_RULE {
                            false => match N::describe(self.error.context) {
                                Some(context) if self.error.context != ROOT_RULE => formatter
                                    .write_fmt(format_args!(
                                        "Unexpected end of input in {context}."
                                    )),

                                _ => formatter.write_str("Unexpected end of input."),
                            },
                            true => formatter.write_str("Unexpected end of input."),
                        },
                        false => formatter.write_str("unexpected end of input"),
                    };
                }

                expected_rules.sort();
                expected_tokens.sort();

                let limit;
                let mut length;

                match formatter.alternate() {
                    true => {
                        limit = usize::MAX;
                        length = 0;

                        match total <= 2 {
                            true => {
                                formatter.write_str("Missing ")?;
                            }

                            false => {
                                if let Some(context) = N::describe(self.error.context) {
                                    formatter
                                        .write_fmt(format_args!("{context} format mismatch. "))?;
                                }

                                formatter.write_str("Expected ")?;
                            }
                        }
                    }

                    false => {
                        limit = SHORT_LENGTH;

                        match total == 1 {
                            true => {
                                formatter.write_str("missing ")?;
                                length = "missing ".len();
                            }

                            false => {
                                formatter.write_str("expected ")?;
                                length = "expected ".len();
                            }
                        }
                    }
                }

                let mut index = 0;

                for item in expected_rules.into_iter().chain(expected_tokens) {
                    let mut next = String::with_capacity(SHORT_LENGTH);

                    match index {
                        0 => (),
                        1 => match total == 2 {
                            true => {
                                next += " or ";
                            }

                            false => {
                                next += ", ";
                            }
                        },

                        _ => {
                            next += ", ";
                        }
                    }

                    next += item.to_string().as_str();

                    if index == 0 || length + next.len() < limit {
                        index += 1;
                        length += next.len();
                        formatter.write_str(&next)?;
                        continue;
                    }

                    break;
                }

                match index < total - 1 {
                    true => formatter.write_str("...")?,

                    false => {
                        if formatter.alternate() {
                            if total <= 2 {
                                if let Some(context) = N::describe(self.error.context) {
                                    formatter.write_fmt(format_args!(" in {context}"))?;
                                }
                            }

                            formatter.write_str(".")?;
                        }
                    }
                }

                Ok(())
            }
        }

        Describe {
            error: self,
            _node: PhantomData::<N>,
        }
    }

    #[inline(always)]
    pub fn display<'a>(&'a self, unit: &'a impl CompilationUnit) -> impl Display + '_ {
        struct DisplaySyntaxError<'a, U: CompilationUnit> {
            error: &'a ParseError,
            unit: &'a U,
        }

        impl<'a, U: CompilationUnit> Display for DisplaySyntaxError<'a, U> {
            #[inline]
            fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                Display::fmt(&self.error.span.display(self.unit), formatter)?;
                formatter.write_str(": ")?;
                formatter.write_fmt(format_args!("{:#}", self.error.describe::<U::Node>()))?;

                Ok(())
            }
        }

        DisplaySyntaxError { error: self, unit }
    }
}

/// A weak reference of the syntax/semantic error object inside the syntax tree.
///
/// This objects represents a long-lived lifetime independent and type independent cheap to
/// [Copy](::std::marker::Copy) safe weak reference into the syntax structure of the source code.
///
/// ErrorRef is capable to survive source code incremental changes happening aside of a part of the
/// syntax tree this error belongs to.
///
/// ```rust
/// use lady_deirdre::{
///     Document,
///     syntax::{
///         SimpleNode,
///         SyntaxTree,
///         ParseError,
///         TreeContent,
///         RuleSet,
///         ROOT_RULE,
///         EMPTY_RULE_SET,
///     },
///     lexis::{SiteRef, TokenSet, EMPTY_TOKEN_SET},
/// };
///
/// let mut doc = Document::<SimpleNode>::from("foo bar");
///
/// let new_custom_error_ref = doc.root_node_ref().cluster().link_error(
///     &mut doc,
///     ParseError {
///         span: SiteRef::nil()..SiteRef::nil(),
///         context: ROOT_RULE,
///         expected_tokens: &EMPTY_TOKEN_SET,
///         expected_rules: &EMPTY_RULE_SET,
///     },
/// );
///
/// assert_eq!(
///     new_custom_error_ref.deref(&doc).unwrap().display(&doc).to_string(),
///     "?: Unexpected end of input.",
/// );
///
/// // This change touches "root" node of the syntax tree(the only node of the tree), as such
/// // referred error will not survive.
/// doc.write(0..0, "123");
///
/// assert!(!new_custom_error_ref.is_valid_ref(&doc));
/// ```
///
/// An API user normally does not need to inspect ErrorRef inner fields manually or to construct
/// an ErrorRef manually unless you are working on the Crate API Extension.
///
/// For details on the Weak references framework design see [Arena](crate::arena) module
/// documentation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErrorRef {
    /// An [identifier](crate::arena::Id) of the [SyntaxTree](crate::syntax::SyntaxTree) instance
    /// this weakly referred error object belongs to.
    pub id: Id,

    /// An internal weak reference of the error object's [Cluster](crate::syntax::Cluster) of the
    /// [SyntaxTree](crate::syntax::SyntaxTree) instance.
    pub cluster_ref: Ref,

    /// An internal weak reference of the error object in the
    /// [`Cluster::errors`](crate::syntax::Cluster::errors) repository.
    pub error_ref: Ref,
}

impl Debug for ErrorRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self.is_nil() {
            false => formatter.write_fmt(format_args!("ErrorRef({:?})", self.id())),
            true => formatter.write_str("ErrorRef(Nil)"),
        }
    }
}

impl Identifiable for ErrorRef {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl ErrorRef {
    /// Returns an invalid instance of the ErrorRef.
    ///
    /// This instance never resolves to valid error object.
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            cluster_ref: Ref::Nil,
            error_ref: Ref::Nil,
        }
    }

    /// Returns `true` if this instance will never resolve to valid error object.
    ///
    /// It is guaranteed that `ErrorRef::nil().is_nil()` is always `true`, but in general if
    /// this function returns `false` it is not guaranteed that provided instance is a valid
    /// reference.
    ///
    /// To determine reference validity per specified [SyntaxTree](crate::syntax::SyntaxTree)
    /// instance use [is_valid_ref](crate::syntax::ErrorRef::is_valid_ref) function instead.
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.id.is_nil() || self.cluster_ref.is_nil() || self.error_ref.is_nil()
    }

    /// Immutably dereferences weakly referred error object of specified
    /// [SyntaxTree](crate::syntax::SyntaxTree).
    ///
    /// Returns [None] if this ErrorRef is not valid reference for specified `tree` instance.
    ///
    /// Use [is_valid_ref](crate::syntax::ErrorRef::is_valid_ref) to check ErrorRef validity.
    ///
    /// This function uses [`SyntaxTree::get_cluster`](crate::syntax::SyntaxTree::get_cluster)
    /// function under the hood.
    #[inline(always)]
    pub fn deref<'tree, N: Node>(
        &self,
        tree: &'tree impl SyntaxTree<Node = N>,
    ) -> Option<&'tree <N as Node>::Error> {
        if self.id != tree.id() {
            return None;
        }

        match tree.get_cluster(&self.cluster_ref) {
            None => None,
            Some(cluster) => cluster.errors.get(&self.error_ref),
        }
    }

    /// Mutably dereferences weakly referred error object of specified
    /// [SyntaxTree](crate::syntax::SyntaxTree).
    ///
    /// Returns [None] if this ErrorRef is not valid reference for specified `tree` instance.
    ///
    /// Use [is_valid_ref](crate::syntax::ErrorRef::is_valid_ref) to check ErrorRef validity.
    ///
    /// This function uses
    /// [`SyntaxTree::get_cluster_mut`](crate::syntax::SyntaxTree::get_cluster_mut) function under
    /// the hood.
    #[inline(always)]
    pub fn deref_mut<'tree, N: Node>(
        &self,
        tree: &'tree mut impl SyntaxTree<Node = N>,
    ) -> Option<&'tree mut <N as Node>::Error> {
        if self.id != tree.id() {
            return None;
        }

        match tree.get_cluster_mut(&self.cluster_ref) {
            None => None,
            Some(data) => data.errors.get_mut(&self.error_ref),
        }
    }

    /// Creates a weak reference of the [Cluster](crate::syntax::Cluster) of referred error object.
    #[inline(always)]
    pub fn cluster(&self) -> ClusterRef {
        ClusterRef {
            id: self.id,
            cluster_ref: self.cluster_ref,
        }
    }

    /// Removes an instance of the error object from the [SyntaxTree](crate::syntax::SyntaxTree)
    /// that is weakly referred by this reference.
    ///
    /// Returns [Some] value of the error object if this weak reference is a valid reference of
    /// existing error object inside `tree` instance. Otherwise returns [None].
    ///
    /// Use [is_valid_ref](crate::syntax::ErrorRef::is_valid_ref) to check ErrorRef validity.
    ///
    /// This function uses
    /// [`SyntaxTree::get_cluster_mut`](crate::syntax::SyntaxTree::get_cluster_mut) function under
    /// the hood.
    #[inline(always)]
    pub fn unlink<N: Node>(
        &self,
        tree: &mut impl SyntaxTree<Node = N>,
    ) -> Option<<N as Node>::Error> {
        if self.id != tree.id() {
            return None;
        }

        match tree.get_cluster_mut(&self.cluster_ref) {
            None => None,
            Some(data) => data.errors.remove(&self.error_ref),
        }
    }

    /// Returns `true` if and only if weakly referred error object belongs to specified
    /// [SyntaxTree](crate::syntax::SyntaxTree), and referred error object exists in this SyntaxTree
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

        match tree.get_cluster(&self.cluster_ref) {
            None => false,
            Some(cluster) => cluster.errors.contains(&self.error_ref),
        }
    }
}
