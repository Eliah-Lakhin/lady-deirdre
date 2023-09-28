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
    compiler::CompilationUnit,
    format::{Delimited, PrintString, Priority, SnippetFormatter},
    lexis::{Length, SiteRefSpan, SourceCode, ToSite, ToSpan, Token, TokenRule, TokenSet},
    std::*,
    syntax::{ClusterRef, Node, NodeRule, NodeSet, RecoveryResult, SyntaxTree, ROOT_RULE},
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
    pub fn title<N: Node>(&self) -> impl Display + '_ {
        struct Title<'error, N> {
            error: &'error ParseError,
            _node: PhantomData<N>,
        }

        impl<'error, N: Node> Display for Title<'error, N> {
            #[inline(always)]
            fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
                match N::describe(self.error.context, true) {
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
                    Token(PrintString<'static>),
                    Node(PrintString<'static>),
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
                    fn print_to(&self, target: &mut PrintString<'static>) {
                        match self {
                            Self::Node(string) => target.append(string.clone()),
                            Self::Token(string) => {
                                target.push('\'');
                                target.append(string.clone());
                                target.push('\'');
                            }
                        }
                    }
                }

                struct OutString {
                    alt: bool,
                    set: StdSet<&'static str>,
                    empty_span: bool,
                    context: PrintString<'static>,
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
                        let set = StdSet::new_std_set(capacity);

                        let context = N::describe(context, true)
                            .filter(|_| context != ROOT_RULE)
                            .map(PrintString::borrowed)
                            .unwrap_or(PrintString::empty());

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
                        let description = match <N as Node>::Token::describe(rule, self.alt) {
                            Some(string) => string,
                            None => return,
                        };

                        if self.set.insert(description) {
                            self.components
                                .push(TokenOrNode::Token(PrintString::borrowed(description)));
                        }
                    }

                    fn push_node<N: Node>(&mut self, rule: NodeRule) {
                        let description = match N::describe(rule, self.alt) {
                            Some(string) => string,
                            None => return,
                        };

                        if self.set.insert(description) {
                            self.components
                                .push(TokenOrNode::Node(PrintString::borrowed(description)));
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
                    fn missing_str(&self) -> PrintString<'static> {
                        static STRING: PrintString<'static> = PrintString::borrowed("missing");
                        static ALT_STR: PrintString<'static> = PrintString::borrowed("Missing");

                        match self.alt {
                            false => STRING.clone(),
                            true => ALT_STR.clone(),
                        }
                    }

                    #[inline(always)]
                    fn unexpected_str(&self) -> PrintString<'static> {
                        static STRING: PrintString<'static> =
                            PrintString::borrowed("unexpected input");
                        static ALT_STR: PrintString<'static> =
                            PrintString::borrowed("Unexpected input");

                        match self.alt {
                            false => STRING.clone(),
                            true => ALT_STR.clone(),
                        }
                    }

                    #[inline(always)]
                    fn in_str(&self) -> PrintString<'static> {
                        static STRING: PrintString<'static> = PrintString::borrowed(" in ");
                        static ALT_STR: PrintString<'static> = PrintString::borrowed(" in ");

                        match self.alt {
                            false => STRING.clone(),
                            true => ALT_STR.clone(),
                        }
                    }

                    #[inline(always)]
                    fn eoi_str(&self) -> PrintString<'static> {
                        static STRING: PrintString<'static> =
                            PrintString::borrowed("unexpected end of input");
                        static ALT_STR: PrintString<'static> =
                            PrintString::borrowed("Unexpected end of input");

                        match self.alt {
                            false => STRING.clone(),
                            true => ALT_STR.clone(),
                        }
                    }

                    #[inline(always)]
                    fn or_str(&self) -> PrintString<'static> {
                        static STRING: PrintString<'static> = PrintString::borrowed(" or ");

                        STRING.clone()
                    }

                    #[inline(always)]
                    fn comma_str(&self) -> PrintString<'static> {
                        static STRING: PrintString<'static> = PrintString::borrowed(", ");

                        STRING.clone()
                    }

                    #[inline(always)]
                    fn etc_str(&self) -> PrintString<'static> {
                        static STRING: PrintString<'static> = PrintString::borrowed("…");
                        static ALT_STR: PrintString<'static> = PrintString::borrowed("...");

                        match self.alt {
                            false => STRING.clone(),
                            true => ALT_STR.clone(),
                        }
                    }

                    fn string(&self) -> PrintString<'static> {
                        let mut result = PrintString::empty();

                        let print_components;

                        match self.recovery {
                            RecoveryResult::InsertRecover => {
                                result.append(self.missing_str());
                                print_components = true;
                            }

                            RecoveryResult::PanicRecover if self.empty_span => {
                                result.append(self.missing_str());
                                print_components = true;
                            }

                            RecoveryResult::PanicRecover => {
                                result.append(self.unexpected_str());
                                print_components = false;
                            }

                            RecoveryResult::UnexpectedEOI => {
                                result.append(self.eoi_str());
                                print_components = false;
                            }

                            RecoveryResult::UnexpectedToken => {
                                result.append(self.missing_str());
                                print_components = true;
                            }
                        };

                        if print_components {
                            for component in self.components.iter().delimited() {
                                match component.is_first {
                                    true => result.push(' '),
                                    false => match self.components.len() == 2 && self.exhaustive {
                                        true => result.append(self.or_str()),
                                        false => result.append(self.comma_str()),
                                    },
                                }

                                component.print_to(&mut result);
                            }
                        }

                        match self.exhaustive {
                            false => {
                                result.append(self.etc_str());
                            }

                            true => {
                                if !self.context.is_empty() {
                                    result.append(self.in_str());
                                    result.append(self.context.clone());
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

                while string.length() > LENGTH_MAX {
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

            if !previous.token_ref().is_blank(code) {
                break;
            }

            start = previous;
        }

        loop {
            if !end.token_ref().is_blank(code) {
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

            if !previous.token_ref().is_blank(code) {
                break;
            }

            end = previous;
        }

        while start != end {
            if !start.token_ref().is_blank(code) {
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

                if !previous.token_ref().is_blank(code) {
                    break;
                }

                site_ref = previous;
            }

            start = site_ref;
            end = site_ref;
        }

        start..end
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
/// let new_custom_error_ref = doc.root_node_ref().cluster_ref().link_error(
///     &mut doc,
///     ParseError {
///         span: SiteRef::nil()..SiteRef::nil(),
///         context: ROOT_RULE,
///         recovery: RecoveryResult::UnexpectedEOI,
///         expected_tokens: &EMPTY_TOKEN_SET,
///         expected_nodes: &EMPTY_NODE_SET,
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
    pub cluster_entry: Entry,

    /// An internal weak reference of the error object in the
    /// [`Cluster::errors`](crate::syntax::Cluster::errors) repository.
    pub error_entry: Entry,
}

impl Debug for ErrorRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        match self.is_nil() {
            false => formatter.write_fmt(format_args!(
                "ErrorRef(id: {:?}, cluster_entry: {:?}, error_entry: {:?})",
                self.id, self.cluster_entry, self.error_entry,
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
            cluster_entry: Entry::Nil,
            error_entry: Entry::Nil,
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
        self.id.is_nil() || self.cluster_entry.is_nil() || self.error_entry.is_nil()
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

        match tree.get_cluster(&self.cluster_entry) {
            None => None,
            Some(cluster) => cluster.errors.get(&self.error_entry),
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

        match tree.get_cluster_mut(&self.cluster_entry) {
            None => None,
            Some(data) => data.errors.get_mut(&self.error_entry),
        }
    }

    /// Creates a weak reference of the [Cluster](crate::syntax::Cluster) of referred error object.
    #[inline(always)]
    pub fn cluster(&self) -> ClusterRef {
        ClusterRef {
            id: self.id,
            cluster_entry: self.cluster_entry,
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

        match tree.get_cluster_mut(&self.cluster_entry) {
            None => None,
            Some(data) => data.errors.remove(&self.error_entry),
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

        match tree.get_cluster(&self.cluster_entry) {
            None => false,
            Some(cluster) => cluster.errors.contains(&self.error_entry),
        }
    }
}
