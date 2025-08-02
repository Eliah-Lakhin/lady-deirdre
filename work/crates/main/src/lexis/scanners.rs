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
    collections::VecDeque,
    fmt::{Debug, Formatter},
    iter::FusedIterator,
    mem::take,
};

use crate::{
    arena::{Id, Identifiable},
    lexis::{
        session::Cursor,
        ByteIndex,
        Chunk,
        Length,
        LexisSession,
        Site,
        SiteRef,
        Token,
        TokenCount,
        TokenCursor,
        TokenRef,
    },
    report::{ld_assert, ld_unreachable, system_panic},
};

/// A stateless scanner over a string with token lookahead capabilities.
///
/// The object is created by calling the [stream](Scannable::stream)
/// function on any input string (`String` or `&str`).
///
/// TokenStream is an Iterator that yields scanned [Chunks](Chunk) until
/// the end of the scanned input string.
///
/// Additionally, TokenStream implements the [TokenCursor] trait, effectively
/// making it suitable as input for the syntax parser (e.g.,
/// [ImmutableSyntaxTree::parse](crate::syntax::ImmutableSyntaxTree::parse)).
/// To provide the lookahead capabilities required by TokenCursor, TokenStream
/// maintains a temporary internal buffer. The size of the allocation depends on
/// the maximum lookahead length.
///
/// If the lookahead feature or the TokenCursor implementation is not required,
/// consider using [ChunkScanner] instead, a less expensive alternative to
/// the TokenStream.
///
/// Note that, since TokenStream is a temporary object, it does not semantically
/// represent compilation unit storage, and its [Id] is nil.
///
/// If you need long-term storage of the source code lexis with random-access
/// capabilities, consider using [TokenBuffer](crate::lexis::TokenBuffer)
/// instead.
///
/// The Debug implementation of TokenStream prints the remaining token chunks:
/// in non-alternate mode it displays up to a few chunks; in alternate mode it
/// displays all of them.
#[derive(Clone)]
pub struct TokenStream<'input, T: Token> {
    begin: ByteIndex,
    iter: ChunkScanner<'input, T>,
    buffer: VecDeque<Chunk<'input, T>>,
}

impl<'input, T: Token + Debug> Debug for TokenStream<'input, T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        debug_list(self.clone(), formatter)
    }
}

impl<'input, T: Token> Identifiable for TokenStream<'input, T> {
    #[inline(always)]
    fn id(&self) -> Id {
        Id::nil()
    }
}

impl<'input, T: Token> TokenCursor<'input> for TokenStream<'input, T> {
    type Token = T;

    #[inline(always)]
    fn advance(&mut self) -> bool {
        if let Some(chunk) = self.buffer.pop_front() {
            self.begin += chunk.string.len();
            return true;
        }

        if let Some(chunk) = self.iter.next() {
            self.begin += chunk.string.len();
            return true;
        };

        false
    }

    #[inline(always)]
    fn skip(&mut self, mut distance: TokenCount) {
        while distance > 0 {
            let Some(chunk) = self.buffer.pop_front() else {
                break;
            };

            self.begin += chunk.string.len();

            distance -= 1;
        }

        while distance > 0 {
            let Some(chunk) = self.iter.next() else {
                break;
            };

            self.begin += chunk.string.len();

            distance -= 1;
        }
    }

    #[inline(always)]
    fn token(&mut self, distance: TokenCount) -> Self::Token {
        let Some(chunk) = self.chunk(distance) else {
            return <Self::Token as Token>::eoi();
        };

        chunk.token
    }

    #[inline(always)]
    fn site(&mut self, distance: TokenCount) -> Option<Site> {
        Some(self.chunk(distance)?.site)
    }

    #[inline(always)]
    fn length(&mut self, distance: TokenCount) -> Option<Length> {
        Some(self.chunk(distance)?.length)
    }

    #[inline(always)]
    fn string(&mut self, distance: TokenCount) -> Option<&'input str> {
        Some(self.chunk(distance)?.string)
    }

    #[inline(always)]
    fn token_ref(&mut self, _distance: TokenCount) -> TokenRef {
        TokenRef::nil()
    }

    #[inline(always)]
    fn site_ref(&mut self, _distance: TokenCount) -> SiteRef {
        SiteRef::nil()
    }

    #[inline(always)]
    fn end_site_ref(&mut self) -> SiteRef {
        SiteRef::nil()
    }
}

