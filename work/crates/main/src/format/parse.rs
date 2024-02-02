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
    arena::{Id, Identifiable},
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
    report::debug_unreachable,
    std::*,
    syntax::{ErrorRef, ImmutableSyntaxTree, Node, NodeRef, NodeRule, Observer, ROOT_RULE},
    units::{CompilationUnit, Lexis, Syntax},
};

pub struct ParseTree<N: Node> {
    lexis: TokenBuffer<N::Token>,
    syntax: ImmutableSyntaxTree<N>,
    root: ParseNode,
}

impl<N: Node> Identifiable for ParseTree<N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.lexis.id()
    }
}

impl<N: Node> Lexis for ParseTree<N> {
    type Lexis = TokenBuffer<N::Token>;

    #[inline(always)]
    fn lexis(&self) -> &Self::Lexis {
        &self.lexis
    }
}

impl<N: Node> Syntax for ParseTree<N> {
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

impl<N: Node> CompilationUnit for ParseTree<N> {
    #[inline(always)]
    fn is_mutable(&self) -> bool {
        false
    }

    #[inline(always)]
    fn into_token_buffer(self) -> TokenBuffer<<Self as SourceCode>::Token> {
        self.lexis
    }
}

impl<N: Node> Debug for ParseTree<N> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        self.root.debug("", self, formatter)
    }
}

impl<N: Node> ParseTree<N> {
    pub fn new(text: impl Into<TokenBuffer<N::Token>>) -> Self {
        let lexis = text.into();

        let mut builder = ParseTreeBuilder {
            lexis: &lexis,
            site: 0,
            position: Position::default(),
            stack: Vec::new(),
        };

        let syntax = ImmutableSyntaxTree::new(lexis.id(), lexis.cursor(..), &mut builder);

        let root = builder.stack.pop().unwrap_or_else(|| ParseNode {
            rule: ROOT_RULE,
            node_ref: NodeRef::nil(),
            site_span: 0..0,
            position_span: Position::default()..Position::default(),
            children: Vec::new(),
        });

        Self {
            lexis,
            syntax,
            root,
        }
    }

    #[inline(always)]
    pub fn root_segment(&self) -> &ParseNode {
        &self.root
    }

