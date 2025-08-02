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
    borrow::Cow,
    fmt::{Debug, Display, Formatter},
    iter::{FusedIterator, Peekable, Take},
    slice::Iter,
    str::Chars,
};

use crate::{
    arena::{Entry, Id, Identifiable},
    format::SnippetFormatter,
    lexis::{
        cursor::TokenBufferCursor,
        session::{BufferLexisSession, Cursor},
        ByteIndex,
        Chunk,
        Length,
        LineIndex,
        Site,
        SourceCode,
        ToSpan,
        Token,
        TokenCount,
        CHUNK_SIZE,
    },
    report::{ld_assert, ld_unreachable},
};

/// A growable buffer of tokens.
///
/// This object provides canonical implementation of the [SourceCode] trait
/// specifically optimized for the one-time scanning of the source code text.
///
/// The TokenBuffer parses the lexical structure of the source code text and
/// provides a way to append text to the buffer, but it does not offer
/// incremental reparsing capabilities when the user edits random ranges
/// of the source code text.
///
/// If you need a compilation unit with the incremental reparsing capabilities
/// but without the syntax tree parser, consider using the mutable
/// [Document](crate::units::Document) or
/// [MutableUnit](crate::units::MutableUnit) specifying the type of the Node
/// to `VoidSyntax<T>` (see [VoidSyntax](crate::syntax::VoidSyntax)).
pub struct TokenBuffer<T: Token> {
    id: Id,
    pub(crate) tokens: Vec<T>,
    pub(super) sites: Vec<Site>,
    pub(crate) spans: Vec<Length>,
    pub(crate) indices: Vec<ByteIndex>,
    pub(crate) text: String,
    pub(crate) lines: LineIndex,
}

impl<T: Token> Debug for TokenBuffer<T> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter
            .debug_struct("TokenBuffer")
            .field("id", &self.id)
            .field("length", &self.length())
            .finish_non_exhaustive()
    }
}

impl<T: Token> Display for TokenBuffer<T> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter
            .snippet(self)
            .set_caption(format!("TokenBuffer({})", self.id))
            .finish()
    }
}

impl<T: Token, S: AsRef<str>> From<S> for TokenBuffer<T> {
    #[inline(always)]
    fn from(string: S) -> Self {
        let string = string.as_ref();

        let token_capacity = (string.len() / CHUNK_SIZE + 1).next_power_of_two();

        let mut buffer = TokenBuffer::with_capacity(token_capacity, string.len());

        buffer.append(string);

        buffer
    }
}

impl<T: Token> Drop for TokenBuffer<T> {
    fn drop(&mut self) {
        self.id.clear_name();
    }
}

impl<T: Token> Identifiable for TokenBuffer<T> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<T: Token> SourceCode for TokenBuffer<T> {
    type Token = T;

    type Cursor<'code> = TokenBufferCursor<'code, Self::Token>;

    type CharIterator<'code> = Take<Chars<'code>>;