impl<'input, T: Token> Iterator for TokenStream<'input, T> {
    type Item = Chunk<'input, T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = *self.chunk(0)?;

        let _ = self.advance();

        Some(chunk)
    }
}

impl<'input, T: Token> TokenStream<'input, T> {
    #[inline(always)]
    fn new(text: &'input str) -> Self {
        Self {
            begin: 0,
            iter: ChunkScanner::new(text),
            buffer: VecDeque::new(),
        }
    }

    /// Returns the remaining tail of the original input string.
    #[inline(always)]
    pub fn as_str(&self) -> &'input str {
        ld_assert!(
            self.begin <= self.iter.text.len(),
            "Malformed TokenStream byte index.",
        );

        unsafe { self.iter.text.get_unchecked(self.begin..) }
    }

    fn chunk(&mut self, distance: TokenCount) -> Option<&Chunk<'input, T>> {
        while distance >= self.buffer.len() {
            self.buffer.push_front(self.iter.next()?);
        }

        let Some(chunk) = self.buffer.get(distance) else {
            unsafe { ld_unreachable!("Malformed token stream buffer.") }
        };

        Some(chunk)
    }
}

/// A stateless lexical scanner over a string that yields [Chunks](Chunk).
///
/// The object is created by calling the [chunks](Scannable::chunks) function on
/// any input string (`String` or `&str`).
///
/// ChunkScanner implements the Iterator trait, scanning the input string
/// token by token until the end of the scanned input string, and returning
/// token metadata in the form of a [Chunk]. In contrast to
/// [TokenBuffer](crate::lexis::TokenBuffer), this implementation is stateless
/// and does not persist the lexical metadata of the underlying source code.
///
/// The Debug implementation for ChunkScanner prints the remaining token
/// chunks: in non-alternate mode it displays up to a few chunks; in alternate
/// mode it displays all of them.
#[derive(Clone)]
pub struct ChunkScanner<'input, T: Token> {
    text: &'input str,
    begin: Cursor<Site>,
    end: Cursor<Site>,
    current: Cursor<Site>,
    pending: Option<Chunk<'input, T>>,
}

impl<'input, T: Token + Debug> Debug for ChunkScanner<'input, T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        debug_list(self.clone(), formatter)
    }
}

unsafe impl<'input, T: Token> LexisSession for ChunkScanner<'input, T> {
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        self.current.advance(self.text)
    }

    #[inline(always)]
    unsafe fn consume(&mut self) {
        self.current.consume(self.text)
    }

    #[inline(always)]
    unsafe fn read(&mut self) -> char {
        self.current.read(self.text)
    }

    #[inline(always)]
    unsafe fn submit(&mut self) {
        #[cfg(debug_assertions)]
        if self.current.byte < self.text.len() {
            let byte = self.text.as_bytes()[self.current.byte];

            if byte & 0xC0 == 0x80 {
                system_panic!(
                    "Incorrect use of the LexisSession::submit function.\nA \
                    byte in front of the current cursor is UTF-8 continuation \
                    byte."
                );
            }
        }

        self.end = self.current;
    }
}

impl<'input, T: Token> Iterator for ChunkScanner<'input, T> {
    type Item = Chunk<'input, T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pending) = take(&mut self.pending) {
            return Some(pending);
        }

        if self.begin.byte == self.text.len() {
            return None;
        }

        let token = T::scan(self);

        if self.begin.byte != self.end.byte {
            let chunk = self.chunk(token, &self.begin, &self.end);

            self.begin = self.end;
            self.current = self.end;

            return Some(chunk);
        }

        let mismatch = self.begin;

        loop {
            if self.begin.advance(&self.text) == 0xFF {
                return Some(self.chunk(T::mismatch(), &mismatch, &self.begin));
            }

            self.begin.consume(&self.text);

            self.end = self.begin;
            self.current = self.begin;

            let token = T::scan(self);

            if self.begin.byte == self.end.byte {
                continue;
            }

            let result = self.chunk(T::mismatch(), &mismatch, &self.begin);
            let pending = self.chunk(token, &self.begin, &self.end);

            self.pending = Some(pending);

            self.begin = self.end;
            self.current = self.end;

            return Some(result);
        }
    }
}

