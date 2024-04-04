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
    arena::{Entry, Id, Identifiable},
    format::{Priority, SnippetFormatter},
    lexis::{
        Length,
        SiteRefSpan,
        SourceCode,
        ToSite,
        ToSpan,
        Token,
        TokenRef,
        TokenRule,
        TokenSet,
    },
    std::*,
    syntax::{AbstractNode, Node, NodeRule, NodeSet, RecoveryResult, SyntaxTree, ROOT_RULE},
    units::CompilationUnit,
};

pub static NIL_ERROR_REF: ErrorRef = ErrorRef::nil();

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
    pub context: NodeRule,

    pub recovery: RecoveryResult,

    /// A set of tokens that the parser was expected.
    ///
    /// Possibly empty set.
    pub expected_tokens: &'static TokenSet,

    /// A set of named rules that the parser was expected to be descend to.
    ///
    /// Possibly empty set.
    pub expected_nodes: &'static NodeSet,
}

impl ParseError {
    #[inline(always)]
    pub fn title<N: AbstractNode>(&self) -> impl Display + '_ {
        struct Title<'error, N> {
            error: &'error ParseError,
            _node: PhantomData<N>,
        }

        impl<'error, N: AbstractNode> Display for Title<'error, N> {
            #[inline(always)]
            fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
                match N::rule_description(self.error.context, true) {
                    Some(context) => formatter.write_fmt(format_args!("{context} syntax error.")),
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
    pub fn message<N: Node>(
        &self,
        code: &impl SourceCode<Token = <N as Node>::Token>,
    ) -> impl Display + '_ {
        struct Message<'error, N> {
            error: &'error ParseError,
            empty_span: bool,
            _node: PhantomData<N>,
        }

        impl<'error, N: Node> Display for Message<'error, N> {
            fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
                const LENGTH_MAX: Length = 80;

                #[derive(PartialEq, Eq)]
                enum TokenOrNode {
                    Token(Cow<'static, str>),
                    Node(Cow<'static, str>),
                }

                impl PartialOrd for TokenOrNode {
                    #[inline(always)]
                    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                        Some(self.cmp(other))
                    }
                }

                impl Ord for TokenOrNode {
                    fn cmp(&self, other: &Self) -> Ordering {
                        match (self, other) {
                            (Self::Token(_), Self::Node(_)) => Ordering::Greater,
                            (Self::Node(_), Self::Token(_)) => Ordering::Less,
                            (Self::Token(this), Self::Token(other)) => this.cmp(other),
                            (Self::Node(this), Self::Node(other)) => this.cmp(other),
                        }
                    }
                }

                impl TokenOrNode {
                    #[inline(always)]
                    fn print_to(&self, target: &mut String) {
                        match self {
                            Self::Node(string) => target.push_str(string.as_ref()),

                            Self::Token(string) => {
                                target.push('\'');
                                target.push_str(string.as_ref());
                                target.push('\'');
                            }
                        }
                    }
                }

                struct OutString {
                    alt: bool,
                    set: StdSet<&'static str>,
                    empty_span: bool,
                    context: Cow<'static, str>,
                    recovery: RecoveryResult,
                    components: Vec<TokenOrNode>,
                    exhaustive: bool,
                }

                impl OutString {
                    fn new<N: Node>(
                        alt: bool,
                        capacity: usize,
                        empty_span: bool,
                        context: NodeRule,
                        recovery: RecoveryResult,
                    ) -> Self {
                        let set = StdSet::new_std_set_with_capacity(capacity);

                        let context = N::rule_description(context, true)
                            .filter(|_| context != ROOT_RULE)
                            .map(Cow::Borrowed)
                            .unwrap_or(Cow::Borrowed(""));

                        Self {
                            alt,
                            set,
                            empty_span,
                            context,
                            recovery,
                            components: Vec::with_capacity(capacity),
                            exhaustive: true,
                        }
                    }

                    fn push_token<N: Node>(&mut self, rule: TokenRule) {
                        let description = match <N as Node>::Token::rule_description(rule, self.alt)
                        {
                            Some(string) => string,
                            None => return,
                        };

                        if self.set.insert(description) {
                            self.components
                                .push(TokenOrNode::Token(Cow::Borrowed(description)));
                        }
                    }

                    fn push_node<N: Node>(&mut self, rule: NodeRule) {
                        let description = match N::rule_description(rule, self.alt) {
                            Some(string) => string,
                            None => return,
                        };

                        if self.set.insert(description) {
                            self.components
                                .push(TokenOrNode::Node(Cow::Borrowed(description)));
                        }
                    }

                    fn shorten(&mut self) -> bool {
                        if self.alt {
                            return false;
                        }

                        if self.components.len() <= 2 {
                            return false;
                        }

                        let _ = self.components.pop();
                        self.exhaustive = false;

                        true
                    }

                    #[inline(always)]
                    fn missing_str(&self) -> &'static str {
                        static STRING: &'static str = "missing";
                        static ALT_STR: &'static str = "Missing";

                        match self.alt {
                            false => STRING,
                            true => ALT_STR,
                        }
                    }

                    #[inline(always)]
                    fn unexpected_str(&self) -> &'static str {
                        static STRING: &'static str = "unexpected input";
                        static ALT_STR: &'static str = "Unexpected input";

                        match self.alt {
                            false => STRING,
                            true => ALT_STR,
                        }
                    }

                    #[inline(always)]
                    fn in_str(&self) -> &'static str {
                        static STRING: &'static str = " in ";
                        static ALT_STR: &'static str = " in ";

                        match self.alt {
                            false => STRING,
                            true => ALT_STR,
                        }
                    }

                    #[inline(always)]
                    fn eoi_str(&self) -> &'static str {
                        static STRING: &'static str = "unexpected end of input";
                        static ALT_STR: &'static str = "Unexpected end of input";

                        match self.alt {
                            false => STRING,
                            true => ALT_STR,
                        }
                    }

                    #[inline(always)]
                    fn or_str(&self) -> &'static str {
                        static STRING: &'static str = " or ";

                        STRING
                    }

                    #[inline(always)]
                    fn comma_str(&self) -> &'static str {
                        static STRING: &'static str = ", ";

                        STRING
                    }

                    #[inline(always)]
                    fn etc_str(&self) -> &'static str {
                        static STRING: &'static str = "…";
                        static ALT_STR: &'static str = "...";

                        match self.alt {
                            false => STRING,
                            true => ALT_STR,
                        }
                    }

                    fn string(&self) -> String {
                        let mut result = String::new();

                        let print_components;

                        match self.recovery {
                            RecoveryResult::InsertRecover => {
                                result.push_str(self.missing_str());
                                print_components = true;
                            }

                            RecoveryResult::PanicRecover if self.empty_span => {
                                result.push_str(self.missing_str());
                                print_components = true;
                            }

                            RecoveryResult::PanicRecover => {
                                result.push_str(self.unexpected_str());
                                print_components = false;
                            }

                            RecoveryResult::UnexpectedEOI => {
                                result.push_str(self.eoi_str());
                                print_components = false;
                            }

                            RecoveryResult::UnexpectedToken => {
                                result.push_str(self.missing_str());
                                print_components = true;
                            }
                        };

                        if print_components {
                            let mut is_first = true;

                            for component in &self.components {
                                match is_first {
                                    true => {
                                        result.push(' ');
                                        is_first = false;
                                    }
                                    false => match self.components.len() == 2 && self.exhaustive {
                                        true => result.push_str(self.or_str()),
                                        false => result.push_str(self.comma_str()),
                                    },
                                }

                                component.print_to(&mut result);
                            }
                        }

                        match self.exhaustive {
                            false => {
                                result.push_str(self.etc_str());
                            }

                            true => {
                                if !self.context.is_empty() {
                                    result.push_str(self.in_str());
                                    result.push_str(self.context.as_ref());
                                }

                                if self.alt {
                                    result.push('.');
                                }
                            }
                        }

                        result
                    }
                }

                let mut out = OutString::new::<N>(
                    formatter.alternate(),
                    self.error.expected_tokens.length() + self.error.expected_nodes.length(),
                    self.empty_span,
                    self.error.context,
                    self.error.recovery,
                );

                for rule in self.error.expected_nodes {
                    out.push_node::<N>(rule);
                }

                for rule in self.error.expected_tokens {
                    out.push_token::<N>(rule);
                }

                out.components.sort();

                let mut string = out.string();

                while string.chars().count() > LENGTH_MAX {
                    if !out.shorten() {
                        break;
                    }

                    string = out.string();
                }

                formatter.write_str(string.as_ref())
            }
        }

        let span = self.aligned_span(code);

        Message {
            error: self,
            empty_span: span.start == span.end,
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
            fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
                let aligned_span = self.error.aligned_span(self.unit);

                if !formatter.alternate() {
                    formatter.write_fmt(format_args!("{}", aligned_span.display(self.unit)))?;
                    formatter.write_str(": ")?;
                    formatter.write_fmt(format_args!(
                        "{:#}",
                        self.error.message::<U::Node>(self.unit)
                    ))?;

                    return Ok(());
                }

                formatter
                    .snippet(self.unit)
                    .set_caption(format!("Unit({})", self.unit.id()))
                    .set_summary(self.error.title::<U::Node>().to_string())
                    .annotate(
                        aligned_span,
                        Priority::Primary,
                        format!("{}", self.error.message::<U::Node>(self.unit)),
                    )
                    .finish()
            }
        }

        DisplaySyntaxError { error: self, unit }
    }

    #[inline(always)]
    pub fn aligned_span(&self, code: &impl SourceCode) -> SiteRefSpan {
        match self.recovery {
            RecoveryResult::InsertRecover => self.widen_span(code),
            _ => self.shorten_span(code),
        }
    }

    fn widen_span(&self, code: &impl SourceCode) -> SiteRefSpan {
        if !self.span.is_valid_span(code) {
            return self.span.clone();
        }

        let mut start = self.span.start;
        let mut end = self.span.end;

        loop {
            let previous = start.prev(code);

            if previous == start {
                break;
            }

            if !previous.is_valid_site(code) {
                break;
            }

            if !Self::is_blank(code, previous.token_ref()) {
                break;
            }

            start = previous;
        }

        loop {
            if !Self::is_blank(code, end.token_ref()) {
                break;
            }

            let next = end.next(code);

            if next == end {
                break;
            }

            if !end.is_valid_site(code) {
                break;
            }

            end = next;
        }

        start..end
    }

    fn shorten_span(&self, code: &impl SourceCode) -> SiteRefSpan {
        if !self.span.is_valid_span(code) {
            return self.span.clone();
        }

        let mut start = self.span.start;
        let mut end = self.span.end;

        while start != end {
            let previous = end.prev(code);

            if previous == end {
                break;
            }

            if !previous.is_valid_site(code) {
                break;
            }

            if !Self::is_blank(code, previous.token_ref()) {
                break;
            }

            end = previous;
        }

        while start != end {
            if !Self::is_blank(code, start.token_ref()) {
                break;
            }

            let next = start.next(code);

            if next == start {
                break;
            }

            if !next.is_valid_site(code) {
                break;
            }

            start = next;
        }

        if start == end {
            let mut site_ref = self.span.start;

            loop {
                let previous = site_ref.prev(code);

                if previous == site_ref {
                    break;
                }

                if !previous.is_valid_site(code) {
                    break;
                }

                if !Self::is_blank(code, previous.token_ref()) {
                    break;
                }

                site_ref = previous;
            }

            start = site_ref;
            end = site_ref;
        }

        start..end
    }

    #[inline(always)]
    fn is_blank(code: &impl SourceCode, token_ref: &TokenRef) -> bool {
        let Some(string) = token_ref.string(code) else {
            return false;
        };

        string
            .as_bytes()
            .iter()
            .all(|&ch| ch == b' ' || ch == b'\t' || ch == b'\r' || ch == b'\n' || ch == b'\x0c')
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
///     units::Document,
///     syntax::{
///         SimpleNode,
///         SyntaxTree,
///         ParseError,
///         RecoveryResult,
///         NodeSet,
///         ROOT_RULE,
///         EMPTY_NODE_SET,
///     },
///     lexis::{SiteRef, TokenSet, EMPTY_TOKEN_SET},
/// };
///
/// let mut doc = Document::<SimpleNode>::from("foo bar");
///
/// // This change touches "root" node of the syntax tree(the only node of the tree), as such
/// // referred error will not survive.
/// doc.write(0..0, "123");
/// ```
///
/// An API user normally does not need to inspect ErrorRef inner fields manually or to construct
/// an ErrorRef manually unless you are working on the Crate API Extension.
///
/// For details on the Weak references framework design see [Arena](crate::arena) module
/// documentation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorRef {
    /// An [identifier](crate::arena::Id) of the [SyntaxTree](crate::syntax::SyntaxTree) instance
    /// this weakly referred error object belongs to.
    pub id: Id,

    /// An internal weak reference of the error object in the
    /// [`Cluster::errors`](crate::syntax::Cluster::errors) repository.
    pub entry: Entry,
}

impl Debug for ErrorRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        match self.is_nil() {
            false => formatter.write_fmt(format_args!(
                "ErrorRef(id: {:?}, entry: {:?})",
                self.id, self.entry,
            )),
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
            entry: Entry::nil(),
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
        self.id.is_nil() || self.entry.is_nil()
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

        tree.get_error(&self.entry)
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

        tree.has_error(&self.entry)
    }
}
