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

use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
    mem::replace,
};

use crate::{
    arena::{Id, Identifiable, SubId},
    lexis::{
        Line,
        Position,
        PositionSpan,
        Site,
        SiteSpan,
        SourceCode,
        ToSpan,
        Token,
        TokenBuffer,
        TokenRef,
        TokenRule,
    },
    report::ld_unreachable,
    syntax::{ErrorRef, ImmutableSyntaxTree, Node, NodeRef, NodeRule, Observer, ROOT_RULE},
    units::{CompilationUnit, Lexis, Syntax},
};

/// A reconstruction of the concrete parsing tree.
///
/// The [SyntaxTree](crate::syntax::SyntaxTree) represents an abstract syntax
/// tree wherein all whitespaces, comments and semantically meaningless parts of
/// the source code intentionally omitted.
///
/// In contrast, the ParseTree includes everything that originally existed
/// in the source code, including the
/// [column-line position spans](crate::lexis::PositionSpan) of the of
/// the source code text from which the parse tree nodes originated.
///
/// Parse trees are of particular interest to code formatters.
///
/// The parse tree [constructor](ParseTree::new) reconstructs the tree based
/// on the syntax parsing algorithm's interactions with
/// the [SyntaxSession](crate::syntax::SyntaxSession) rather the the syntax rule
/// products.
///
/// For convenient purposes of the code formatter authors, parse tree child
/// nodes are owned by their parent nodes and could be directly borrowed and
/// traversed without the extra abstractions.
///
/// The [parse_tree_root](ParseTree::parse_tree_root) and
/// the [parse_tree_root_mut](ParseTree::parse_tree_root_mut) functions return
/// references to the parse tree root node.
///
/// The parse tree nodes and their children exhaustively cover the input parsed
/// sequence of tokens without overlap.
pub struct ParseTree<'a, N: Node, C: SourceCode<Token = N::Token>> {
    code: &'a C,
    syntax: ImmutableSyntaxTree<N>,
    root: ParseNode,
}

impl<'a, N: Node, C: SourceCode<Token = N::Token>> Identifiable for ParseTree<'a, N, C> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.code.id()
    }
}

impl<'a, N: Node, C: SourceCode<Token = N::Token>> Lexis for ParseTree<'a, N, C> {
    type Lexis = C;

    #[inline(always)]
    fn lexis(&self) -> &Self::Lexis {
        self.code
    }
}

impl<'a, N: Node, C: SourceCode<Token = N::Token>> Syntax for ParseTree<'a, N, C> {
    type Syntax = ImmutableSyntaxTree<N>;

    #[inline(always)]
    fn syntax(&self) -> &Self::Syntax {
        &self.syntax
    }

    #[inline(always)]
    fn syntax_mut(&mut self) -> &mut Self::Syntax {
        &mut self.syntax
    }
}

impl<'a, N: Node, C: SourceCode<Token = N::Token>> CompilationUnit for ParseTree<'a, N, C> {
    #[inline(always)]
    fn is_mutable(&self) -> bool {
        false
    }

    #[inline(always)]
    fn into_token_buffer(self) -> TokenBuffer<<Self as SourceCode>::Token> {
        TokenBuffer::from(self.code.substring(..))
    }
}

impl<'a, N: Node, C: SourceCode<Token = N::Token>> Debug for ParseTree<'a, N, C> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.root.debug("", self, formatter)
    }
}

impl<'a, N: Node, C: SourceCode<Token = N::Token>> ParseTree<'a, N, C> {
    /// Creates the parse tree by parsing the sequence of tokens.
    ///
    /// The `code` parameter is an object that grants access to the source code
    /// tokens (e.g., a [TokenBuffer] or a [Document](crate::units::Document)).
    ///
    /// The `span` parameter is a range of tokens in the `code` that requires
    /// parsing (use `..` to parse the entire code).
    ///
    /// The generic parameter `N` of type [Node] of the ParseTree specifies
    /// programming language syntax grammar.
    pub fn new(code: &'a C, span: impl ToSpan) -> Self {
        let mut builder = ParseTreeBuilder {
            code,
            site: 0,
            position: Position::default(),
            stack: Vec::new(),
            node: PhantomData,
        };

        let syntax = ImmutableSyntaxTree::parse_with_id_and_observer(
            SubId::fork(code.id()),
            code.cursor(span),
            &mut builder,
        );

        let root = builder.stack.pop().unwrap_or_else(|| ParseNode {
            rule: ROOT_RULE,
            node_ref: NodeRef::nil(),
            site_span: 0..0,
            position_span: Position::default()..Position::default(),
            well_formed: false,
            children: Vec::new(),
        });

        Self { code, syntax, root }
    }