    #[inline(always)]
    pub fn root_segment_mut(&mut self) -> &mut ParseNode {
        &mut self.root
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ParseSegment {
    Blank(ParseBlank),
    Token(ParseToken),
    Node(ParseNode),
    Error(ErrorRef),
}

impl ParseSegment {
    #[inline(always)]
    pub fn breaks(&self) -> usize {
        match self {
            ParseSegment::Blank(segment) => segment.breaks(),
            ParseSegment::Token(segment) => segment.breaks(),
            ParseSegment::Node(segment) => segment.breaks(),
            ParseSegment::Error(..) => 0,
        }
    }

    #[inline(always)]
    pub fn start_line(&self) -> Line {
        match self {
            ParseSegment::Blank(segment) => segment.start_line(),
            ParseSegment::Token(segment) => segment.start_line(),
            ParseSegment::Node(segment) => segment.start_line(),
            ParseSegment::Error(..) => 0,
        }
    }

    #[inline(always)]
    pub fn end_line(&self) -> Line {
        match self {
            ParseSegment::Blank(segment) => segment.end_line(),
            ParseSegment::Token(segment) => segment.end_line(),
            ParseSegment::Node(segment) => segment.end_line(),
            ParseSegment::Error(..) => 0,
        }
    }

    fn debug<N: Node>(
        &self,
        indent: &str,
        tree: &ParseTree<N>,
        formatter: &mut Formatter<'_>,
    ) -> FmtResult {
        match self {
            ParseSegment::Blank(segment) => segment.debug(indent, tree, formatter),
            ParseSegment::Token(segment) => segment.debug(indent, tree, formatter),
            ParseSegment::Node(segment) => segment.debug(indent, tree, formatter),
            ParseSegment::Error(..) => formatter.write_fmt(format_args!("{indent}<error>")),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ParseBlank {
    pub site_span: SiteSpan,
    pub position_span: PositionSpan,
    pub children: Vec<TokenRef>,
}

impl ParseBlank {
    #[inline(always)]
    pub fn breaks(&self) -> usize {
        self.position_span.end.line - self.position_span.start.line
    }

    #[inline(always)]
    pub fn start_line(&self) -> Line {
        self.position_span.start.line
    }

    #[inline(always)]
    pub fn end_line(&self) -> Line {
        self.position_span.end.line
    }

    #[inline(always)]
    fn debug<N: Node>(
        &self,
        indent: &str,
        tree: &ParseTree<N>,
        formatter: &mut Formatter<'_>,
    ) -> FmtResult {
        let span = self.position_span.display(tree);

        formatter.write_fmt(format_args!("{indent}<blank> [{span}]"))
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ParseToken {
    pub rule: TokenRule,
    pub token_ref: TokenRef,
    pub site_span: SiteSpan,
    pub position_span: PositionSpan,
}

impl ParseToken {
    #[inline(always)]
    pub fn breaks(&self) -> usize {
        self.position_span.end.line - self.position_span.start.line
    }

    #[inline(always)]
    pub fn start_line(&self) -> Line {
        self.position_span.start.line
    }

    #[inline(always)]
    pub fn end_line(&self) -> Line {
        self.position_span.end.line
    }

    #[inline(always)]
    fn debug<N: Node>(
        &self,
        indent: &str,
        tree: &ParseTree<N>,
        formatter: &mut Formatter<'_>,
    ) -> FmtResult {
        let name = <N::Token as Token>::rule_name(self.rule).unwrap_or("?");

        let span = self.position_span.display(tree);

        formatter.write_fmt(format_args!("{indent}${name} [{span}]"))
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ParseNode {
    pub rule: NodeRule,
    pub node_ref: NodeRef,
    pub site_span: SiteSpan,
    pub position_span: PositionSpan,
    pub children: Vec<ParseSegment>,
}

impl ParseNode {
    #[inline(always)]
    pub fn breaks(&self) -> usize {
        self.position_span.end.line - self.position_span.start.line
    }

    #[inline(always)]
    pub fn start_line(&self) -> Line {
        self.position_span.start.line
    }

    #[inline(always)]
    pub fn end_line(&self) -> Line {
        self.position_span.end.line
    }

    #[inline(always)]
    fn debug<N: Node>(
        &self,
        indent: &str,
        tree: &ParseTree<N>,
        formatter: &mut Formatter<'_>,
    ) -> FmtResult {
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

struct ParseTreeBuilder<'a, N: Node> {
    lexis: &'a TokenBuffer<N::Token>,
    site: Site,
    position: Position,
    stack: Vec<ParseNode>,
}

impl<'a, N: Node> Observer for ParseTreeBuilder<'a, N> {
    type Node = N;

    fn read_token(&mut self, token: <Self::Node as Node>::Token, token_ref: TokenRef) {
        let start_site = self.site;
        let start_position = self.position;

        if let Some(length) = token_ref.length(self.lexis) {
            self.site += length;
        };

        let mut is_blank = true;

        if let Some(string) = token_ref.string(self.lexis) {
            for ch in string.chars() {
                match ch {
                    ' ' | '\r' | '\x0c' => {
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
                Some(ParseSegment::Blank(ParseBlank {
                    site_span,
                    position_span,
                    children,
                })) => {
                    site_span.end = end_site;
                    position_span.end = end_position;
                    children.push(token_ref);
                }

                _ => children.push(ParseSegment::Blank(ParseBlank {
                    site_span: start_site..end_site,
                    position_span: start_position..end_position,
                    children: Vec::from([token_ref]),
                })),
            }

            return;
        }

        children.push(ParseSegment::Token(ParseToken {
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
            children: Vec::new(),
        });
    }

    fn leave_rule(&mut self, _rule: NodeRule, _node_ref: NodeRef) {
        let Some(mut segment) = self.stack.pop() else {
            return;
        };

        segment.site_span.end = self.site;
        segment.position_span.end = self.position;

        let Some(parent) = self.stack.last_mut() else {
            self.stack.push(segment);
            return;
        };

        parent.children.push(ParseSegment::Node(segment));
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
            let ParseSegment::Node(sibling) = sibling else {
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
            unsafe { debug_unreachable!("Missing current segment.") };
        };

        current.site_span.start = from_site;
        current.position_span.start = from_position;

        let mut tail = replace(&mut current.children, lifted);

        current.children.append(&mut tail);
    }

    fn parse_error(&mut self, error_ref: ErrorRef) {
        let Some(current) = self.stack.last_mut() else {
            return;
        };

        current.children.push(ParseSegment::Error(error_ref))
    }
}
