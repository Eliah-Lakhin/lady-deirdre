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

use std::{borrow::Cow, iter::FusedIterator, marker::PhantomData};

use crate::{
    arena::{Entry, Identifiable},
    lexis::{Chunk, Length, LineIndex, Site, SiteRef, ToSpan, Token, TokenCount, TokenCursor},
};

/// An object that provides access to the source code text and the lexical
/// structure of a compilation unit.
///
/// The lexical structure is a sequence of [tokens](crate::lexis::Token) that
/// fully covers the source code text exhaustively and without overlaps.
///
/// This trait provides a low-level interface to access tokens and their
/// metadata by the [versioned index](Entry) from the compilation unit.
///
/// A higher-level referential object, [TokenRef](crate::lexis::TokenRef),
/// offers a more convenient interface to access these objects from
/// the SourceCode.
///
/// Additionally, the SourceCode interface provides functions to access
/// substrings of the source code text, to iterate through individual characters
/// in range, and to iterate through the tokens and their metadata in range.
pub trait SourceCode: Identifiable {
    /// Specifies the type of the source code token and the lexical scanner
    /// of a programming language through the [Token::scan] function.
    type Token: Token;

    /// Specifies the type of the [token cursor](TokenCursor) that iterates
    /// through the streams of tokens of this source code type.
    type Cursor<'code>: TokenCursor<'code, Token = Self::Token>
    where
        Self: 'code;

    /// Specifies the type of the iterator that iterates through the unicode
    /// characters of the source code text substrings.
    type CharIterator<'code>: Iterator<Item = char> + FusedIterator + 'code
    where
        Self: 'code;

    /// Returns an iterator of the source code tokens [metadata](Chunk)
    /// in the specified `span`.
    ///
    /// The iterator will yield all token chunks if the `span` intersects with
    /// the token spans, including the span bounds.
    ///
    /// For example, in the text `FooBarBaz` with three tokens, the span `3..6`
    /// will iterate through all three tokens because this span covers
    /// the token `Bar` and intersects with the tokens `Foo` and `Baz`
    /// by their bounds. The span `3..3` would iterate through the `Foo` and
    /// `Bar` tokens because this span intersects with these two token bounds.
    /// The span `4..5` would yield the `Bar` token only because this span
    /// intersects with the token's span.
    ///
    /// **Panic**
    ///
    /// This function may panic if the specified `span` is not
    /// [valid](ToSpan::is_valid_span) for this source code.
    #[inline(always)]
    fn chunks(&self, span: impl ToSpan) -> ChunkIter<'_, Self::Cursor<'_>>
    where
        Self: Sized,
    {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),
            Some(span) => span,
        };

        let cursor = self.cursor(span.clone());

        ChunkIter {
            cursor,
            _code_lifetime: PhantomData::default(),
        }
    }

    /// Returns an iterator that iterates over the [Unicode chars](char) of the
    /// source code text substring in the specified `span`.
    ///
    /// For example, in the text `FooBarBaz` with three tokens, the span `2..6`
    /// will iterate the characters of the `oBar` substring.
    ///
    /// **Panic**
    ///
    /// This function may panic if the specified `span` is not
    /// [valid](ToSpan::is_valid_span) for this source code.
    fn chars(&self, span: impl ToSpan) -> Self::CharIterator<'_>;

    /// Returns a borrowed or an owned substring of the source code text in
    /// the specified `span`.
    ///
    /// For example, in the text `FooBarBaz` with three tokens, the span `2..6`
    /// will yield an `oBar` substring.
    ///
    /// The decision about the returning string ownership is implementation
    /// dependent.
    ///
    /// **Panic**
    ///
    /// This function may panic if the specified `span` is not
    /// [valid](ToSpan::is_valid_span) for this source code.
    fn substring(&self, span: impl ToSpan) -> Cow<str>
    where
        Self: Sized,
    {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),
            Some(span) => span,
        };

        let mut cursor = self.cursor(span.clone());

        if cursor.site(0) == Some(span.start) && cursor.site(1) == Some(span.end) {
            if let Some(string) = cursor.string(0) {
                return Cow::Borrowed(string);
            }
        }

        Cow::from(self.chars(span).collect::<String>())
    }

    /// Checks if the token referred to by the versioned index exists in this
    /// source code.
    fn has_chunk(&self, entry: &Entry) -> bool;

    /// Returns a copy of the token referred to by the versioned index.
    ///
    /// If the index parameter `entry` is not valid, returns None.
    fn get_token(&self, entry: &Entry) -> Option<Self::Token>;

    /// Returns a start site of the token referred to by the versioned index.
    ///
    /// If the index parameter `entry` is not valid, returns None.
    fn get_site(&self, entry: &Entry) -> Option<Site>;

    /// Returns a reference to the source code text substring covered by
    /// the token referred to by the versioned index.
    ///
    /// If the index parameter `entry` is not valid, returns None.
    fn get_string(&self, entry: &Entry) -> Option<&str>;

    /// Returns the [length](Length) of the source code text substring covered
    /// by the token referred to by the versioned index.
    ///
    /// If the index parameter `entry` is not valid, returns None.
    fn get_length(&self, entry: &Entry) -> Option<Length>;

    /// Returns a cursor of the source code tokens stream in
    /// the specified `span`.
    ///
    /// The token stream includes a source code token if the `span` intersects
    /// with the span of this token, including the span bounds.
    ///
    /// The returning object does not implement the [Iterator] interface
    /// but provides the capabilities for manual control over the iteration
    /// process, including the capabilities to look ahead of the token stream.
    ///
    /// If you need just a normal Iterator over the token chunks,
    /// use the [chunks](Self::chunks) function instead.
    ///
    /// **Panic**
    ///
    /// This function may panic if the specified `span` is not
    /// [valid](ToSpan::is_valid_span) for this source code.
    fn cursor(&self, span: impl ToSpan) -> Self::Cursor<'_>;

    /// Returns a [SiteRef] that points to the end of this source code.
    #[inline(always)]
    fn end_site_ref(&self) -> SiteRef {
        SiteRef::end_of(self.id())
    }

    /// Returns the total number of Unicode characters in this source code text.
    fn length(&self) -> Length;

    /// Returns the total number of tokens in this source code.
    fn tokens(&self) -> TokenCount;

    /// Provides access to the [line index](LineIndex) of this source code.
    ///
    /// From this object, you can convert char sites back and forth to their
    /// line indices, and to reveal the total number of lines in this source
    /// code.
    fn lines(&self) -> &LineIndex;

    /// Returns true if the source code text is empty.
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.length() == 0
    }
}

/// An iterator over the [SourceCode] tokens and their metadata.
///
/// This object is created by the [SourceCode::chunks] function.
#[repr(transparent)]
pub struct ChunkIter<'code, C: TokenCursor<'code>> {
    cursor: C,
    _code_lifetime: PhantomData<&'code ()>,
}

impl<'code, C: TokenCursor<'code>> Iterator for ChunkIter<'code, C> {
    type Item = Chunk<'code, <C as TokenCursor<'code>>::Token>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let token = self.cursor.token(0);
        let site = self.cursor.site(0)?;
        let length = self.cursor.length(0)?;
        let string = self.cursor.string(0)?;

        if !self.cursor.advance() {
            return None;
        }

        Some(Self::Item {
            token,
            site,
            length,
            string,
        })
    }
}

impl<'code, C: TokenCursor<'code>> FusedIterator for ChunkIter<'code, C> {}
