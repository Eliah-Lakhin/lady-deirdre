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
    arena::{Entry, Id, Identifiable, Sequence},
    lexis::{
        cursor::TokenBufferCursor,
        session::{Cursor, SequentialLexisSession},
        ByteIndex,
        Chunk,
        Length,
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
/// Unit using [SyntaxBuffer](crate::syntax::SyntaxBuffer).
///
/// ```rust
/// use lady_deirdre::{
///     Document,
///     lexis::{TokenBuffer, SimpleToken, SourceCode, Chunk},
///     syntax::{SyntaxBuffer, SimpleNode, Node},
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
/// let _syntax_tree: SyntaxBuffer<SimpleNode> = SyntaxBuffer::parse(token_buf.cursor(..));
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
    length: Length,
    pub(crate) tokens: Sequence<T>,
    pub(super) sites: Sequence<Site>,
    pub(crate) spans: Sequence<Length>,
    pub(crate) indices: Sequence<ByteIndex>,
    pub(crate) text: String,
}

impl<T: Token> Debug for TokenBuffer<T> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("TokenBuffer")
            .field("id", &self.id)
            .field("length", &self.length)
            .finish_non_exhaustive()
    }
}

impl<T: Token, S: Borrow<str>> From<S> for TokenBuffer<T> {
    #[inline(always)]
    fn from(string: S) -> Self {
        let string = string.borrow();

        let mut buffer = TokenBuffer::with_capacity(string.len() / CHUNK_SIZE);

        buffer.append(string);

        buffer
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

    #[inline(always)]
    fn contains_chunk(&self, chunk_entry: &Entry) -> bool {
        self.tokens.contains(chunk_entry)
    }

    #[inline(always)]
    fn get_token(&self, chunk_entry: &Entry) -> Option<Self::Token> {
        self.tokens.get(chunk_entry).copied()
    }

    #[inline(always)]
    fn get_site(&self, chunk_entry: &Entry) -> Option<Site> {
        self.sites.get(chunk_entry).copied()
    }

    #[inline(always)]
    fn get_string(&self, chunk_entry: &Entry) -> Option<&str> {
        let index = match chunk_entry {
            Entry::Seq { index } => *index,
            _ => return None,
        };

        let inner = self.indices.inner();

        let start = *inner.get(index)?;
        let end = inner.get(index + 1).copied().unwrap_or(self.text.len());

        Some(unsafe { self.text.get_unchecked(start..end) })
    }

    #[inline(always)]
    fn get_length(&self, chunk_entry: &Entry) -> Option<Length> {
        self.spans.get(chunk_entry).copied()
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
        self.length
    }

    #[inline(always)]
    fn token_count(&self) -> TokenCount {
        let inner = unsafe { self.tokens.inner() };

        inner.len()
    }
}

impl<T: Token> Default for TokenBuffer<T> {
    #[inline]
    fn default() -> Self {
        Self {
            id: Id::new(),
            length: 0,
            tokens: Default::default(),
            sites: Default::default(),
            spans: Default::default(),
            indices: Default::default(),
            text: String::new(),
        }
    }
}

impl<'buffer, T: Token> IntoIterator for &'buffer TokenBuffer<T> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = TokenBufferIter<'buffer, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            site: 0,
            tokens: self.tokens.inner().iter(),
            spans: self.spans.inner().iter(),
            indices: self.indices.inner().iter().peekable(),
            text: self.text.as_str(),
        }
    }
}

impl<T: Token> TokenBuffer<T> {
    #[inline(always)]
    pub fn parse(string: impl Borrow<str>) -> Self {
        Self::from(string)
    }

    /// Creates a new TokenBuffer instance with pre-allocated memory for at least `capacity` token
    /// chunks to be stored in.
    #[inline(always)]
    pub fn with_capacity(capacity: TokenCount) -> Self {
        Self {
            id: Id::new(),
            length: 0,
            tokens: Sequence::with_capacity(capacity),
            sites: Sequence::with_capacity(capacity),
            spans: Sequence::with_capacity(capacity),
            indices: Sequence::with_capacity(capacity),
            text: String::with_capacity(CHUNK_SIZE * capacity),
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
                let _ = self.spans.pop();
                let _ = self.tokens.pop();

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

        SequentialLexisSession::run(self, byte, site);
    }

    /// Reserves capacity to store at least `additional` token chunks to be inserted on top of this
    /// buffer.
    #[inline(always)]
    pub fn reserve(&mut self, additional: TokenCount) {
        self.tokens.reserve(additional);
        self.sites.reserve(additional);
        self.spans.reserve(additional);
        self.indices.reserve(additional);
        self.text.reserve(additional * CHUNK_SIZE);
    }

    #[inline(always)]
    pub(crate) fn reset_id(&mut self) {
        self.id = Id::new();
    }

    #[inline(always)]
    pub(crate) fn set_length(&mut self, length: Length) {
        self.length = length;
    }

    #[inline]
    pub(super) fn push(&mut self, token: T, from: &Cursor, to: &Cursor) {
        let span = to.site - from.site;

        debug_assert!(span > 0, "Empty span.");

        self.length += span;

        let _ = self.tokens.push(token);
        let _ = self.sites.push(from.site);
        let _ = self.spans.push(span);
        let _ = self.indices.push(from.byte);
    }
}

pub struct TokenBufferIter<'buffer, T: Token> {
    site: Site,
    tokens: Iter<'buffer, T>,
    spans: Iter<'buffer, Length>,
    indices: Peekable<Iter<'buffer, ByteIndex>>,
    text: &'buffer str,
}

impl<'sequence, T: Token> Iterator for TokenBufferIter<'sequence, T> {
    type Item = Chunk<'sequence, T>;

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

impl<'sequence, T: Token> FusedIterator for TokenBufferIter<'sequence, T> {}
