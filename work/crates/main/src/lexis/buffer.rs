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
    format::{PrintString, SnippetFormatter},
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
    report::{debug_assert, debug_unreachable},
    std::*,
};

/// A growable buffer of the source code lexical data.
///
/// This buffer is a default implementation of the [SourceCode](crate::lexis::SourceCode) trait that
/// holds the source code of a compilation unit, and the lexical structure of the code(tokens and
/// token metadata).
///
/// In contrast to [Document](crate::Document), TokenBuffer provides an
/// [append](TokenBuffer::append) function to write strings to the end of underlying source code
/// only, but this operation works faster than the random-access
/// [`Document::write`](crate::Document::write) function.
///
/// An API user is encouraged to use TokenBuffer together with Rust's [BufRead](::std::io::BufRead)
/// and similar objects to preload source code files by sequentially reading strings from the source
/// and feeding them to the TokenBuffer using Append function.
///
/// Later on a TokenBuffer can either be turned into a Document instance using
/// [into_document](TokenBuffer::into_document) function, or to be used by itself as
/// a non-incremental storage of the lexical data of compilation unit.
///
/// For non-incremental usage an API user can also obtain a non-incremental syntax structure of the
/// Unit using [SyntaxBuffer](crate::syntax::ImmutableSyntaxTree).
///
/// ```rust
/// use lady_deirdre::{
///     units::Document,
///     lexis::{TokenBuffer, SimpleToken, SourceCode, Chunk},
///     syntax::{ImmutableSyntaxTree, SimpleNode, Node},
/// };
///
/// // Alternatively, you can use
/// //  - `TokenBuffer::from("head string")` providing an initial String to parse;
/// //  - or a shortcut function `SimpleToken::parse("head string")`;
/// //  - or a `TokenBuffer::with_capacity(10)` function to specify buffer's token capacity.
/// let mut token_buf = TokenBuffer::<SimpleToken>::default();
///
/// token_buf.append("First line\n");
/// token_buf.append("Second line\n");
///
/// // Turning the TokenBuffer to incremental Document.
/// let _doc = token_buf.into_document::<SimpleNode>();
///
/// let mut token_buf = TokenBuffer::<SimpleToken>::default();
///
/// token_buf.append("First line\n");
/// token_buf.append("Second line\n");
///
/// // Obtaining a non-incremental syntax structure of the entire compilation unit.
/// let _syntax_tree: ImmutableSyntaxTree<SimpleNode> = ImmutableSyntaxTree::parse(token_buf.cursor(..));
///
/// // TokenBuffer is traversable structure of Chunk references.
/// let token_strings = (&token_buf)
///     .into_iter()
///     .map(|chunk: Chunk<SimpleToken>| chunk.string)
///     .collect::<Vec<&str>>();
///
/// assert_eq!(token_strings, ["First", " ", "line", "\n", "Second", " ", "line", "\n"]);
///
/// // An API user can iterate through the TokenBuffer Chunks.
/// let chunks = (&token_buf).into_iter().collect::<Vec<Chunk<SimpleToken>>>();
///
/// assert_eq!(chunks[4].string, "Second");
/// ```
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
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter
            .debug_struct("TokenBuffer")
            .field("id", &self.id)
            .field("length", &self.length())
            .finish_non_exhaustive()
    }
}