    fn chars(&self, span: impl ToSpan) -> Self::CharIterator<'_> {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),
            Some(span) => span,
        };

        let span_length = span.end - span.start;

        if span_length == 0 {
            return "".chars().take(0);
        };

        let rest = match self.search(span.start) {
            Ok(byte_index) => {
                ld_assert!(byte_index < self.text.len(), "Byte index out of bounds.");

                unsafe { self.text.get_unchecked(byte_index..) }.chars()
            }

            Err((byte_index, mut remaining)) => {
                ld_assert!(byte_index < self.text.len(), "Byte index out of bounds.");

                let mut rest = unsafe { self.text.get_unchecked(byte_index..) }.chars();

                while remaining > 0 {
                    remaining -= 1;
                    let _ = rest.next();
                }

                rest
            }
        };

        rest.take(span_length)
    }

    fn substring(&self, span: impl ToSpan) -> Cow<str> {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),
            Some(span) => span,
        };

        let span_length = span.end - span.start;

        if span_length == 0 {
            return Cow::from("");
        };

        let start = match self.search(span.start) {
            Ok(byte_index) => byte_index,

            Err((byte_index, remaining)) => {
                ld_assert!(byte_index < self.text.len(), "Byte index out of bounds.");

                let rest = unsafe { self.text.get_unchecked(byte_index..) }.char_indices();

                let Some((offset, _)) = rest.take(remaining + 1).last() else {
                    unsafe { ld_unreachable!("Empty tail.") };
                };

                byte_index + offset
            }
        };

        let end = match self.search(span.end) {
            Ok(byte_index) => byte_index,

            Err((byte_index, remaining)) => {
                ld_assert!(byte_index < self.text.len(), "Byte index out of bounds.");

                let rest = unsafe { self.text.get_unchecked(byte_index..) }.char_indices();

                let Some((offset, _)) = rest.take(remaining + 1).last() else {
                    unsafe { ld_unreachable!("Empty tail.") };
                };

                byte_index + offset
            }
        };

        ld_assert!(start <= end, "Invalid byte bounds.");
        ld_assert!(end <= self.text.len(), "Invalid byte bounds.");

        unsafe { Cow::from(self.text.get_unchecked(start..end)) }
    }

    #[inline(always)]
    fn has_chunk(&self, entry: &Entry) -> bool {
        if entry.version > 0 {
            return false;
        }

        entry.index < self.tokens()
    }

    #[inline(always)]
    fn get_token(&self, entry: &Entry) -> Option<Self::Token> {
        if entry.version > 0 {
            return None;
        }

        self.tokens.get(entry.index).copied()
    }

    #[inline(always)]
    fn get_site(&self, entry: &Entry) -> Option<Site> {
        if entry.version > 0 {
            return None;
        }

        self.sites.get(entry.index).copied()
    }

    #[inline(always)]
    fn get_string(&self, entry: &Entry) -> Option<&str> {
        if entry.version > 0 {
            return None;
        }

        let start = *self.indices.get(entry.index)?;

        let end = self
            .indices
            .get(entry.index + 1)
            .copied()
            .unwrap_or(self.text.len());

        Some(unsafe { self.text.get_unchecked(start..end) })
    }

    #[inline(always)]
    fn get_length(&self, entry: &Entry) -> Option<Length> {
        if entry.version > 0 {
            return None;
        }

        self.spans.get(entry.index).copied()
    }

    #[inline(always)]
    fn cursor(&self, span: impl ToSpan) -> TokenBufferCursor<'_, Self::Token> {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),
            Some(span) => span,
        };

        Self::Cursor::new(self, span)
    }

    #[inline(always)]
    fn length(&self) -> Length {
        self.lines.code_length()
    }

    #[inline(always)]
    fn tokens(&self) -> TokenCount {
        self.tokens.len()
    }

    #[inline(always)]
    fn lines(&self) -> &LineIndex {
        &self.lines
    }
}

impl<T: Token> Default for TokenBuffer<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'buffer, T: Token> IntoIterator for &'buffer TokenBuffer<T> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = TokenBufferIter<'buffer, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            site: 0,
            tokens: self.tokens.iter(),
            spans: self.spans.iter(),
            indices: self.indices.iter().peekable(),
            text: self.text.as_str(),
        }
    }
}

impl<T: Token> TokenBuffer<T> {
    /// Creates a TokenBuffer by scanning the specified source code `text`.
    #[inline(always)]
    pub fn parse(text: impl AsRef<str>) -> Self {
        Self::from(text)
    }