impl<'input, T: Token> FusedIterator for ChunkScanner<'input, T> {}

impl<'input, T: Token> ChunkScanner<'input, T> {
    #[inline(always)]
    fn new(input: &'input str) -> Self {
        Self {
            text: input,
            begin: Cursor::default(),
            end: Cursor::default(),
            current: Cursor::default(),
            pending: None,
        }
    }

    /// Returns the remaining tail of the original input string.
    #[inline(always)]
    pub fn as_str(&self) -> &'input str {
        ld_assert!(
            self.begin.byte <= self.text.len(),
            "Malformed TokenStream byte index.",
        );

        unsafe { self.text.get_unchecked(self.begin.byte..) }
    }

    #[inline(always)]
    fn chunk(&self, token: T, from: &Cursor<Site>, to: &Cursor<Site>) -> Chunk<'input, T> {
        let length = to.site - from.site;

        ld_assert!(length > 0, "Empty length.");
        ld_assert!(from.byte < to.byte, "Invalid range.");
        ld_assert!(to.byte <= self.text.len(), "Invalid range.");

        let site = from.site;
        let string = unsafe { self.text.get_unchecked(from.byte..to.byte) };

        Chunk {
            token,
            site,
            length,
            string,
        }
    }
}

/// A stateless lexical scanner over a string that yields [Chunks](Chunk) and
/// their byte positions.
///
/// The object is created by calling
/// the [chunk_indices](Scannable::chunk_indices) function on any input string
/// (`String` or `&str`).
///
/// ChunkIndicesScanner implements the Iterator trait, scanning the input
/// string token by token until the end of the scanned input string, and
/// returning token metadata in the form of a `(ByteIndex, Chunk)`. In contrast
/// to [TokenBuffer](crate::lexis::TokenBuffer), this implementation is
/// stateless and does not persist the lexical metadata of the underlying
/// source code.
///
/// The Debug implementation for ChunkIndicesScanner prints the remaining token
/// chunks and their byte positions: in non-alternate mode it displays up to
/// a few chunks; in alternate mode it displays all of them.
#[derive(Clone)]
pub struct ChunkIndicesScanner<'input, T: Token> {
    text: &'input str,
    begin: Cursor<Site>,
    end: Cursor<Site>,
    current: Cursor<Site>,
    pending: Option<(ByteIndex, Chunk<'input, T>)>,
}

impl<'input, T: Token + Debug> Debug for ChunkIndicesScanner<'input, T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        debug_list(self.clone(), formatter)
    }
}

unsafe impl<'input, T: Token> LexisSession for ChunkIndicesScanner<'input, T> {
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        self.current.advance(self.text)
    }

    #[inline(always)]
    unsafe fn consume(&mut self) {
        self.current.consume(self.text)
    }

    #[inline(always)]
    unsafe fn read(&mut self) -> char {
        self.current.read(self.text)
    }

    #[inline(always)]
    unsafe fn submit(&mut self) {
        #[cfg(debug_assertions)]
        if self.current.byte < self.text.len() {
            let byte = self.text.as_bytes()[self.current.byte];

            if byte & 0xC0 == 0x80 {
                system_panic!(
                    "Incorrect use of the LexisSession::submit function.\nA \
                    byte in front of the current cursor is UTF-8 continuation \
                    byte."
                );
            }
        }

        self.end = self.current;
    }
}

impl<'input, T: Token> Iterator for ChunkIndicesScanner<'input, T> {
    type Item = (ByteIndex, Chunk<'input, T>);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pending) = take(&mut self.pending) {
            return Some(pending);
        }

        if self.begin.byte == self.text.len() {
            return None;
        }

        let token = T::scan(self);

        if self.begin.byte != self.end.byte {
            let chunk = self.chunk(token, &self.begin, &self.end);

            self.begin = self.end;
            self.current = self.end;

            return Some(chunk);
        }

        let mismatch = self.begin;

        loop {
            if self.begin.advance(&self.text) == 0xFF {
                return Some(self.chunk(T::mismatch(), &mismatch, &self.begin));
            }

            self.begin.consume(&self.text);

            self.end = self.begin;
            self.current = self.begin;

            let token = T::scan(self);

            if self.begin.byte == self.end.byte {
                continue;
            }

            let result = self.chunk(T::mismatch(), &mismatch, &self.begin);
            let pending = self.chunk(token, &self.begin, &self.end);

            self.pending = Some(pending);

            self.begin = self.end;
            self.current = self.end;

            return Some(result);
        }
    }
}