    /// Grants immutable access to the root node of the parse tree.
    #[inline(always)]
    pub fn parse_tree_root(&self) -> &ParseNode {
        &self.root
    }

    /// Grants mutable access to the root node of the parse tree.
    #[inline(always)]
    pub fn parse_tree_root_mut(&mut self) -> &mut ParseNode {
        &mut self.root
    }
}

/// A child of the parse tree [node](ParseNode).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ParseNodeChild {
    /// A sequence of blank tokens (whitespaces and line breaks).
    Blank(ParseBlank),

    /// A single token.
    Token(ParseToken),

    /// A sub-node.
    Node(ParseNode),
}

impl ParseNodeChild {
    /// Returns the number of line breaks covered by this child.
    #[inline(always)]
    pub fn breaks(&self) -> usize {
        match self {
            ParseNodeChild::Blank(child) => child.breaks(),
            ParseNodeChild::Token(child) => child.breaks(),
            ParseNodeChild::Node(child) => child.breaks(),
        }
    }

    /// Returns the first line number of the span covered by this child.
    #[inline(always)]
    pub fn start_line(&self) -> Line {
        match self {
            ParseNodeChild::Blank(child) => child.start_line(),
            ParseNodeChild::Token(child) => child.start_line(),
            ParseNodeChild::Node(child) => child.start_line(),
        }
    }

    /// Returns the last line number of the span covered by this child.
    #[inline(always)]
    pub fn end_line(&self) -> Line {
        match self {
            ParseNodeChild::Blank(child) => child.end_line(),
            ParseNodeChild::Token(child) => child.end_line(),
            ParseNodeChild::Node(child) => child.end_line(),
        }
    }

    /// Returns true if no syntax errors have been detected during this
    /// child parsing.
    pub fn well_formed(&self) -> bool {
        match self {
            ParseNodeChild::Blank(_) => true,
            ParseNodeChild::Token(_) => true,
            ParseNodeChild::Node(child) => child.well_formed,
        }
    }

    fn debug<'a, N: Node, C: SourceCode<Token = N::Token>>(
        &self,
        indent: &str,
        tree: &ParseTree<'a, N, C>,
        formatter: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ParseNodeChild::Blank(child) => child.debug(indent, tree, formatter),
            ParseNodeChild::Token(child) => child.debug(indent, tree, formatter),
            ParseNodeChild::Node(child) => child.debug(indent, tree, formatter),
        }
    }
}

/// A contiguous sequence of tokens in the [ParseTree] that represents only
/// whitespaces and line breaks.
///
/// For convenience, the ParseTree groups contiguous tokens, each of which
/// contains only whitespace and line break chars, into a dedicated object.
///
/// Blank chars are the ASCII chars from the set of `' '`, `'\r'`, `'\x0C'`,
/// `'\t'`, `'\n'`.
///
/// In other words, these are chars for which the [char::is_ascii_whitespace]
/// function returns true.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ParseBlank {
    /// The site span covered by this token sequence.
    pub site_span: SiteSpan,

    /// The column-line span covered by this token sequence.
    pub position_span: PositionSpan,

    /// The sequence of [TokenRef] references of the actual token instances
    /// in the [SourceCode] that comprise this ParseBlank object.
    pub children: Vec<TokenRef>,
}

impl ParseBlank {
    /// Returns the number of line breaks covered by this token sequence.
    #[inline(always)]
    pub fn breaks(&self) -> usize {
        self.position_span.end.line - self.position_span.start.line
    }

