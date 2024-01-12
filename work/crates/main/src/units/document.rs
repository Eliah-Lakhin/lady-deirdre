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
    lexis::{
        Length,
        LineIndex,
        Site,
        SiteRef,
        SourceCode,
        ToSpan,
        Token,
        TokenBuffer,
        TokenCount,
        TokenCursor,
        TokenRef,
    },
    std::*,
    syntax::{ErrorRef, Node, NodeRef, SyntaxTree},
    units::{CompilationUnit, ImmutableUnit, MutableUnit, VoidWatch, Watch},
};

pub enum Document<N: Node> {
    Mutable(MutableUnit<N>),
    Immutable(ImmutableUnit<N>),
}

impl<N: Node> Debug for Document<N> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        match self {
            Self::Mutable(unit) => Debug::fmt(unit, formatter),
            Self::Immutable(unit) => Debug::fmt(unit, formatter),
        }
    }
}

impl<N: Node> Display for Document<N> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        match self {
            Self::Mutable(unit) => Display::fmt(unit, formatter),
            Self::Immutable(unit) => Display::fmt(unit, formatter),
        }
    }
}

impl<N: Node> Identifiable for Document<N> {
    #[inline(always)]
    fn id(&self) -> Id {
        match self {
            Self::Mutable(unit) => unit.id(),
            Self::Immutable(unit) => unit.id(),
        }
    }
}

impl<N: Node> Default for Document<N> {
    #[inline(always)]
    fn default() -> Self {
        Self::Mutable(MutableUnit::default())
    }
}

impl<N: Node> From<MutableUnit<N>> for Document<N> {
    #[inline(always)]
    fn from(unit: MutableUnit<N>) -> Self {
        Self::Mutable(unit)
    }
}

impl<N: Node> From<ImmutableUnit<N>> for Document<N> {
    #[inline(always)]
    fn from(unit: ImmutableUnit<N>) -> Self {
        Self::Immutable(unit)
    }
}

impl<N: Node, S: AsRef<str>> From<S> for Document<N> {
    #[inline(always)]
    fn from(string: S) -> Self {
        Self::Mutable(MutableUnit::from(string))
    }
}

impl<N: Node> SourceCode for Document<N> {
    type Token = N::Token;

    type Cursor<'document> = DocumentCursor<'document, N>;

    type CharIterator<'document> = DocumentCharIter<'document, N>;

    #[inline(always)]
    fn chars(&self, span: impl ToSpan) -> Self::CharIterator<'_> {
        match self {
            Self::Mutable(unit) => DocumentCharIter::Mutable(unit.chars(span)),
            Self::Immutable(unit) => DocumentCharIter::Immutable(unit.chars(span)),
        }
    }

    #[inline(always)]
    fn has_chunk(&self, entry: &Entry) -> bool {
        match self {
            Self::Mutable(unit) => unit.has_chunk(entry),
            Self::Immutable(unit) => unit.has_chunk(entry),
        }
    }

    #[inline(always)]
    fn get_token(&self, entry: &Entry) -> Option<Self::Token> {
        match self {
            Self::Mutable(unit) => unit.get_token(entry),
            Self::Immutable(unit) => unit.get_token(entry),
        }
    }

    #[inline(always)]
    fn get_site(&self, entry: &Entry) -> Option<Site> {
        match self {
            Self::Mutable(unit) => unit.get_site(entry),
            Self::Immutable(unit) => unit.get_site(entry),
        }
    }

    #[inline(always)]
    fn get_string(&self, entry: &Entry) -> Option<&str> {
        match self {
            Self::Mutable(unit) => unit.get_string(entry),
            Self::Immutable(unit) => unit.get_string(entry),
        }
    }

    #[inline(always)]
    fn get_length(&self, entry: &Entry) -> Option<Length> {
        match self {
            Self::Mutable(unit) => unit.get_length(entry),
            Self::Immutable(unit) => unit.get_length(entry),
        }
    }

    #[inline(always)]
    fn cursor(&self, span: impl ToSpan) -> Self::Cursor<'_> {
        match self {
            Self::Mutable(unit) => DocumentCursor::Mutable(unit.cursor(span)),
            Self::Immutable(unit) => DocumentCursor::Immutable(unit.cursor(span)),
        }
    }

    #[inline(always)]
    fn length(&self) -> Length {
        match self {
            Self::Mutable(unit) => unit.length(),
            Self::Immutable(unit) => unit.length(),
        }
    }

    #[inline(always)]
    fn tokens(&self) -> TokenCount {
        match self {
            Self::Mutable(unit) => unit.tokens(),
            Self::Immutable(unit) => unit.tokens(),
        }
    }

    #[inline(always)]
    fn lines(&self) -> &LineIndex {
        match self {
            Self::Mutable(unit) => unit.lines(),
            Self::Immutable(unit) => unit.lines(),
        }
    }
}

impl<N: Node> SyntaxTree for Document<N> {
    type Node = N;

    type NodeIterator<'document> = DocumentNodeIter<'document, N>;

    type ErrorIterator<'document> = DocumentErrorIter<'document, N>;