impl<'input, T: Token> FusedIterator for ChunkIndicesScanner<'input, T> {}

impl<'input, T: Token> ChunkIndicesScanner<'input, T> {
    #[inline(always)]
    fn new(input: &'input str) -> Self {
        Self {
            text: input,
            begin: Cursor::default(),
            end: Cursor::default(),
            current: Cursor::default(),
            pending: None,
        }
    }

    /// Returns the remaining tail of the original input string.
    #[inline(always)]
    pub fn as_str(&self) -> &'input str {
        ld_assert!(
            self.begin.byte <= self.text.len(),
            "Malformed TokenStream byte index.",
        );

        unsafe { self.text.get_unchecked(self.begin.byte..) }
    }

    /// Returns the byte position of the next token, or the length of
    /// the original input string if there are no more tokens.
    #[inline(always)]
    pub fn offset(&self) -> ByteIndex {
        match &self.pending {
            Some((byte, _)) => *byte,
            None => self.begin.byte,
        }
    }

    #[inline(always)]
    fn chunk(
        &self,
        token: T,
        from: &Cursor<Site>,
        to: &Cursor<Site>,
    ) -> (ByteIndex, Chunk<'input, T>) {
        let length = to.site - from.site;

        ld_assert!(length > 0, "Empty length.");
        ld_assert!(from.byte < to.byte, "Invalid range.");
        ld_assert!(to.byte <= self.text.len(), "Invalid range.");

        let byte = from.byte;
        let site = from.site;
        let string = unsafe { self.text.get_unchecked(from.byte..to.byte) };

        (
            byte,
            Chunk {
                token,
                site,
                length,
                string,
            },
        )
    }
}

/// A stateless lexical scanner over a string that yields tokens.
///
/// The object is created by calling the [tokens](Scannable::tokens) function on
/// any input string (`String` or `&str`).
///
/// TokenScanner implements the Iterator trait, scanning the input string
/// token by token until the end of the scanned input string, and returning
/// these tokens. In contrast to [TokenBuffer](crate::lexis::TokenBuffer), this
/// implementation is stateless and does not persist the lexical metadata of
/// the underlying source code.
///
/// The Debug implementation for TokenScanner prints the remaining tokens:
/// in non-alternate mode it displays up to a few tokens; in alternate mode
/// it displays all of them.
#[derive(Clone)]
pub struct TokenScanner<'input, T: Token> {
    text: &'input str,
    begin: Cursor<()>,
    end: Cursor<()>,
    current: Cursor<()>,
    pending: Option<T>,
}

impl<'input, T: Token + Debug> Debug for TokenScanner<'input, T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        debug_list(self.clone(), formatter)
    }
}

unsafe impl<'input, T: Token> LexisSession for TokenScanner<'input, T> {
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        self.current.advance(self.text)
    }

    #[inline(always)]
    unsafe fn consume(&mut self) {
        self.current.consume(self.text)
    }

    #[inline(always)]
    unsafe fn read(&mut self) -> char {
        self.current.read(self.text)
    }

    #[inline(always)]
    unsafe fn submit(&mut self) {
        #[cfg(debug_assertions)]
        if self.current.byte < self.text.len() {
            let byte = self.text.as_bytes()[self.current.byte];

            if byte & 0xC0 == 0x80 {
                system_panic!(
                    "Incorrect use of the LexisSession::submit function.\nA \
                    byte in front of the current cursor is UTF-8 continuation \
                    byte."
                );
            }
        }

        self.end = self.current;
    }
}

impl<'input, T: Token> Iterator for TokenScanner<'input, T> {
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pending) = take(&mut self.pending) {
            return Some(pending);
        }

        if self.begin.byte == self.text.len() {
            return None;
        }

        let token = T::scan(self);

        if self.begin.byte != self.end.byte {
            self.begin = self.end;
            self.current = self.end;

            return Some(token);
        }

        loop {
            if self.begin.advance(self.text) == 0xFF {
                return Some(T::mismatch());
            }

            self.begin.consume(self.text);

            self.end = self.begin;
            self.current = self.begin;

            let token = T::scan(self);

            if self.begin.byte == self.end.byte {
                continue;
            }

            self.pending = Some(token);

            self.begin = self.end;
            self.current = self.end;

            return Some(T::mismatch());
        }
    }
}

