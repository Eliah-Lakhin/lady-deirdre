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
    lexis::{Length, Site, SiteRefSpan, SourceCode, ToSpan, Token, TokenBuffer, TokenCount},
    std::*,
    syntax::{Cluster, ClusterRef, Node, SyntaxBuffer, SyntaxTree},
};

pub struct ImmutableUnit<N: Node> {
    lexis: TokenBuffer<N::Token>,
    syntax: SyntaxBuffer<N>,
}

impl<N: Node> Identifiable for ImmutableUnit<N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.lexis.id()
    }
}

impl<N: Node> Debug for ImmutableUnit<N> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("ImmutableUnit")
            .field("id", &self.lexis.id())
            .field("length", &self.lexis.length())
            .finish_non_exhaustive()
    }
}

impl<N: Node> SourceCode for ImmutableUnit<N> {
    type Token = N::Token;

    type Cursor<'code> = <TokenBuffer<N::Token> as SourceCode>::Cursor<'code>;

    #[inline(always)]
    fn contains_chunk(&self, chunk_ref: &Ref) -> bool {
        self.lexis.contains_chunk(chunk_ref)
    }

    #[inline(always)]
    fn get_token(&self, chunk_ref: &Ref) -> Option<Self::Token> {
        self.lexis.get_token(chunk_ref)
    }

    #[inline(always)]
    fn get_site(&self, chunk_ref: &Ref) -> Option<Site> {
        self.lexis.get_site(chunk_ref)
    }

    #[inline(always)]
    fn get_string(&self, chunk_ref: &Ref) -> Option<&str> {
        self.lexis.get_string(chunk_ref)
    }

    #[inline(always)]
    fn get_length(&self, chunk_ref: &Ref) -> Option<Length> {
        self.lexis.get_length(chunk_ref)
    }

    #[inline(always)]
    fn cursor(&self, span: impl ToSpan) -> Self::Cursor<'_> {
        self.lexis.cursor(span)
    }

    #[inline(always)]
    fn length(&self) -> Length {
        self.lexis.length()
    }

    #[inline(always)]
    fn token_count(&self) -> TokenCount {
        self.lexis.token_count()
    }
}

impl<N: Node> SyntaxTree for ImmutableUnit<N> {
    type Node = N;

    #[inline(always)]
    fn cover(&self, span: impl ToSpan) -> ClusterRef {
        self.syntax.cover(span)
    }

    #[inline(always)]
    fn contains_cluster(&self, cluster_ref: &Ref) -> bool {
        self.syntax.contains_cluster(cluster_ref)
    }

    #[inline(always)]
    fn get_cluster(&self, cluster_ref: &Ref) -> Option<&Cluster<Self::Node>> {
        self.syntax.get_cluster(cluster_ref)
    }

    #[inline(always)]
    fn get_cluster_mut(&mut self, cluster_ref: &Ref) -> Option<&mut Cluster<Self::Node>> {
        self.syntax.get_cluster_mut(cluster_ref)
    }

    #[inline(always)]
    fn get_cluster_span(&self, cluster_ref: &Ref) -> SiteRefSpan {
        self.syntax.get_cluster_span(cluster_ref)
    }

    #[inline(always)]
    fn get_previous_cluster(&self, cluster_ref: &Ref) -> Ref {
        self.syntax.get_previous_cluster(cluster_ref)
    }

    #[inline(always)]
    fn get_next_cluster(&self, cluster_ref: &Ref) -> Ref {
        self.syntax.get_next_cluster(cluster_ref)
    }

    #[inline(always)]
    fn remove_cluster(&mut self, cluster_ref: &Ref) -> Option<Cluster<Self::Node>> {
        self.syntax.remove_cluster(cluster_ref)
    }
}

impl<N, S> From<S> for ImmutableUnit<N>
where
    N: Node,
    S: Borrow<str>,
{
    #[inline(always)]
    fn from(string: S) -> Self {
        let mut lexis = TokenBuffer::<N::Token>::default();

        lexis.append(string.borrow());

        let syntax = SyntaxBuffer::new(lexis.id(), lexis.cursor(..));

        Self { lexis, syntax }
    }
}

impl<T: Token> TokenBuffer<T> {
    #[inline(always)]
    pub fn into_immutable_unit<N>(mut self) -> ImmutableUnit<N>
    where
        N: Node<Token = T>,
    {
        self.reset_id();

        let syntax = SyntaxBuffer::new(self.id(), self.cursor(..));

        ImmutableUnit {
            lexis: self,
            syntax,
        }
    }
}

impl<N: Node> CompilationUnit for ImmutableUnit<N> {
    #[inline(always)]
    fn is_mutable(&self) -> bool {
        false
    }

    #[inline(always)]
    fn into_token_buffer(mut self) -> TokenBuffer<N::Token> {
        self.lexis.reset_id();

        self.lexis
    }

    #[inline(always)]
    fn into_immutable_unit(self) -> ImmutableUnit<N> {
        self
    }
}