    #[inline(always)]
    fn root_node_ref(&self) -> NodeRef {
        match self {
            Self::Mutable(unit) => unit.root_node_ref(),
            Self::Immutable(unit) => unit.root_node_ref(),
        }
    }

    #[inline(always)]
    fn node_refs(&self) -> Self::NodeIterator<'_> {
        match self {
            Self::Mutable(unit) => DocumentNodeIter::Mutable(unit.node_refs()),
            Self::Immutable(unit) => DocumentNodeIter::Immutable(unit.node_refs()),
        }
    }

    #[inline(always)]
    fn error_refs(&self) -> Self::ErrorIterator<'_> {
        match self {
            Self::Mutable(unit) => DocumentErrorIter::Mutable(unit.error_refs()),
            Self::Immutable(unit) => DocumentErrorIter::Immutable(unit.error_refs()),
        }
    }

    #[inline(always)]
    fn has_node(&self, entry: &Entry) -> bool {
        match self {
            Self::Mutable(unit) => unit.has_node(entry),
            Self::Immutable(unit) => unit.has_node(entry),
        }
    }

    #[inline(always)]
    fn get_node(&self, entry: &Entry) -> Option<&Self::Node> {
        match self {
            Self::Mutable(unit) => unit.get_node(entry),
            Self::Immutable(unit) => unit.get_node(entry),
        }
    }

    #[inline(always)]
    fn get_node_mut(&mut self, entry: &Entry) -> Option<&mut Self::Node> {
        match self {
            Self::Mutable(unit) => unit.get_node_mut(entry),
            Self::Immutable(unit) => unit.get_node_mut(entry),
        }
    }

    #[inline(always)]
    fn has_error(&self, entry: &Entry) -> bool {
        match self {
            Self::Mutable(unit) => unit.has_error(entry),
            Self::Immutable(unit) => unit.has_error(entry),
        }
    }

    #[inline(always)]
    fn get_error(&self, entry: &Entry) -> Option<&<Self::Node as Node>::Error> {
        match self {
            Self::Mutable(unit) => unit.get_error(entry),
            Self::Immutable(unit) => unit.get_error(entry),
        }
    }
}

impl<N: Node> CompilationUnit for Document<N> {
    #[inline(always)]
    fn is_mutable(&self) -> bool {
        match self {
            Self::Mutable(..) => true,
            Self::Immutable(..) => false,
        }
    }

    #[inline(always)]
    fn into_token_buffer(self) -> TokenBuffer<N::Token> {
        match self {
            Self::Mutable(unit) => unit.into_token_buffer(),
            Self::Immutable(unit) => unit.into_token_buffer(),
        }
    }

    #[inline(always)]
    fn into_document(self) -> Document<N> {
        self
    }

    #[inline(always)]
    fn into_mutable_unit(self) -> MutableUnit<N> {
        match self {
            Self::Mutable(unit) => unit,
            Self::Immutable(unit) => unit.into_mutable_unit(),
        }
    }

    #[inline(always)]
    fn into_immutable_unit(self) -> ImmutableUnit<N> {
        match self {
            Self::Mutable(unit) => unit.into_immutable_unit(),
            Self::Immutable(unit) => unit,
        }
    }

    #[inline(always)]
    fn cover(&self, span: impl ToSpan) -> NodeRef {
        match self {
            Self::Mutable(unit) => unit.cover(span),
            Self::Immutable(unit) => unit.cover(span),
        }
    }
}

impl<N: Node> Document<N> {
    #[inline(always)]
    pub fn new_mutable(text: impl Into<TokenBuffer<N::Token>>) -> Self {
        Self::Mutable(MutableUnit::new(text))
    }

    #[inline(always)]
    pub fn new_immutable(text: impl Into<TokenBuffer<N::Token>>) -> Self {
        Self::Immutable(ImmutableUnit::new(text))
    }

    #[inline(always)]
    pub fn is_mutable(&self) -> bool {
        match self {
            Self::Mutable(..) => true,
            Self::Immutable(..) => false,
        }
    }

    #[inline(always)]
    pub fn is_immutable(&self) -> bool {
        match self {
            Self::Mutable(..) => false,
            Self::Immutable(..) => true,
        }
    }

    #[inline(always)]
    pub fn write(&mut self, span: impl ToSpan, text: impl AsRef<str>) {
        self.write_and_watch(span, text, &mut VoidWatch)
    }

    #[inline(always)]
    pub fn write_and_watch(
        &mut self,
        span: impl ToSpan,
        text: impl AsRef<str>,
        watch: &mut impl Watch,
    ) {
        let unit = match self.as_mutable() {
            Some(unit) => unit,
            None => panic!("Specified Document is not mutable."),
        };

        unit.write_and_watch(span, text, watch);
    }

    #[inline(always)]
    pub fn as_mutable(&mut self) -> Option<&mut MutableUnit<N>> {
        match self {
            Self::Mutable(unit) => Some(unit),
            Self::Immutable(..) => None,
        }
    }

    #[inline(always)]
    pub fn into_mutable(self) -> Self {
        match self {
            Self::Mutable(..) => self,
            Self::Immutable(unit) => Self::Mutable(unit.into_mutable_unit()),
        }
    }