impl<'input, T: Token> FusedIterator for TokenScanner<'input, T> {}

impl<'input, T: Token> TokenScanner<'input, T> {
    #[inline(always)]
    fn new(input: &'input str) -> Self {
        Self {
            text: input,
            begin: Cursor::default(),
            end: Cursor::default(),
            current: Cursor::default(),
            pending: None,
        }
    }

    /// Returns the remaining tail of the original input string.
    #[inline(always)]
    pub fn as_str(&self) -> &'input str {
        ld_assert!(
            self.begin.byte <= self.text.len(),
            "Malformed TokenStream byte index.",
        );

        unsafe { self.text.get_unchecked(self.begin.byte..) }
    }
}

/// A stateless lexical scanner over a string that yields tokens and
/// their byte positions.
///
/// The object is created by calling
/// the [token_indices](Scannable::token_indices) function on any input
/// string (`String` or `&str`).
///
/// TokenIndicesScanner implements the Iterator trait, scanning the input string
/// token by token until the end of the scanned input string, and returning
/// these tokens in form of `(ByteIndex, Token)`. In contrast to
/// [TokenBuffer](crate::lexis::TokenBuffer), this implementation is stateless
/// and does not persist the lexical metadata of the underlying source code.
///
/// The Debug implementation for TokenScanner prints the remaining tokens and
/// their byte positions: in non-alternate mode it displays up to a few tokens;
/// in alternate mode it displays all of them.
#[derive(Clone)]
pub struct TokenIndicesScanner<'input, T: Token> {
    text: &'input str,
    begin: Cursor<()>,
    end: Cursor<()>,
    current: Cursor<()>,
    pending: Option<(ByteIndex, T)>,
}

impl<'input, T: Token + Debug> Debug for TokenIndicesScanner<'input, T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        debug_list(self.clone(), formatter)
    }
}

unsafe impl<'input, T: Token> LexisSession for TokenIndicesScanner<'input, T> {
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        self.current.advance(self.text)
    }

    #[inline(always)]
    unsafe fn consume(&mut self) {
        self.current.consume(self.text)
    }

    #[inline(always)]
    unsafe fn read(&mut self) -> char {
        self.current.read(self.text)
    }

    #[inline(always)]
    unsafe fn submit(&mut self) {
        #[cfg(debug_assertions)]
        if self.current.byte < self.text.len() {
            let byte = self.text.as_bytes()[self.current.byte];

            if byte & 0xC0 == 0x80 {
                system_panic!(
                    "Incorrect use of the LexisSession::submit function.\nA \
                    byte in front of the current cursor is UTF-8 continuation \
                    byte."
                );
            }
        }

        self.end = self.current;
    }
}

impl<'input, T: Token> Iterator for TokenIndicesScanner<'input, T> {
    type Item = (ByteIndex, T);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pending) = take(&mut self.pending) {
            return Some(pending);
        }

        if self.begin.byte == self.text.len() {
            return None;
        }

        let token = T::scan(self);

        if self.begin.byte != self.end.byte {
            let byte = self.begin.byte;

            self.begin = self.end;
            self.current = self.end;

            return Some((byte, token));
        }

        let mismatch = self.begin.byte;

        loop {
            if self.begin.advance(self.text) == 0xFF {
                return Some((mismatch, T::mismatch()));
            }

            self.begin.consume(self.text);

            self.end = self.begin;
            self.current = self.begin;

            let token = T::scan(self);

            if self.begin.byte == self.end.byte {
                continue;
            }

            self.pending = Some((self.begin.byte, token));

            self.begin = self.end;
            self.current = self.end;

            return Some((mismatch, T::mismatch()));
        }
    }
}

impl<'input, T: Token> FusedIterator for TokenIndicesScanner<'input, T> {}

impl<'input, T: Token> TokenIndicesScanner<'input, T> {
    #[inline(always)]
    fn new(input: &'input str) -> Self {
        Self {
            text: input,
            begin: Cursor::default(),
            end: Cursor::default(),
            current: Cursor::default(),
            pending: None,
        }
    }

