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
    arena::{Entry, Identifiable},
    compiler::{ImmutableUnit, MutableUnit},
    format::Delimited,
    lexis::{
        Chunk,
        Length,
        Site,
        SiteRefSpan,
        SiteSpan,
        SourceCode,
        ToSpan,
        Token,
        TokenBuffer,
        TokenCount,
        TokenCursor,
    },
    std::*,
    syntax::{Child, Cluster, ClusterRef, Node, NodeRef, PolyRef, PolyVariant, SyntaxTree},
    Document,
};

pub trait CompilationUnit:
    SourceCode<Token = <<Self as SyntaxTree>::Node as Node>::Token> + SyntaxTree
{
    fn is_mutable(&self) -> bool;

    #[inline(always)]
    fn is_immutable(&self) -> bool {
        !self.is_mutable()
    }

    fn into_token_buffer(self) -> TokenBuffer<<Self as SourceCode>::Token>;

    #[inline(always)]
    fn into_document(self) -> Document<<Self as SyntaxTree>::Node>
    where
        Self: Sized,
    {
        match self.is_mutable() {
            true => Document::Mutable(self.into_mutable_unit()),
            false => Document::Immutable(self.into_immutable_unit()),
        }
    }

    #[inline(always)]
    fn into_mutable_unit(self) -> MutableUnit<<Self as SyntaxTree>::Node>
    where
        Self: Sized,
    {
        self.into_token_buffer().into_mutable_unit()
    }

    #[inline(always)]
    fn into_immutable_unit(self) -> ImmutableUnit<<Self as SyntaxTree>::Node>
    where
        Self: Sized,
    {
        self.into_token_buffer().into_immutable_unit()
    }

    #[inline(always)]
    fn cover(&self, span: impl ToSpan) -> NodeRef
    where
        Self: Sized,
    {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),

            Some(span) => span,
        };

        let root = self.root_node_ref();

        match NodeCoverage::cover(self, &root, &span) {
            NodeCoverage::Fit(result) => result,
            _ => root,
        }
    }

    #[inline(always)]
    fn debug_tree(
        &self,
        poly_ref: &impl PolyRef,
    ) -> DebugTree<<Self as SyntaxTree>::Node, <Self as SourceCode>::Cursor<'_>, Self>
    where
        Self: Sized,
    {
        DebugTree {
            unit: self,
            variant: poly_ref.as_variant(),
        }
    }
}

pub trait Lexis: Identifiable {
    type Lexis: SourceCode;

    fn lexis(&self) -> &Self::Lexis;
}

impl<F: Lexis> SourceCode for F {
    type Token = <F::Lexis as SourceCode>::Token;

    type Cursor<'code> = <F::Lexis as SourceCode>::Cursor<'code>
        where Self: 'code;

    #[inline(always)]
    fn has_chunk(&self, chunk_entry: &Entry) -> bool {
        self.lexis().has_chunk(chunk_entry)
    }

    #[inline(always)]
    fn get_token(&self, chunk_entry: &Entry) -> Option<Self::Token> {
        self.lexis().get_token(chunk_entry)
    }

    #[inline(always)]
    fn get_site(&self, chunk_entry: &Entry) -> Option<Site> {
        self.lexis().get_site(chunk_entry)
    }

    #[inline(always)]
    fn get_string(&self, chunk_entry: &Entry) -> Option<&str> {
        self.lexis().get_string(chunk_entry)
    }

    #[inline(always)]
    fn get_length(&self, chunk_entry: &Entry) -> Option<Length> {
        self.lexis().get_length(chunk_entry)
    }

    #[inline(always)]
    fn cursor(&self, span: impl ToSpan) -> Self::Cursor<'_> {
        self.lexis().cursor(span)
    }

    #[inline(always)]
    fn length(&self) -> Length {
        self.lexis().length()
    }

    #[inline(always)]
    fn token_count(&self) -> TokenCount {
        self.lexis().token_count()
    }
}

pub trait Syntax: Identifiable {
    type Syntax: SyntaxTree;

    fn syntax(&self) -> &Self::Syntax;

    fn syntax_mut(&mut self) -> &mut Self::Syntax;
}

impl<F: Syntax> SyntaxTree for F {
    type Node = <F::Syntax as SyntaxTree>::Node;

    #[inline(always)]
    fn has_cluster(&self, cluster_entry: &Entry) -> bool {
        self.syntax().has_cluster(cluster_entry)
    }

    #[inline(always)]
    fn get_cluster(&self, cluster_entry: &Entry) -> Option<&Cluster<Self::Node>> {
        self.syntax().get_cluster(cluster_entry)
    }

    #[inline(always)]
    fn get_cluster_mut(&mut self, cluster_entry: &Entry) -> Option<&mut Cluster<Self::Node>> {
        self.syntax_mut().get_cluster_mut(cluster_entry)
    }