    #[inline(always)]
    pub fn into_immutable(self) -> Self {
        match self {
            Self::Mutable(unit) => Self::Immutable(unit.into_immutable_unit()),
            Self::Immutable(..) => self,
        }
    }
}

impl<T: Token> TokenBuffer<T> {
    #[inline(always)]
    pub fn into_document<N>(self) -> Document<N>
    where
        N: Node<Token = T>,
    {
        self.into_mutable_unit().into()
    }
}

pub enum DocumentCursor<'document, N: Node> {
    Mutable(<MutableUnit<N> as SourceCode>::Cursor<'document>),
    Immutable(<ImmutableUnit<N> as SourceCode>::Cursor<'document>),
}

impl<'document, N: Node> Identifiable for DocumentCursor<'document, N> {
    #[inline(always)]
    fn id(&self) -> Id {
        match self {
            Self::Mutable(cursor) => cursor.id(),
            Self::Immutable(cursor) => cursor.id(),
        }
    }
}

impl<'document, N: Node> TokenCursor<'document> for DocumentCursor<'document, N> {
    type Token = N::Token;

    #[inline(always)]
    fn advance(&mut self) -> bool {
        match self {
            Self::Mutable(cursor) => cursor.advance(),
            Self::Immutable(cursor) => cursor.advance(),
        }
    }

    #[inline(always)]
    fn skip(&mut self, distance: TokenCount) {
        match self {
            Self::Mutable(cursor) => cursor.skip(distance),
            Self::Immutable(cursor) => cursor.skip(distance),
        }
    }

    #[inline(always)]
    fn token(&mut self, distance: TokenCount) -> Self::Token {
        match self {
            Self::Mutable(cursor) => cursor.token(distance),
            Self::Immutable(cursor) => cursor.token(distance),
        }
    }

    #[inline(always)]
    fn site(&mut self, distance: TokenCount) -> Option<Site> {
        match self {
            Self::Mutable(cursor) => cursor.site(distance),
            Self::Immutable(cursor) => cursor.site(distance),
        }
    }

    #[inline(always)]
    fn length(&mut self, distance: TokenCount) -> Option<Length> {
        match self {
            Self::Mutable(cursor) => cursor.length(distance),
            Self::Immutable(cursor) => cursor.length(distance),
        }
    }

    #[inline(always)]
    fn string(&mut self, distance: TokenCount) -> Option<&'document str> {
        match self {
            Self::Mutable(cursor) => cursor.string(distance),
            Self::Immutable(cursor) => cursor.string(distance),
        }
    }

    #[inline(always)]
    fn token_ref(&mut self, distance: TokenCount) -> TokenRef {
        match self {
            Self::Mutable(cursor) => cursor.token_ref(distance),
            Self::Immutable(cursor) => cursor.token_ref(distance),
        }
    }

    #[inline(always)]
    fn site_ref(&mut self, distance: TokenCount) -> SiteRef {
        match self {
            Self::Mutable(cursor) => cursor.site_ref(distance),
            Self::Immutable(cursor) => cursor.site_ref(distance),
        }
    }

    #[inline(always)]
    fn end_site_ref(&mut self) -> SiteRef {
        match self {
            Self::Mutable(cursor) => cursor.end_site_ref(),
            Self::Immutable(cursor) => cursor.end_site_ref(),
        }
    }
}

pub enum DocumentCharIter<'document, N: Node> {
    Mutable(<MutableUnit<N> as SourceCode>::CharIterator<'document>),
    Immutable(<ImmutableUnit<N> as SourceCode>::CharIterator<'document>),
}

impl<'document, N: Node> Iterator for DocumentCharIter<'document, N> {
    type Item = char;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Mutable(iterator) => iterator.next(),
            Self::Immutable(iterator) => iterator.next(),
        }
    }
}

impl<'document, N: Node> FusedIterator for DocumentCharIter<'document, N> {}

pub enum DocumentNodeIter<'document, N: Node> {
    Mutable(<MutableUnit<N> as SyntaxTree>::NodeIterator<'document>),
    Immutable(<ImmutableUnit<N> as SyntaxTree>::NodeIterator<'document>),
}

impl<'document, N: Node> Iterator for DocumentNodeIter<'document, N> {
    type Item = NodeRef;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Mutable(iterator) => iterator.next(),
            Self::Immutable(iterator) => iterator.next(),
        }
    }
}

impl<'document, N: Node> FusedIterator for DocumentNodeIter<'document, N> {}

pub enum DocumentErrorIter<'document, N: Node> {
    Mutable(<MutableUnit<N> as SyntaxTree>::ErrorIterator<'document>),
    Immutable(<ImmutableUnit<N> as SyntaxTree>::ErrorIterator<'document>),
}

impl<'document, N: Node> Iterator for DocumentErrorIter<'document, N> {
    type Item = ErrorRef;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Mutable(iterator) => iterator.next(),
            Self::Immutable(iterator) => iterator.next(),
        }
    }
}

impl<'document, N: Node> FusedIterator for DocumentErrorIter<'document, N> {}