    /// Returns the remaining tail of the original input string.
    #[inline(always)]
    pub fn as_str(&self) -> &'input str {
        ld_assert!(
            self.begin.byte <= self.text.len(),
            "Malformed TokenStream byte index.",
        );

        unsafe { self.text.get_unchecked(self.begin.byte..) }
    }

    /// Returns the byte position of the next token, or the length of
    /// the original input string if there are no more tokens.
    #[inline(always)]
    pub fn offset(&self) -> ByteIndex {
        match &self.pending {
            Some((byte, _)) => *byte,
            None => self.begin.byte,
        }
    }
}

/// An extension of a string that provides functions for creating stateless
/// lexical scanners.
///
/// In contrast to [TokenBuffer](crate::lexis::TokenBuffer), these scanners are
/// implemented as lazy iterators and do not persist the lexical metadata of the
/// underlying source code.
///
/// This trait is auto-implemented for any object that implements `AsRef<str>`.
pub trait Scannable {
    /// Creates an iterator that scans source code tokens one by one, yielding
    /// their [Chunks](Chunk).
    ///
    /// In contrast to the [chunks](Self::chunks) iterator, this iterator also
    /// implements [TokenCursor], and is therefore suitable as input for the
    /// syntax parser.
    ///
    /// The returned iterator's Debug implementation prints a list of the
    /// scanned token chunks.
    ///
    /// See [TokenStream] for details.
    fn stream<T: Token>(&self) -> TokenStream<T>;

    /// Creates an iterator that scans source code tokens one by one, yielding
    /// their [Chunks](Chunk).
    ///
    /// The returned iterator's Debug implementation prints a list of the
    /// scanned token chunks.
    ///
    /// See [ChunkScanner] for details.
    fn chunks<T: Token>(&self) -> ChunkScanner<T>;

    /// Creates an iterator that scans source code tokens one by one, yielding
    /// their [Chunks](Chunk) and [byte positions](ByteIndex).
    ///
    /// The returned iterator's Debug implementation prints a list of the
    /// scanned token chunks, and their byte positions.
    ///
    /// See [ChunkIndicesScanner] for details.
    fn chunk_indices<T: Token>(&self) -> ChunkIndicesScanner<T>;

    /// Creates an iterator that scans source code tokens one by one.
    ///
    /// The returned iterator's Debug implementation prints a list of the
    /// scanned tokens.
    ///
    /// See [TokenScanner] for details.
    fn tokens<T: Token>(&self) -> TokenScanner<T>;

    /// Creates an iterator that scans source code tokens one by one, and
    /// their [byte positions](ByteIndex).
    ///
    /// The returned iterator's Debug implementation prints a list of the
    /// scanned tokens, and their byte positions.
    ///
    /// See [TokenIndicesScanner] for details.
    fn token_indices<T: Token>(&self) -> TokenIndicesScanner<T>;
}

impl<S: AsRef<str>> Scannable for S {
    #[inline(always)]
    fn stream<T: Token>(&self) -> TokenStream<T> {
        TokenStream::new(self.as_ref())
    }

    #[inline(always)]
    fn chunks<T: Token>(&self) -> ChunkScanner<T> {
        ChunkScanner::new(self.as_ref())
    }

    #[inline(always)]
    fn chunk_indices<T: Token>(&self) -> ChunkIndicesScanner<T> {
        ChunkIndicesScanner::new(self.as_ref())
    }

    #[inline(always)]
    fn tokens<T: Token>(&self) -> TokenScanner<T> {
        TokenScanner::new(self.as_ref())
    }

    #[inline(always)]
    fn token_indices<T: Token>(&self) -> TokenIndicesScanner<T> {
        TokenIndicesScanner::new(self.as_ref())
    }
}

#[inline(always)]
fn debug_list<T: Debug>(
    iter: impl Iterator<Item = T>,
    formatter: &mut Formatter<'_>,
) -> std::fmt::Result {
    let alt = formatter.alternate();

    let mut debug_list = formatter.debug_list();

    for (index, item) in iter.enumerate() {
        if alt && index >= 20 {
            return debug_list.finish_non_exhaustive();
        }

        debug_list.entry(&item);
    }

    debug_list.finish()
}
