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
    lexis::{ByteIndex, ChunkRef, Site, SiteSpan, SourceCode, ToSpan, TokenCursor},
    std::*,
};

/// A high-level extension interface to inspect lexical data of the source code.
///
/// This trait is auto-implemented for any [SourceCode](crate::lexis::SourceCode) object.
/// An API user normally does not need to implement it manually.
///
/// The interface provides three high-level helper functions to access the source code lecical
/// structure:
///  - A [chunks](CodeContent::chunks) function that returns an iterator over the
///    [ChunkRef](crate::lexis::ChunkRef) token metadata objects "touched" by specified
///    [Span](crate::lexis::ToSpan).
///  - A [chars](CodeContent::chars) function that returns an iterator over Unicode characters
///    of the source code text in specified Span.
///  - A [substring](CodeContent::substring) that returns a clone of the source code substring
///    in specified Span.
pub trait CodeContent: SourceCode {
    /// An iterator over the [ChunkRef](crate::lexis::ChunkRef) token metadata objects "touched"
    /// by specified [Span](crate::lexis::ToSpan),
    type ChunkIterator<'code>: Iterator<Item = ChunkRef<'code, <Self as SourceCode>::Token>>
        + FusedIterator
        + Identifiable
        + 'code
    where
        Self: 'code;

    /// An iterator over the Unicode characters of the source code text in specified
    /// [Span](crate::lexis::ToSpan).
    type CharIterator<'code>: Iterator<Item = char> + FusedIterator + Identifiable + 'code
    where
        Self: 'code;

    /// Returns an iterator over the [ChunkRef](crate::lexis::ChunkRef) token metadata objects "touched"
    /// by specified `span`.
    ///
    /// Span "touching" means such tokens that their substring characters lie inside, intersect
    /// with, or adjacent to this [Span](crate::lexis::ToSpan) object.
    ///
    /// ```rust
    /// use lady_deirdre::lexis::{TokenBuffer, CodeContent, SimpleToken, ChunkRef};
    ///
    /// let buf = TokenBuffer::<SimpleToken>::from("foo bar baz");
    ///
    /// assert_eq!(
    ///     buf
    ///         // Second whitespace token " " is adjacent to site 4.
    ///         // Third identifier token "bar" covered by `4..7` span.
    ///         // Fourth whitespace token " " is adjacent to site 7.
    ///         .chunks(4..7)
    ///         .map(|chunk_ref: ChunkRef<'_, SimpleToken>| (chunk_ref.token, chunk_ref.string))
    ///         .collect::<Vec<_>>(),
    ///     vec![
    ///         (&SimpleToken::Whitespace, " "),
    ///         (&SimpleToken::Identifier, "bar"),
    ///         (&SimpleToken::Whitespace, " "),
    ///     ],
    /// );
    /// ```
    fn chunks(&self, span: impl ToSpan) -> Self::ChunkIterator<'_>;

    /// Returns an iterator over the Unicode characters of the source code text in specified
    /// [span](crate::lexis::ToSpan).
    ///
    /// ```rust
    /// use lady_deirdre::lexis::{TokenBuffer, CodeContent, SimpleToken, ChunkRef};
    ///
    /// let buf = TokenBuffer::<SimpleToken>::from("foo bar baz");
    ///
    /// assert_eq!(
    ///     buf.chars(4..7).map(|ch| ch.to_string().to_uppercase()).collect::<Vec<_>>().join("."),
    ///     "B.A.R",
    /// );
    /// ```
    fn chars(&self, span: impl ToSpan) -> Self::CharIterator<'_>;

    /// Returns a substring of the source code text in [span](crate::lexis::ToSpan).
    ///
    /// ```rust
    /// use lady_deirdre::lexis::{TokenBuffer, CodeContent, SimpleToken, Position};
    ///
    /// let mut buf = TokenBuffer::<SimpleToken>::default();
    ///
    /// buf.append("First line\n");
    /// buf.append("Second line\n");
    /// buf.append("Third line\n");
    ///
    /// assert_eq!(
    ///     buf.substring(Position::new(1, 7)..=Position::new(3, 5)),
    ///     "line\nSecond line\nThird",
    /// );
    /// ```
    #[inline(always)]
    fn substring(&self, span: impl ToSpan) -> String {
        self.chars(span).collect()
    }
}

impl<C: SourceCode> CodeContent for C {
    type ChunkIterator<'code> = ChunkIterator<'code, Self::Cursor<'code>>
    where
        Self: 'code;

    type CharIterator<'code> = CharIterator<'code, Self::Cursor<'code>>
    where
        Self: 'code;

    #[inline(always)]
    fn chunks(&self, span: impl ToSpan) -> Self::ChunkIterator<'_> {
        let span = match span.to_span(self) {
            None => panic!("Specified span is invalid."),
            Some(span) => span,
        };

        let cursor = self.cursor(span.clone());

        Self::ChunkIterator {
            cursor,
            _code_lifetime: PhantomData::default(),
        }
    }

    #[inline(always)]
    fn chars(&self, span: impl ToSpan) -> Self::CharIterator<'_> {
        let span = match span.to_span(self) {
            None => panic!("Specified span is invalid."),
            Some(span) => span,
        };

        let cursor = self.cursor(span.clone());

        Self::CharIterator {
            span,
            cursor,
            site: 0,
            byte: 0,
            _code_lifetime: PhantomData::default(),
        }
    }
}

#[repr(transparent)]
pub struct ChunkIterator<'code, C: TokenCursor<'code>> {
    cursor: C,
    _code_lifetime: PhantomData<&'code ()>,
}

impl<'code, C: TokenCursor<'code>> Identifiable for ChunkIterator<'code, C> {
    #[inline(always)]
    fn id(&self) -> &Id {
        self.cursor.id()
    }
}

impl<'code, C: TokenCursor<'code>> Iterator for ChunkIterator<'code, C> {
    type Item = ChunkRef<'code, <C as TokenCursor<'code>>::Token>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let token = self.cursor.token(0)?;
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

impl<'code, C: TokenCursor<'code>> FusedIterator for ChunkIterator<'code, C> {}

pub struct CharIterator<'code, C: TokenCursor<'code>> {
    span: SiteSpan,
    cursor: C,
    site: Site,
    byte: ByteIndex,
    _code_lifetime: PhantomData<&'code ()>,
}

impl<'code, C: TokenCursor<'code>> Identifiable for CharIterator<'code, C> {
    #[inline(always)]
    fn id(&self) -> &Id {
        self.cursor.id()
    }
}

impl<'code, C: TokenCursor<'code>> Iterator for CharIterator<'code, C> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let site = self.cursor.site(0)?;

            if self.site + site >= self.span.end {
                return None;
            }

            let length = self.cursor.length(0)?;

            if site + length < self.span.start || self.site >= length {
                let _ = self.cursor.advance();
                self.site = 0;
                self.byte = 0;
                continue;
            }

            let string = self.cursor.string(0)?;

            let character = unsafe {
                string
                    .get_unchecked(self.byte..)
                    .chars()
                    .next()
                    .unwrap_unchecked()
            };

            self.site += 1;
            self.byte += character.len_utf8();

            if self.site + site <= self.span.start {
                continue;
            }

            return Some(character);
        }
    }
}

impl<'code, C: TokenCursor<'code>> FusedIterator for CharIterator<'code, C> {}