    #[inline(always)]
    fn get_previous_cluster(&self, cluster_entry: &Entry) -> Entry {
        self.syntax().get_previous_cluster(cluster_entry)
    }

    #[inline(always)]
    fn get_next_cluster(&self, cluster_entry: &Entry) -> Entry {
        self.syntax().get_next_cluster(cluster_entry)
    }

    #[inline(always)]
    fn remove_cluster(&mut self, cluster_entry: &Entry) -> Option<Cluster<Self::Node>> {
        self.syntax_mut().remove_cluster(cluster_entry)
    }
}

pub struct DebugTree<
    'unit,
    N: Node,
    C: TokenCursor<'unit>,
    U: CompilationUnit<Cursor<'unit> = C, Node = N>,
> {
    unit: &'unit U,
    variant: PolyVariant,
}

impl<'unit, N, C, U> Debug for DebugTree<'unit, N, C, U>
where
    N: Node,
    C: TokenCursor<'unit>,
    U: CompilationUnit<Cursor<'unit> = C, Node = N>,
{
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self, formatter)
    }
}

impl<'unit, N, C, U> Display for DebugTree<'unit, N, C, U>
where
    N: Node,
    C: TokenCursor<'unit>,
    U: CompilationUnit<Cursor<'unit> = C, Node = N>,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match &self.variant {
            PolyVariant::Token(variant) => {
                let chunk: Chunk<U::Token> = match variant.chunk(self.unit) {
                    None => return Debug::fmt(variant, formatter),
                    Some(chunk) => chunk,
                };

                let rule = chunk.token.rule();
                let name = <U::Token as Token>::name(rule).unwrap_or("TokenRef");

                let mut debug_struct = formatter
                    .debug_struct(&format!("${name}(chunk_entry: {:?})", variant.chunk_entry));

                debug_struct.field("string", &chunk.string);
                debug_struct.field("length", &chunk.length);
                debug_struct.field("site_span", &chunk.to_site_span(self.unit));
                debug_struct.field("position_span", &chunk.display(self.unit));

                debug_struct.finish()
            }

            PolyVariant::Node(variant) => {
                let node: &N = match variant.deref(self.unit) {
                    None => return Debug::fmt(variant, formatter),
                    Some(node) => node,
                };

                let rule = node.rule();
                let name = N::name(rule).unwrap_or("NodeRef");

                let alternate = formatter.alternate();

                let mut debug_struct = formatter.debug_struct(&format!(
                    "{name}(cluster_entry: {:?}, node_entry: {:?})",
                    variant.cluster_entry, variant.node_entry,
                ));

                for (key, value) in node.children().entries() {
                    match value {
                        Child::Token(child) => match alternate {
                            true => debug_struct
                                .field(key, &format_args!("{:#}", self.unit.debug_tree(child))),
                            false => debug_struct
                                .field(key, &format_args!("{}", self.unit.debug_tree(child))),
                        },

                        Child::TokenSeq(child) => {
                            let child = child
                                .iter()
                                .map(|poly_ref| self.unit.debug_tree(poly_ref))
                                .collect::<Vec<_>>();

                            debug_struct.field(key, &child)
                        }

                        Child::Node(child) => match alternate {
                            true => debug_struct
                                .field(key, &format_args!("{:#}", self.unit.debug_tree(child))),
                            false => debug_struct
                                .field(key, &format_args!("{}", self.unit.debug_tree(child))),
                        },

                        Child::NodeSeq(child) => {
                            let child = child
                                .iter()
                                .map(|poly_ref| self.unit.debug_tree(poly_ref))
                                .collect::<Vec<_>>();

                            debug_struct.field(key, &child)
                        }
                    };
                }

                debug_struct.finish()
            }
        }
    }
}

#[derive(Debug)]
pub(super) enum NodeCoverage {
    Nil,
    Fit(NodeRef),
    Misfit(Site),
}

impl NodeCoverage {
    pub(super) fn cover<
        'unit,
        N: Node,
        C: TokenCursor<'unit>,
        U: CompilationUnit<Cursor<'unit> = C, Node = N>,
    >(
        unit: &'unit U,
        node_ref: &NodeRef,
        span: &SiteSpan,
    ) -> Self {
        let node: &N = match node_ref.deref(unit) {
            None => return Self::Nil,
            Some(node) => node,
        };

        let children = node.children();

        let node_span = match children.span(unit) {
            None => return Self::Nil,
            Some(span) => span,
        };

        if node_span.start > span.start || node_span.end < span.end {
            return Self::Misfit(node_span.start);
        }

        for child in children.nodes() {
            match Self::cover(unit, child, span) {
                Self::Nil => continue,
                Self::Misfit(start) => {
                    if start > span.start {
                        break;
                    }
                }
                other => return other,
            }
        }

        Self::Fit(*node_ref)
    }
}
