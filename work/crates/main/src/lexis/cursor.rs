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

use crate::{
    arena::{Entry, EntryIndex, Id, Identifiable},
    lexis::{
        Length,
        Site,
        SiteRef,
        SiteSpan,
        SourceCode,
        Token,
        TokenBuffer,
        TokenCount,
        TokenRef,
    },
    syntax::PolyRef,
};

/// A cursor that iterates through the lexical tokens and their metadata
/// within the tokens stream.
///
/// Most functions of this object (e.g., the [token_ref](TokenCursor::token_ref)
/// function) have a `distance` parameter that allows looking ahead to the
/// token stream without advancing the cursor.
///
/// The `distance` index is zero-based, where the zero value means the current
/// token of where the cursor currently points to.
pub trait TokenCursor<'code>: Identifiable {
    /// Specifies a [Token] type of the underlying source code.
    type Token: Token;

    /// Advances the cursor to the next token in the token stream.
    ///
    /// Returns false if the cursor already at the end of the stream.
    fn advance(&mut self) -> bool;

    /// Skips the next `distance` tokens.
    ///
    /// This function behaves similarly to [advancing](Self::advance) each
    /// token one by one when the number of tokens that need to be skipped
    /// is known upfront.
    ///
    /// If the `distance` equals zero, this function does nothing.
    fn skip(&mut self, distance: TokenCount);

    /// Returns a copy of the [Token] in the token stream where the cursor is
    /// currently pointing, or looks ahead to the `distance` number of tokens
    /// if the distance is greater than zero.
    ///
    /// If the TokenCursor has already reached the end of the token stream, or
    /// the lookahead `distance` exceeds the token stream's end bound, this
    /// function returns [EOI](Token::eoi) token.
    fn token(&mut self, distance: TokenCount) -> Self::Token;

    /// Returns a start [site](Site) of the token in the token stream where
    /// the cursor is currently pointing, or looks ahead to the `distance`
    /// number of tokens if the distance is greater than zero.
    ///
    /// If the TokenCursor has already reached the end of the token stream, or
    /// the lookahead `distance` exceeds the token stream's end bound, this
    /// function returns None.
    fn site(&mut self, distance: TokenCount) -> Option<Site>;

    /// Returns a [length](Length) of the token in the token stream where
    /// the cursor is currently pointing, or looks ahead to the `distance`
    /// number of tokens if the distance is greater than zero.
    ///
    /// If the TokenCursor has already reached the end of the token stream, or
    /// the lookahead `distance` exceeds the token stream's end bound, this
    /// function returns None.
    fn length(&mut self, distance: TokenCount) -> Option<Length>;

    /// Returns a string of the token in the token stream where the cursor
    /// is currently pointing, or looks ahead to the `distance` number of tokens
    /// if the distance is greater than zero.
    ///
    /// If the TokenCursor has already reached the end of the token stream, or
    /// the lookahead `distance` exceeds the token stream's end bound, this
    /// function returns None.
    fn string(&mut self, distance: TokenCount) -> Option<&'code str>;

    /// Returns a [TokenRef] reference of the token in the token stream where
    /// the cursor is currently pointing, or looks ahead to the `distance`
    /// number of tokens if the distance is greater than zero.
    ///
    /// If the TokenCursor has already reached the end of the token stream, or
    /// the lookahead `distance` exceeds the token stream's end bound, this
    /// function returns [nil](TokenRef::nil).
    fn token_ref(&mut self, distance: TokenCount) -> TokenRef;

    /// Returns a [SiteRef] that points to the beginning of the token
    /// in the token stream where the cursor is currently pointing, or looks
    /// ahead to the `distance` number of tokens if the distance is greater
    /// than zero.
    ///
    /// If the TokenCursor has already reached the end of the token stream, or
    /// the lookahead `distance` exceeds the token stream's end bound, this
    /// function returns the [end_site_ref](Self::end_site_ref) value.
    fn site_ref(&mut self, distance: TokenCount) -> SiteRef;

    /// Returns a [SiteRef] that points to the end of the stream.
    fn end_site_ref(&mut self) -> SiteRef;
}

pub struct TokenBufferCursor<'code, T: Token> {
    buffer: &'code TokenBuffer<T>,
    next: EntryIndex,
    end_site: Site,
    end_site_ref: SiteRef,
}

impl<'code, T: Token> Identifiable for TokenBufferCursor<'code, T> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.buffer.id()
    }
}