impl<T: Token> Display for TokenBuffer<T> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
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
                debug_assert!(byte_index < self.text.len(), "Byte index out of bounds.");

                unsafe { self.text.get_unchecked(byte_index..) }.chars()
            }

            Err((byte_index, mut remaining)) => {
                debug_assert!(byte_index < self.text.len(), "Byte index out of bounds.");

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

    fn substring(&self, span: impl ToSpan) -> PrintString<'_> {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),
            Some(span) => span,
        };

        let span_length = span.end - span.start;

        if span_length == 0 {
            return PrintString::empty();
        };

        let start = match self.search(span.start) {
            Ok(byte_index) => byte_index,

            Err((byte_index, remaining)) => {
                debug_assert!(byte_index < self.text.len(), "Byte index out of bounds.");

                let rest = unsafe { self.text.get_unchecked(byte_index..) }.char_indices();

                let Some((offset, _)) = rest.take(remaining + 1).last() else {
                    unsafe { debug_unreachable!("Empty tail.") };
                };

                byte_index + offset
            }
        };

        let end = match self.search(span.end) {
            Ok(byte_index) => byte_index,

            Err((byte_index, remaining)) => {
                debug_assert!(byte_index < self.text.len(), "Byte index out of bounds.");

                let rest = unsafe { self.text.get_unchecked(byte_index..) }.char_indices();

                let Some((offset, _)) = rest.take(remaining + 1).last() else {
                    unsafe { debug_unreachable!("Empty tail.") };
                };

                byte_index + offset
            }
        };

        debug_assert!(start <= end, "Invalid byte bounds.");
        debug_assert!(end <= self.text.len(), "Invalid byte bounds.");

        unsafe {
            PrintString::new_unchecked(Cow::from(self.text.get_unchecked(start..end)), span_length)
        }
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
    #[inline(always)]
    pub fn parse(string: impl AsRef<str>) -> Self {
        Self::from(string)
    }

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

    /// Creates a new TokenBuffer instance with pre-allocated memory for at least `capacity` token
    /// chunks to be stored in.
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

    /// Writes `text` to the end of the buffer's source code, lexically parses source code tail
    /// in accordance to these changes.
    ///
    /// Performance of this operation is relative to the `text` size.
    ///
    /// An intended use of this function is to feed strings(e.g. lines) of the source code file
    /// from the Rust's  [BufRead](::std::io::BufRead).
    pub fn append(&mut self, text: impl AsRef<str>) {
        let text = text.as_ref();

        if text.is_empty() {
            return;
        }

        let byte;
        let site;

        match self.text.is_empty() {
            true => {
                byte = 0;
                site = 0;
            }

            false => {
                if self.spans.pop().is_none() {
                    // Safety: Underlying TokenBuffer collections represent
                    //         a sequence of Chunks.
                    unsafe { debug_unreachable!("TokenBuffer inconsistency.") };
                }

                if self.tokens.pop().is_none() {
                    // Safety: Underlying TokenBuffer collections represent
                    //         a sequence of Chunks.
                    unsafe { debug_unreachable!("TokenBuffer inconsistency.") };
                }

                byte = match self.indices.pop() {
                    Some(index) => index,
                    // Safety: Underlying TokenBuffer collections represent
                    //         a sequence of Chunks.
                    None => unsafe { debug_unreachable!("TokenBuffer inconsistency.") },
                };

                site = match self.sites.pop() {
                    Some(site) => site,

                    // Safety: Underlying TokenBuffer collections represent
                    //         a sequence of Chunks.
                    None => unsafe { debug_unreachable!("TokenBuffer inconsistency.") },
                }
            }
        };

        self.text.push_str(text);
        self.lines.append(text);

        BufferLexisSession::run(self, byte, site);
    }

    /// Reserves capacity to store at least `additional` token chunks to be inserted on top of this
    /// buffer.
    pub fn reserve(&mut self, tokens: TokenCount, text: usize) {
        self.tokens.reserve(tokens);
        self.sites.reserve(tokens);
        self.spans.reserve(tokens);
        self.indices.reserve(tokens);
        self.text.reserve(text);
        self.lines.reserve(text);
    }

    pub fn shrink_to_fit(&mut self) {
        self.tokens.shrink_to_fit();
        self.sites.shrink_to_fit();
        self.spans.shrink_to_fit();
        self.indices.shrink_to_fit();
        self.text.shrink_to_fit();
        self.lines.shrink_to_fit();
    }

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
    pub(super) fn push(&mut self, token: T, from: &Cursor, to: &Cursor) {
        let span = to.site - from.site;

        debug_assert!(span > 0, "Empty span.");

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
                debug_assert!(index < self.indices.len(), "Index out of bounds.");

                let byte_index = unsafe { self.indices.get_unchecked(index) };

                Ok(*byte_index)
            }

            Err(mut index) => {
                debug_assert!(index > 0, "Index out of bounds.");

                index -= 1;

                debug_assert!(index < self.indices.len(), "Index out of bounds.");
                debug_assert!(index < self.sites.len(), "Index out of bounds.");

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
            // Safety: TokenBuffer::indices are well formed.
            None => unsafe { self.text.get_unchecked(start..) },
            // Safety: TokenBuffer::indices are well formed.
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