    /// Returns the first line number of the span covered by this token sequence.
    #[inline(always)]
    pub fn start_line(&self) -> Line {
        self.position_span.start.line
    }

    /// Returns the last line number of the span covered by this token sequence.
    #[inline(always)]
    pub fn end_line(&self) -> Line {
        self.position_span.end.line
    }

    #[inline(always)]
    fn debug<'a, N: Node, C: SourceCode<Token = N::Token>>(
        &self,
        indent: &str,
        tree: &ParseTree<'a, N, C>,
        formatter: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        let span = self.position_span.display(tree);

        formatter.write_fmt(format_args!("{indent}<blank> [{span}]"))
    }
}

/// A single token in the [ParseTree].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ParseToken {
    /// The rule that parses this token.
    pub rule: TokenRule,

    /// The [TokenRef] reference of the instance of this token within
    /// the [SourceCode].
    pub token_ref: TokenRef,

    /// The site span covered by this token.
    pub site_span: SiteSpan,

    /// The column-line span covered by this token.
    pub position_span: PositionSpan,
}

impl ParseToken {
    /// Returns the number of line breaks covered by this token.
    #[inline(always)]
    pub fn breaks(&self) -> usize {
        self.position_span.end.line - self.position_span.start.line
    }

    /// Returns the first line number of the span covered by this token.
    #[inline(always)]
    pub fn start_line(&self) -> Line {
        self.position_span.start.line
    }

    /// Returns the last line number of the span covered by this token sequence.
    #[inline(always)]
    pub fn end_line(&self) -> Line {
        self.position_span.end.line
    }

    #[inline(always)]
    fn debug<'a, N: Node, C: SourceCode<Token = N::Token>>(
        &self,
        indent: &str,
        tree: &ParseTree<'a, N, C>,
        formatter: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        let name = <N::Token as Token>::rule_name(self.rule).unwrap_or("?");

        let span = self.position_span.display(tree);

        formatter.write_fmt(format_args!("{indent}${name} [{span}]"))
    }
}

/// A single parse node in the [ParseTree].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ParseNode {
    /// The rule that parses this node.
    pub rule: NodeRule,

    /// The [NodeRef] reference of the instance of this node within
    /// the [SyntaxTree](crate::syntax::SyntaxTree).
    pub node_ref: NodeRef,

    /// The site span covered by this node.
    pub site_span: SiteSpan,

    /// The column-line span covered by this node.
    pub position_span: PositionSpan,

    /// True, if no syntax errors have been detected during this node parsing.
    pub well_formed: bool,

    /// The child entities of this parse node.
    pub children: Vec<ParseNodeChild>,
}

impl ParseNode {
    /// Returns the number of line breaks covered by this node.
    #[inline(always)]
    pub fn breaks(&self) -> usize {
        self.position_span.end.line - self.position_span.start.line
    }

    /// Returns the first line number of the span covered by this node.
    #[inline(always)]
    pub fn start_line(&self) -> Line {
        self.position_span.start.line
    }

    /// Returns the last line number of the span covered by this node.
    #[inline(always)]
    pub fn end_line(&self) -> Line {
        self.position_span.end.line
    }

    #[inline(always)]
    fn debug<'a, N: Node, C: SourceCode<Token = N::Token>>(
        &self,
        indent: &str,
        tree: &ParseTree<'a, N, C>,
        formatter: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        let name = N::rule_name(self.rule).unwrap_or("?");

        let span = self.position_span.display(tree);

        formatter.write_fmt(format_args!("{indent}{name} [{span}] {{"))?;

        if self.children.is_empty() {
            formatter.write_str("}")?;

            return Ok(());
        }

        formatter.write_str("\n")?;

        let inner_indent = format!("{indent}    ");

        for child in &self.children {
            child.debug(&inner_indent, tree, formatter)?;
            formatter.write_str("\n")?;
        }

        formatter.write_fmt(format_args!("{indent}}}"))?;

        Ok(())
    }
}