impl<'code, T: Token> TokenCursor<'code> for TokenBufferCursor<'code, T> {
    type Token = T;

    #[inline]
    fn advance(&mut self) -> bool {
        if self.next >= self.buffer.tokens() {
            return false;
        }

        let next_site = unsafe { *self.buffer.sites.get_unchecked(self.next) };

        if next_site > self.end_site {
            return false;
        }

        self.next += 1;

        true
    }

    #[inline(always)]
    fn skip(&mut self, distance: TokenCount) {
        self.next = (self.next + distance).min(self.buffer.tokens());
    }

    #[inline]
    fn token(&mut self, mut distance: TokenCount) -> Self::Token {
        distance += self.next;

        if distance >= self.buffer.tokens() {
            return <Self::Token as Token>::eoi();
        }

        let peek_site = unsafe { *self.buffer.sites.get_unchecked(distance) };

        if peek_site > self.end_site {
            return <Self::Token as Token>::eoi();
        }

        *unsafe { self.buffer.tokens.get_unchecked(distance) }
    }

    #[inline]
    fn site(&mut self, mut distance: TokenCount) -> Option<Site> {
        distance += self.next;

        if distance >= self.buffer.tokens() {
            return None;
        }

        let peek_site = unsafe { *self.buffer.sites.get_unchecked(distance) };

        if peek_site > self.end_site {
            return None;
        }

        Some(peek_site)
    }

    #[inline]
    fn length(&mut self, mut distance: TokenCount) -> Option<Length> {
        distance += self.next;

        if distance >= self.buffer.tokens() {
            return None;
        }

        let peek_site = unsafe { *self.buffer.sites.get_unchecked(distance) };

        if peek_site > self.end_site {
            return None;
        }

        Some(*unsafe { self.buffer.spans.get_unchecked(distance) })
    }

    #[inline]
    fn string(&mut self, mut distance: TokenCount) -> Option<&'code str> {
        distance += self.next;

        if distance >= self.buffer.tokens() {
            return None;
        }

        let peek_site = unsafe { *self.buffer.sites.get_unchecked(distance) };

        if peek_site > self.end_site {
            return None;
        }

        let text = self.buffer.text.as_str();

        let start = *unsafe { self.buffer.indices.get_unchecked(distance) };
        let end = self
            .buffer
            .indices
            .get(distance + 1)
            .copied()
            .unwrap_or(text.len());

        Some(unsafe { text.get_unchecked(start..end) })
    }

    #[inline]
    fn token_ref(&mut self, mut distance: TokenCount) -> TokenRef {
        distance += self.next;

        if distance >= self.buffer.tokens() {
            return TokenRef::nil();
        }

        let peek_site = unsafe { *self.buffer.sites.get_unchecked(distance) };

        if peek_site > self.end_site {
            return TokenRef::nil();
        }

        TokenRef {
            id: self.buffer.id(),
            entry: Entry {
                index: distance,
                version: 0,
            },
        }
    }

    #[inline(always)]
    fn site_ref(&mut self, distance: TokenCount) -> SiteRef {
        let token_ref = self.token_ref(distance);

        if token_ref.is_nil() {
            return self.end_site_ref();
        }

        token_ref.site_ref()
    }

    fn end_site_ref(&mut self) -> SiteRef {
        if self.end_site_ref.is_nil() {
            let mut index = self.next;

            loop {
                if index >= self.buffer.tokens() {
                    self.end_site_ref = SiteRef::end_of(self.buffer.id());
                    break;
                }

                let peek_site = unsafe { *self.buffer.sites.get_unchecked(index) };

                if peek_site > self.end_site {
                    self.end_site_ref = TokenRef {
                        id: self.buffer.id(),
                        entry: Entry { index, version: 0 },
                    }
                    .site_ref();
                    break;
                }

                index += 1;
            }
        }

        self.end_site_ref
    }
}

impl<'code, T: Token> TokenBufferCursor<'code, T> {
    pub(super) fn new(buffer: &'code TokenBuffer<T>, span: SiteSpan) -> Self {
        let mut next = 0;

        while next < buffer.tokens() {
            let site = unsafe { *buffer.sites.get_unchecked(next) };
            let length = unsafe { *buffer.spans.get_unchecked(next) };

            if site + length < span.start {
                next += 1;
                continue;
            }

            break;
        }

        let end_site_ref = match span.end >= buffer.length() {
            true => SiteRef::end_of(buffer.id()),
            false => SiteRef::nil(),
        };

        Self {
            buffer,
            next,
            end_site: span.end,
            end_site_ref,
        }
    }
}
