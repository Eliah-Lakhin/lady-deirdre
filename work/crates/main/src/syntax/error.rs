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
    lexis::SiteRefSpan,
    std::*,
    syntax::{ClusterRef, Node, SyntaxTree},
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
/// use lady_deirdre::syntax::SyntaxError;
///
/// enum CustomError {
///     SyntaxError(SyntaxError),
///     SemanticError(&'static str),
/// }
///
/// impl From<SyntaxError> for CustomError {
///     fn from(err: SyntaxError) -> Self {
///         Self::SyntaxError(err)
///     }
/// }
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SyntaxError {
    /// A parse rule `context` did not expect continuation of the token input sequence.
    ///
    /// Usually this parse error indicates that the parser (semi-)successfully parsed
    /// input sequence, but in the end it has matched tail tokens that do not fit any top level
    /// parse rules.
    ///
    /// **Example:**
    ///
    /// ```text
    /// fn main() { foo(); }
    ///
    /// fn foo() {}
    ///
    /// bar
    /// ^^^ Unexpected end of input.
    /// ```
    UnexpectedEndOfInput {
        /// A [site](crate::lexis::Site) reference span of where the rule has failed.
        ///
        /// Usually this span is the tail of input site.
        span: SiteRefSpan,

        /// A name of the rule that has failed.
        context: &'static str,
    },

    /// A parse rule `context` expected a `token` in specified `span`.
    ///
    /// Usually this parse error indicates that specific parse rule expected particular token in
    /// particular place, and decided to recover this error using "insert" recovery
    /// strategy(by virtually skipping this unambiguous sub-rule switching to the next sub-rule).
    ///
    /// **Example:**
    ///
    /// ```text
    /// fn main() { foo(10   20); }
    ///                   ^^^ Missing token ",".
    ///
    /// fn foo(x: usize, y: usize) {}
    /// ```
    MissingToken {
        /// A [site](crate::lexis::Site) reference span of where the rule has failed.
        ///
        /// Usually this span is just a single Site.
        span: SiteRefSpan,

        /// A name of the rule that has failed.
        context: &'static str,

        /// A name of expected mismatched token.
        token: &'static str,
    },

    /// A parse rule `context` expected a `token` in specified `span`.
    ///
    /// Usually this parse error indicates that specific parse rule expected particular named rule
    /// in particular place to be descend to, and decided to recover this error using "insert"
    /// recovery strategy(by virtually skipping this unambiguous sub-rule switching to the next
    /// sub-rule).
    ///
    /// **Example:**
    ///
    /// ```text
    /// fn main() { foo(10,   ); }
    ///                    ^^^ Missing rule "Rust expression".
    ///
    /// fn foo(x: usize, y: usize) {}
    /// ```
    MissingRule {
        /// A [site](crate::lexis::Site) reference span of where the rule has failed.
        ///
        /// Usually this span is just a single Site.
        span: SiteRefSpan,

        /// A name of the rule that has failed.
        context: &'static str,

        /// A name of expected mismatched rule.
        rule: &'static str,
    },

    /// A parse rule `context` expected a set of tokens and/or a set of parse rules in specified
    /// `span`.
    ///
    /// Usually this parse error indicates that specific parse rule failed to match specific set of
    /// possible tokens and/or named rules to be descend to due to ambiguity between possible rules
    /// in specified parse position. The rule decided to recover from this error using "panic"
    /// recovery strategy(by virtually skipping a number of tokens ahead until expected token was
    /// found, or just by skipping a number of tokens in some parse context and then skipping
    /// specified sub-rule).
    ///
    /// **Example:**
    ///
    /// ```text
    /// fn main() { foo(10, 20; }
    ///                       ^ Mismatch. ")" or any other expression operator expected,
    ///                         but ";" found.
    ///
    /// fn foo(x: usize, y: usize) {}
    /// ```
    Mismatch {
        /// A [site](crate::lexis::Site) reference span of where the rule has failed.
        span: SiteRefSpan,

        /// A name of the rule that has failed.
        context: &'static str,

        /// A set of tokens that the parser was expected.
        ///
        /// Possibly empty set.
        expected_tokens: Vec<&'static str>,

        /// A set of named rules that the parser was expected to be descend to.
        ///
        /// Possibly empty set.
        expected_rules: Vec<&'static str>,
    },
}