struct ParseTreeBuilder<'a, N: Node, C: SourceCode<Token = N::Token>> {
    code: &'a C,
    site: Site,
    position: Position,
    stack: Vec<ParseNode>,
    node: PhantomData<N>,
}

impl<'a, N, C> Observer for ParseTreeBuilder<'a, N, C>
where
    N: Node,
    C: SourceCode<Token = N::Token>,
{
    type Node = N;

    fn read_token(&mut self, token: <Self::Node as Node>::Token, token_ref: TokenRef) {
        let start_site = self.site;
        let start_position = self.position;

        if let Some(length) = token_ref.length(self.code) {
            self.site += length;
        };

        let mut is_blank = true;

        if let Some(string) = token_ref.string(self.code) {
            for ch in string.chars() {
                match ch {
                    ' ' | '\r' | '\x0c' | '\t' => {
                        self.position.column += 1;
                    }

                    '\n' => {
                        self.position.line += 1;
                        self.position.column = 1;
                    }

                    _ => {
                        self.position.column += 1;
                        is_blank = false;
                    }
                }
            }
        };

        let end_site = self.site;
        let end_position = self.position;

        let Some(ParseNode { children, .. }) = self.stack.last_mut() else {
            return;
        };

        if is_blank {
            match children.last_mut() {
                Some(ParseNodeChild::Blank(ParseBlank {
                    site_span,
                    position_span,
                    children,
                })) => {
                    site_span.end = end_site;
                    position_span.end = end_position;
                    children.push(token_ref);
                }

                _ => children.push(ParseNodeChild::Blank(ParseBlank {
                    site_span: start_site..end_site,
                    position_span: start_position..end_position,
                    children: Vec::from([token_ref]),
                })),
            }

            return;
        }

        children.push(ParseNodeChild::Token(ParseToken {
            rule: token.rule(),
            token_ref,
            site_span: start_site..end_site,
            position_span: start_position..end_position,
        }));
    }

    fn enter_rule(&mut self, rule: NodeRule, node_ref: NodeRef) {
        self.stack.push(ParseNode {
            rule,
            node_ref,
            site_span: self.site..self.site,
            position_span: self.position..self.position,
            well_formed: true,
            children: Vec::new(),
        });
    }

    fn leave_rule(&mut self, _rule: NodeRule, _node_ref: NodeRef) {
        let Some(mut child) = self.stack.pop() else {
            return;
        };

        child.site_span.end = self.site;
        child.position_span.end = self.position;

        let Some(parent) = self.stack.last_mut() else {
            self.stack.push(child);
            return;
        };

        parent.children.push(ParseNodeChild::Node(child));
    }

    fn lift_node(&mut self, node_ref: NodeRef) {
        let Some(current) = self.stack.last() else {
            return;
        };

        if !current.children.is_empty() {
            return;
        }

        let Some(parent_index) = self.stack.len().checked_sub(2) else {
            return;
        };

        let Some(parent) = self.stack.get_mut(parent_index) else {
            return;
        };

        let mut from_index = usize::MAX;
        let mut from_site = 0;
        let mut from_position = Position::default();

        for (index, sibling) in parent.children.iter().enumerate().rev() {
            let ParseNodeChild::Node(sibling) = sibling else {
                continue;
            };

            if sibling.node_ref == node_ref {
                from_index = index;
                from_site = sibling.site_span.start;
                from_position = sibling.position_span.start;
                break;
            }
        }

        if from_index >= parent.children.len() {
            return;
        }

        let lifted = parent.children.drain(from_index..).collect::<Vec<_>>();

        let Some(current) = self.stack.last_mut() else {
            // Safety: Emptiness checked above.
            unsafe { ld_unreachable!("Missing current segment.") };
        };

        current.site_span.start = from_site;
        current.position_span.start = from_position;

        let mut tail = replace(&mut current.children, lifted);

        current.children.append(&mut tail);
    }

    fn syntax_error(&mut self, _error_ref: ErrorRef) {
        let Some(current) = self.stack.last_mut() else {
            return;
        };

        current.well_formed = false;
    }
}