    /// Creates an empty TokenBuffer.
    ///
    /// You can append and scan the source code text later using
    /// the [append](Self::append) function.
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            id: Id::new(),
            tokens: Vec::new(),
            sites: Vec::new(),
            spans: Vec::new(),
            indices: Vec::new(),
            text: String::new(),
            lines: LineIndex::new(),
        }
    }

    /// Creates an empty TokenBuffer but preallocates memory to store
    /// at least `tokens` number of the source code tokens, and the `text`
    /// number of bytes to store the source code text.
    #[inline(always)]
    pub fn with_capacity(tokens: TokenCount, text: usize) -> Self {
        Self {
            id: Id::new(),
            tokens: Vec::with_capacity(tokens),
            sites: Vec::with_capacity(tokens),
            spans: Vec::with_capacity(tokens),
            indices: Vec::with_capacity(tokens),
            text: String::with_capacity(text),
            lines: LineIndex::with_capacity(text),
        }
    }

    /// Appends the `text` to the end of the buffer by rescanning the tail of
    /// the former buffer (if non-empty) and the appendable text.
    pub fn append(&mut self, text: impl AsRef<str>) {
        let text = text.as_ref();

        if text.is_empty() {
            return;
        }

        let mut byte = self.text.len();
        let mut site = self.length();

        let mut lookback = 0;

        while lookback < T::LOOKBACK {
            let Some(span) = self.spans.pop() else {
                break;
            };

            lookback += span;

            if self.tokens.pop().is_none() {
                // Safety: Underlying TokenBuffer collections represent
                //         a sequence of Chunks.
                unsafe { ld_unreachable!("TokenBuffer inconsistency.") };
            }

            byte = match self.indices.pop() {
                Some(index) => index,
                // Safety: Underlying TokenBuffer collections represent
                //         a sequence of Chunks.
                None => unsafe { ld_unreachable!("TokenBuffer inconsistency.") },
            };

            site = match self.sites.pop() {
                Some(site) => site,
                // Safety: Underlying TokenBuffer collections represent
                //         a sequence of Chunks.
                None => unsafe { ld_unreachable!("TokenBuffer inconsistency.") },
            }
        }

        self.text.push_str(text);
        self.lines.append(text);

        BufferLexisSession::run(self, byte, site);
    }

    /// Reserves capacity to store at least `tokens` number of the source code
    /// tokens, and the `text` number of bytes to store the source code text.
    pub fn reserve(&mut self, tokens: TokenCount, text: usize) {
        self.tokens.reserve(tokens);
        self.sites.reserve(tokens);
        self.spans.reserve(tokens);
        self.indices.reserve(tokens);
        self.text.reserve(text);
        self.lines.reserve(text);
    }

    /// Shrinks the allocation capacity of the TokenBuffer as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.tokens.shrink_to_fit();
        self.sites.shrink_to_fit();
        self.spans.shrink_to_fit();
        self.indices.shrink_to_fit();
        self.text.shrink_to_fit();
        self.lines.shrink_to_fit();
    }

    /// Clears the TokenBuffer while preserving allocated memory.
    pub fn clear(&mut self) {
        self.tokens.clear();
        self.sites.clear();
        self.spans.clear();
        self.indices.clear();
        self.text.clear();
        self.lines.clear();
    }

    #[inline(always)]
    pub(crate) fn reset_id(&mut self) {
        self.id = Id::new();
    }

    #[inline(always)]
    pub(crate) fn update_line_index(&mut self) {
        self.lines.clear();
        self.lines.append(self.text.as_str());
    }

    #[inline]
    pub(super) fn push(&mut self, token: T, from: &Cursor<Site>, to: &Cursor<Site>) {
        let span = to.site - from.site;

        ld_assert!(span > 0, "Empty span.");

        let _ = self.tokens.push(token);
        let _ = self.sites.push(from.site);
        let _ = self.spans.push(span);
        let _ = self.indices.push(from.byte);
    }

    #[inline]
    fn search(&self, site: Site) -> Result<ByteIndex, (ByteIndex, Length)> {
        if site >= self.length() {
            return Ok(self.text.len());
        }

        match self.sites.binary_search(&site) {
            Ok(index) => {
                ld_assert!(index < self.indices.len(), "Index out of bounds.");

                let byte_index = unsafe { self.indices.get_unchecked(index) };

                Ok(*byte_index)
            }

            Err(mut index) => {
                ld_assert!(index > 0, "Index out of bounds.");

                index -= 1;

                ld_assert!(index < self.indices.len(), "Index out of bounds.");
                ld_assert!(index < self.sites.len(), "Index out of bounds.");

                let byte_index = unsafe { self.indices.get_unchecked(index) };
                let nearest_site = unsafe { self.sites.get_unchecked(index) };

                Err((*byte_index, site - *nearest_site))
            }
        }
    }
}

pub struct TokenBufferIter<'buffer, T: Token> {
    site: Site,
    tokens: Iter<'buffer, T>,
    spans: Iter<'buffer, Length>,
    indices: Peekable<Iter<'buffer, ByteIndex>>,
    text: &'buffer str,
}

impl<'buffer, T: Token> Iterator for TokenBufferIter<'buffer, T> {
    type Item = Chunk<'buffer, T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let token = *self.tokens.next()?;
        let site = self.site;
        let length = *self.spans.next()?;
        let start = *self.indices.next()?;

        let string = match self.indices.peek() {
            // Safety: TokenBuffer::indices are well-formed.
            None => unsafe { self.text.get_unchecked(start..) },
            // Safety: TokenBuffer::indices are well-formed.
            Some(end) => unsafe { self.text.get_unchecked(start..**end) },
        };

        self.site += length;

        Some(Self::Item {
            token,
            site,
            length,
            string,
        })
    }
}

impl<'buffer, T: Token> FusedIterator for TokenBufferIter<'buffer, T> {}
