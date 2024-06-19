////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::HashSet,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
};

use crate::{
    arena::{Entry, Id, Identifiable},
    format::{AnnotationPriority, SnippetFormatter},
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
    syntax::{AbstractNode, Node, NodeRule, NodeSet, RecoveryResult, SyntaxTree, ROOT_RULE},
    units::CompilationUnit,
};

/// An [ErrorRef] reference that does not point to any syntax error.
///
/// The value of this static equals to the [ErrorRef::nil] value.
pub static NIL_ERROR_REF: ErrorRef = ErrorRef::nil();

/// A syntax error that may occur during the parsing process.
///
/// In Lady Deirdre syntax parsing is an
/// [infallible](crate::syntax::SyntaxSession#parsing-algorithm-considerations)
/// process. Hence, the syntax error object represents a report of the parser's
/// error recovery attempt.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SyntaxError {
    /// A [span of tokens](SiteRefSpan) where the error occurred.
    pub span: SiteRefSpan,

    /// A parsing rule that reported the error.
    pub context: NodeRule,

    /// A type of the recovery strategy that has been applied.
    pub recovery: RecoveryResult,

    /// A set of tokens that the parser expected in the [span](Self::span).
    pub expected_tokens: &'static TokenSet,

    /// A set of nodes that the parser expected in the [span](Self::span).
    pub expected_nodes: &'static NodeSet,
}

impl SyntaxError {
    /// Returns a displayable object that prints a canonical title of
    /// this syntax error.
    #[inline(always)]
    pub fn title<N: AbstractNode>(&self) -> impl Display + '_ {
        struct Title<'error, N> {
            error: &'error SyntaxError,
            _node: PhantomData<N>,
        }

        impl<'error, N: Node> Debug for Title<'error, N> {
            #[inline(always)]
            fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
                Display::fmt(self, formatter)
            }
        }

        impl<'error, N: AbstractNode> Display for Title<'error, N> {
            #[inline(always)]
            fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
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

    /// Returns a displayable object that prints a canonical message of
    /// this syntax error.
    ///
    /// The `code` parameter provides access to the compilation unit's tokens
    /// of where the error occurred.
    #[inline(always)]
    pub fn message<N: Node>(
        &self,
        code: &impl SourceCode<Token = <N as Node>::Token>,
    ) -> impl Debug + Display + '_ {
        struct Message<'error, N> {
            error: &'error SyntaxError,
            empty_span: bool,
            _node: PhantomData<N>,
        }

        impl<'error, N: Node> Debug for Message<'error, N> {
            #[inline(always)]
            fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
                Display::fmt(self, formatter)
            }
        }

        impl<'error, N: Node> Display for Message<'error, N> {
            fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
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
                    set: HashSet<&'static str>,
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
                        let set = HashSet::with_capacity(capacity);

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
                    self.error.expected_tokens.len() + self.error.expected_nodes.len(),
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

    /// Returns a displayable object that prints a
    /// [Snippet](crate::format::Snippet) that annotates the source code span
    /// with an error message.
    ///
    /// The `unit` parameter provides access to the compilation unit of where
    /// the error occurred.
    #[inline(always)]
    pub fn display<'a>(&'a self, unit: &'a impl CompilationUnit) -> impl Debug + Display + '_ {
        struct DisplaySyntaxError<'a, U: CompilationUnit> {
            error: &'a SyntaxError,
            unit: &'a U,
        }

        impl<'a, U: CompilationUnit> Debug for DisplaySyntaxError<'a, U> {
            #[inline(always)]
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                Display::fmt(self, formatter)
            }
        }

        impl<'a, U: CompilationUnit> Display for DisplaySyntaxError<'a, U> {
            #[inline]
            fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
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
                        AnnotationPriority::Primary,
                        format!("{}", self.error.message::<U::Node>(self.unit)),
                    )
                    .finish()
            }
        }

        DisplaySyntaxError { error: self, unit }
    }

    /// Computes a [token span](SiteRefSpan) from the syntax error's original
    /// span such that the new span would be properly aligned in regards to
    /// the whitespaces and the line breaks surrounding the original span.
    ///
    /// The `code` parameter provides access to the compilation unit's tokens
    /// of where the error occurred.
    ///
    /// The exact details of the underlying algorithm are not specified,
    /// and the algorithm is subject to improvements over time in the minor
    /// versions of this crate, but the function attempts to generate a span
    /// that would better fit for the end-user facing rather than the original
    /// machine-generated span.
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

/// A globally unique reference of the [syntax error](SyntaxError) in the
/// syntax tree.
///
/// Each [syntax tree's](SyntaxTree) syntax error could be uniquely
/// addressed within a pair of the [Id] and [Entry], where the identifier
/// uniquely addresses a specific compilation unit instance (syntax tree), and
/// the entry part addresses a syntax error within this tree.
///
/// Essentially, ErrorRef is a composite index.
///
/// Both components of this index form a unique pair
/// (within the current process), because each compilation unit has a unique
/// identifier, and the syntax errors within the syntax tree always receive
/// unique [Entry] indices within the syntax tree.
///
/// If the syntax error instance has been removed from the syntax tree
/// over time, new syntax error within this syntax tree will never occupy
/// the same ErrorRef object, but the ErrorRef referred to the removed
/// SyntaxError would become _invalid_.
///
/// The [nil](ErrorRef::nil) ErrorRefs are special references that are
/// considered to be always invalid (they intentionally don't refer
/// to any syntax error within any syntax tree).
///
/// Two distinct instances of the nil ErrorRef are always equal.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorRef {
    /// An identifier of the syntax tree.
    pub id: Id,

    /// A versioned index of the [syntax error](SyntaxError) instance
    /// within the syntax tree.
    pub entry: Entry,
}

impl Debug for ErrorRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
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
    /// Returns an ErrorRef that intentionally does not refer
    /// to any syntax error within any syntax tree.
    ///
    /// If you need just a static reference to the nil ErrorRef, use
    /// the predefined [NIL_ERROR_REF] static.
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            entry: Entry::nil(),
        }
    }

    /// Returns true, if the underlying reference intentionally does not refer
    /// to any syntax error within any syntax tree.
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.id.is_nil() || self.entry.is_nil()
    }

    /// Immutably borrows a syntax tree's syntax error referred to by
    /// this ErrorRef.
    ///
    /// Returns None if this ErrorRef is not valid for specified `tree`.
    #[inline(always)]
    pub fn deref<'tree, N: Node>(
        &self,
        tree: &'tree impl SyntaxTree<Node = N>,
    ) -> Option<&'tree SyntaxError> {
        if self.id != tree.id() {
            return None;
        }

        tree.get_error(&self.entry)
    }

    /// Returns true if the syntax error referred to by this ErrorRef exists in
    /// the specified `tree`.
    #[inline(always)]
    pub fn is_valid_ref(&self, tree: &impl SyntaxTree) -> bool {
        if self.id != tree.id() {
            return false;
        }

        tree.has_error(&self.entry)
    }
}
