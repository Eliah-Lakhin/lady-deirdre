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
    arena::{Id, Identifiable, Ref, Sequence},
    lexis::{
        cursor::TokenBufferCursor,
        session::{Cursor, SequentialLexisSession},
        Chunk,
        ChunkRef,
        Length,
        Site,
        SourceCode,
        ToSpan,
        Token,
        TokenCount,
        CHUNK_SIZE,
    },
    report::debug_unreachable,
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
///     lexis::{TokenBuffer, SimpleToken, SourceCode, ChunkRef, Chunk},
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
///     .map(|chunk_ref: ChunkRef<SimpleToken>| chunk_ref.string)
///     .collect::<Vec<&str>>();
///
/// assert_eq!(token_strings, ["First", " ", "line", "\n", "Second", " ", "line", "\n"]);
///
/// // An API user can turn TokenBuffer into owned iterator of Chunks.
/// let chunks = token_buf.into_iter().collect::<Vec<Chunk<SimpleToken>>>();
///
/// assert_eq!(chunks[4].string.as_str(), "Second");
/// ```
pub struct TokenBuffer<T: Token> {
    id: Id,
    length: Length,
    pub(crate) tokens: Sequence<T>,
    pub(super) sites: Sequence<Site>,
    pub(crate) spans: Sequence<Length>,
    pub(crate) strings: Sequence<String>,
    pub(super) tail: String,
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
    fn contains_chunk(&self, chunk_ref: &Ref) -> bool {
        self.tokens.contains(chunk_ref)
    }

    #[inline(always)]
    fn get_token(&self, chunk_ref: &Ref) -> Option<Self::Token> {
        self.tokens.get(chunk_ref).copied()
    }

    #[inline(always)]
    fn get_site(&self, chunk_ref: &Ref) -> Option<Site> {
        self.sites.get(chunk_ref).copied()
    }

    #[inline(always)]
    fn get_string(&self, chunk_ref: &Ref) -> Option<&str> {
        self.strings.get(chunk_ref).map(|string| string.as_str())
    }

    #[inline(always)]
    fn get_length(&self, chunk_ref: &Ref) -> Option<Length> {
        self.spans.get(chunk_ref).copied()
    }

    #[inline(always)]
    fn cursor(&self, span: impl ToSpan) -> TokenBufferCursor<'_, Self::Token> {
        let span = match span.to_span(self) {
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
            strings: Default::default(),
            tail: String::new(),
        }
    }
}

impl<T: Token> IntoIterator for TokenBuffer<T> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = TokenBufferIntoIter<T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            site: 0,
            tokens: self.tokens.into_vec().into_iter(),
            spans: self.spans.into_vec().into_iter(),
            strings: self.strings.into_vec().into_iter(),
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
            strings: self.strings.inner().iter(),
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
            strings: Sequence::with_capacity(capacity),
            tail: String::new(),
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

        let site = match self.strings.pop() {
            None => 0,

            Some(last) => {
                self.tail = last;

                let _ = self.spans.pop();
                let _ = self.tokens.pop();

                match self.sites.pop() {
                    Some(site) => site,

                    // Safety: Underlying TokenBuffer collections represent
                    //         a sequence of Chunks.
                    None => unsafe { debug_unreachable!("TokenBuffer inconsistency.") },
                }
            }
        };

        self.tail.push_str(text);

        SequentialLexisSession::run(self, site);
    }

    /// Reserves capacity to store at least `additional` token chunks to be inserted on top of this
    /// buffer.
    #[inline(always)]
    pub fn reserve(&mut self, additional: TokenCount) {
        self.tokens.reserve(additional);
        self.sites.reserve(additional);
        self.spans.reserve(additional);
        self.strings.reserve(additional);
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
        let length = to.site - from.site;

        self.length += length;

        let string = unsafe { self.tail.get_unchecked(from.byte_index..to.byte_index) }.to_string();

        let _ = self.tokens.push(token);
        let _ = self.sites.push(from.site);
        let _ = self.spans.push(length);
        let _ = self.strings.push(string);
    }
}

pub struct TokenBufferIntoIter<T: Token> {
    site: Site,
    tokens: IntoIter<T>,
    spans: IntoIter<Length>,
    strings: IntoIter<String>,
}

impl<T: Token> Iterator for TokenBufferIntoIter<T> {
    type Item = Chunk<T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let token = self.tokens.next()?;
        let site = self.site;
        let length = self.spans.next()?;
        let string = self.strings.next()?;

        self.site += length;

        Some(Self::Item {
            token,
            site,
            length,
            string,
        })
    }
}

impl<T: Token> FusedIterator for TokenBufferIntoIter<T> {}

pub struct TokenBufferIter<'buffer, T: Token> {
    site: Site,
    tokens: Iter<'buffer, T>,
    spans: Iter<'buffer, Length>,
    strings: Iter<'buffer, String>,
}

impl<'sequence, T: Token> Iterator for TokenBufferIter<'sequence, T> {
    type Item = ChunkRef<'sequence, T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let token = *self.tokens.next()?;
        let site = self.site;
        let length = *self.spans.next()?;
        let string = self.strings.next()?;

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