impl Display for SyntaxError {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::UnexpectedEndOfInput { context, .. } => {
                formatter.write_str(&format!("{} unexpected end of input.", context))
            }

            Self::MissingToken { context, token, .. } => {
                formatter.write_str(&format!("Missing ${} in {}.", token, context))
            }

            Self::MissingRule { context, rule, .. } => {
                formatter.write_str(&format!("Missing {} in {}.", rule, context))
            }

            Self::Mismatch {
                context,
                expected_tokens,
                expected_rules,
                ..
            } => {
                let mut expected_tokens = expected_tokens
                    .iter()
                    .map(|token| format!("${}", token))
                    .collect::<Vec<_>>();
                expected_tokens.sort();

                let mut expected_rules = expected_rules
                    .iter()
                    .map(|rule| rule.to_string())
                    .collect::<Vec<_>>();
                expected_rules.sort();

                let expected_len = expected_tokens.len() + expected_rules.len();

                let expected = expected_rules
                    .into_iter()
                    .chain(expected_tokens.into_iter());

                formatter.write_str(context)?;
                formatter.write_str(" format mismatch.")?;

                if expected_len > 0 {
                    formatter.write_str(" Expected ")?;

                    let last = expected_len - 1;

                    let is_multi = last > 1;

                    for (index, expected) in expected.enumerate() {
                        let is_first = index == 0;
                        let is_last = index == last;

                        match (is_first, is_last, is_multi) {
                            (true, _, _) => (),
                            (false, false, _) => formatter.write_str(", ")?,
                            (false, true, true) => formatter.write_str(", or ")?,
                            (false, true, false) => formatter.write_str(" or ")?,
                        }

                        formatter.write_str(&expected)?;
                    }

                    formatter.write_str(".")?;
                }

                Ok(())
            }
        }
    }
}

impl SyntaxError {
    /// A [site](crate::lexis::Site) reference span of where the rule has failed.
    #[inline(always)]
    pub fn span(&self) -> &SiteRefSpan {
        match self {
            Self::UnexpectedEndOfInput { span, .. } => span,
            Self::MissingToken { span, .. } => span,
            Self::MissingRule { span, .. } => span,
            Self::Mismatch { span, .. } => span,
        }
    }

    /// A name of the rule that has failed.
    #[inline(always)]
    pub fn context(&self) -> &'static str {
        match self {
            Self::UnexpectedEndOfInput { context, .. } => context,
            Self::MissingToken { context, .. } => context,
            Self::MissingRule { context, .. } => context,
            Self::Mismatch { context, .. } => context,
        }
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
///     syntax::{SimpleNode, SyntaxTree, SyntaxError},
///     lexis::SiteRef,
/// };
///
/// let mut doc = Document::<SimpleNode>::from("foo bar");
///
/// let new_custom_error_ref = doc.root().cluster().link_error(
///     &mut doc,
///     SyntaxError::UnexpectedEndOfInput {
///         span: SiteRef::nil()..SiteRef::nil(),
///         context: "BAZ",
///     },
/// );
///
/// assert_eq!(
///     new_custom_error_ref.deref(&doc).unwrap().to_string(),
///     "BAZ unexpected end of input.",
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
    fn id(&self) -> &Id {
        &self.id
    }
}

impl ErrorRef {
    /// Returns an invalid instance of the ErrorRef.
    ///
    /// This instance never resolves to valid error object.
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: *Id::nil(),
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
        if &self.id != tree.id() {
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
        if &self.id != tree.id() {
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
        if &self.id != tree.id() {
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
        if &self.id != tree.id() {
            return false;
        }

        match tree.get_cluster(&self.cluster_ref) {
            None => false,
            Some(cluster) => cluster.errors.contains(&self.error_ref),
        }
    }
}
