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
    arena::{EntryIndex, Id, Identifiable, Sequence},
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
    std::*,
    syntax::PolyRef,
};

/// A lookahead iterator over the source code Tokens.
///
/// This is a low-level API that provides access features to the subset of the source code tokens
/// sequence. For a higher-level iteration access you can use a
/// [`CodeContent::chunks`](crate::lexis::CodeContent::chunks) function instead that returns a more
/// convenient [Iterator](::std::iter::iterator) over the [Chunk](crate::lexis::Chunk)
/// objects.
///
/// TokenCursor is the main access gate to the [SourceCode](crate::lexis::SourceCode) underlying
/// lexical data. An API user receives this object by calling a
/// [SourceCode::cursor](crate::lexis::SourceCode::cursor) function. Also, TokenCursor is a base
/// interface of the [SyntaxSession](crate::syntax::SyntaxSession) that iterates over a subset of
/// tokens in specified parsing context.
///
/// TokenCursor is an iterator-alike structure. This object provides functions to access
/// particular tokens and their metadata with lookahead capabilities from the current
/// inner [Site](crate::lexis::Site), and a function to [advance](TokenCursor::advance) the
/// inner Site.
///
/// Note that even though the lookahead operations do not advance the inner Cursor position, it's
/// SyntaxSession extension may track the lookahead distance to calculate the final syntax parsing
/// lookahead. This final value affects incremental re-parsing algorithm, and should be minimized
/// to gain the best performance results.
///
/// ```rust
/// use lady_deirdre::lexis::{TokenBuffer, SimpleToken, SourceCode, TokenCursor, ToSite};
///
/// let buf = TokenBuffer::<SimpleToken>::from("(foo bar)");
///
/// // A cursor over the "foo bar" substring tokens "touched" by the `4..5` span:
/// //   - Token "foo" is adjacent to the Site 4.
/// //   - Token " " is covered by 4..5 span.
/// //   - Token "bar" is adjacent to the Site 5.
/// //
/// // In the beginning the inner Cursor Site set to the beginning of the "foo" token(Site 1).
/// let mut cursor = buf.cursor(4..5);
///
/// // Looking ahead from the beginning.
/// assert_eq!(cursor.token(0), SimpleToken::Identifier);
/// assert_eq!(cursor.site(0).unwrap(), 1); // Token "foo" starts from Site 1.
/// assert_eq!(cursor.string(0).unwrap(), "foo");
/// assert_eq!(cursor.string(1).unwrap(), " ");
/// assert_eq!(cursor.string(2).unwrap(), "bar");
/// assert!(cursor.string(3).is_none());
///
/// // Advances cursor Site to the beginning of the next token " ".
/// assert!(cursor.advance());
///
/// assert_eq!(cursor.site(0).unwrap(), 4); // Token " " starts from Site 4.
/// assert_eq!(cursor.string(0).unwrap(), " ");
/// assert_eq!(cursor.string(1).unwrap(), "bar");
/// assert!(cursor.string(2).is_none());
///
/// // Advances cursor Site to the beginning of the last token "bar".
/// assert!(cursor.advance());
///
/// assert_eq!(cursor.site(0).unwrap(), 5); // Token "bar" starts from Site 5.
/// assert_eq!(cursor.string(0).unwrap(), "bar");
/// assert!(cursor.string(1).is_none());
///
/// // Advances cursor Site to the end of the last token "bar".
/// assert!(cursor.advance());
/// assert!(cursor.site(0).is_none()); // There are no more tokens in front of the Cursor.
/// assert!(cursor.string(0).is_none());
///
/// // Further advancement is not possible.
/// assert!(!cursor.advance());
///
/// // Since there are no more tokens in front of the Cursor Site the "site_ref" function returns
/// // a reference to the beginning of the next token "(" which is the Site of the end of
/// // the Cursor covered tokens.
/// let site_ref = cursor.site_ref(0);
///
/// assert_eq!(site_ref.to_site(&buf).unwrap(), 8);
/// ```
pub trait TokenCursor<'code>: Identifiable {
    /// A type of the [Token](crate::lexis::Token) of the [SourceCode](crate::lexis::SourceCode)
    /// instance this Cursor belongs to.
    type Token: Token;

    /// Advances TokenCursor inner [Site](crate::lexis::Site).
    ///
    /// If in front of the inner site there is a token covered by this TokenCursor, the inner site
    /// advances by the token's substring [Length](crate::lexis::Length), and the function
    /// returns `true`.
    ///
    /// Otherwise this function does nothing and returns `false`.
    fn advance(&mut self) -> bool;

    fn skip(&mut self, distance: TokenCount);

    /// Looks ahead of the [Token](crate::lexis::Token) in front of the TokenCursor inner
    /// [Site](crate::lexis::Site).
    ///
    /// If there are `distance` number of tokens covered by the TokenCursor in front of the
    /// TokenCursor inner site, this function returns [Some] reference to this Token,
    /// otherwise returns [None].
    ///
    /// `distance` is zero-based argument. Number `0` refers to the first token in front of the
    /// current inner site. `1` refers to the second token, and so on.
    ///
    /// This function does not advance TokenCursor inner site, but it could track an overall
    /// lookahead distance of the [parsing session](crate::syntax::SyntaxSession) that affects
    /// incremental re-parsing capabilities. An API user should prefer to minimize the lookahead
    /// distance to gain the best performance.
    fn token(&mut self, distance: TokenCount) -> Self::Token;

    /// Looks ahead of the [Token](crate::lexis::Token)'s start [Site](crate::lexis::Site) in front
    /// of the TokenCursor inner Site.
    ///
    /// If there are `distance` number of tokens covered by the TokenCursor in front of the
    /// TokenCursor inner site, this function returns Token's [Some] Site, otherwise returns [None].
    ///
    /// `distance` is zero-based argument. Number `0` refers to the first token in front of the
    /// current inner site. `1` refers to the second token, and so on.
    ///
    /// In particular, `site(0)` would return the current TokenCursor inner site if there are
    /// covered tokens left in front of the site.
    ///
    /// This function does not advance TokenCursor inner site, but it could track an overall
    /// lookahead distance of the [parsing session](crate::syntax::SyntaxSession) that affects
    /// incremental re-parsing capabilities. An API user should prefer to minimize the lookahead
    /// distance to gain the best performance.
    fn site(&mut self, distance: TokenCount) -> Option<Site>;

    /// Looks ahead of the [Token](crate::lexis::Token)'s string [Length](crate::lexis::Length)
    /// in front of the TokenCursor inner [Site](crate::lexis::Site).
    ///
    /// If there are `distance` number of tokens covered by the TokenCursor in front of the
    /// TokenCursor inner site, this function returns Token's string [Some] Length,
    /// otherwise returns [None].
    ///
    /// `distance` is zero-based argument. Number `0` refers to the first token in front of the
    /// current inner site. `1` refers to the second token, and so on.
    ///
    /// This function does not advance TokenCursor inner site, but it could track an overall
    /// lookahead distance of the [parsing session](crate::syntax::SyntaxSession) that affects
    /// incremental re-parsing capabilities. An API user should prefer to minimize the lookahead
    /// distance to gain the best performance.
    fn length(&mut self, distance: TokenCount) -> Option<Length>;

    /// Looks ahead of the [Token](crate::lexis::Token)'s string in front of the TokenCursor inner
    /// [Site](crate::lexis::Site).
    ///
    /// If there are `distance` number of tokens covered by the TokenCursor in front of the
    /// TokenCursor inner site, this function returns Token's [Some] string slice,
    /// otherwise returns [None].
    ///
    /// `distance` is zero-based argument. Number `0` refers to the first token in front of the
    /// current inner site. `1` refers to the second token, and so on.
    ///
    /// This function does not advance TokenCursor inner site, but it could track an overall
    /// lookahead distance of the [parsing session](crate::syntax::SyntaxSession) that affects
    /// incremental re-parsing capabilities. An API user should prefer to minimize the lookahead
    /// distance to gain the best performance.
    fn string(&mut self, distance: TokenCount) -> Option<&'code str>;

    /// Looks ahead of the [Token](crate::lexis::Token) in front of the TokenCursor inner
    /// [Site](crate::lexis::Site), and returns a [weak reference](crate::lexis::TokenRef) to this
    /// token.
    ///
    /// If there are `distance` number of tokens covered by the TokenCursor in front of the
    /// TokenCursor inner site, this function returns [Some] weak reference to this Token,
    /// otherwise returns [None].
    ///
    /// `distance` is zero-based argument. Number `0` refers to the first token in front of the
    /// current inner site. `1` refers to the second token, and so on.
    ///
    /// This function does not advance TokenCursor inner site, but it could track an overall
    /// lookahead distance of the [parsing session](crate::syntax::SyntaxSession) that affects
    /// incremental re-parsing capabilities. An API user should prefer to minimize the lookahead
    /// distance to gain the best performance.
    fn token_ref(&mut self, distance: TokenCount) -> TokenRef;

    /// Returns [weak reference](crate::lexis::SiteRef) to the source code
    /// [Site](crate::lexis::Site) in front of the TokenCursor inner site.
    ///
    /// If there are `distance` number of tokens covered by the TokenCursor in front of the
    /// TokenCursor inner site, this function returns a weak reference to the start site of that
    /// token, otherwise this function returns a SiteRef pointing to the end Site of the covered
    /// token sequence.
    ///
    /// `distance` is zero-based argument. Number `0` refers to the first token in front of the
    /// current inner site. `1` refers to the second token, and so on. In particular, `site_ref(0)`
    /// returns a weak reference to the current TokenCursor inner site.
    ///
    /// Note, that in contrast to [TokenCursor::site](crate::lexis::TokenCursor::site) function this
    /// function always returns meaningful valid SiteRef even if there are no tokens in front of the
    /// inner site, or if there are no tokens covered by this TokenCursor.
    ///
    /// This function does not advance TokenCursor inner site, but it could track an overall
    /// lookahead distance of the [parsing session](crate::syntax::SyntaxSession) that affects
    /// incremental re-parsing capabilities. An API user should prefer to minimize the lookahead
    /// distance to gain the best performance.
    fn site_ref(&mut self, distance: TokenCount) -> SiteRef;

    /// Returns a [weak reference](crate::lexis::SiteRef) pointing to the source code
    /// [Site](crate::lexis::Site) in the end of the TokenCursor covered token sequence.
    ///
    /// Note that this function always returns meaningful valid SiteRef regardless to the
    /// TokenCursor inner site, and even if there are no tokens covered by this TokenCursor
    /// instance.
    ///
    /// This function does not advance TokenCursor inner site, but it could track an overall
    /// lookahead distance of the [parsing session](crate::syntax::SyntaxSession) that affects
    /// incremental re-parsing capabilities. In particular, this function sets max lookahead
    /// to the end of the TokenCursor covered tokens. An API user should prefer to minimize the
    /// lookahead distance to gain the best performance.
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
        if self.next >= self.buffer.token_count() {
            return false;
        }

        let next_site = unsafe { *self.buffer.sites.inner().get_unchecked(self.next) };

        if next_site > self.end_site {
            return false;
        }

        self.next += 1;

        true
    }

    #[inline(always)]
    fn skip(&mut self, distance: TokenCount) {
        self.next = (self.next + distance).min(self.buffer.token_count());
    }

    #[inline]
    fn token(&mut self, mut distance: TokenCount) -> Self::Token {
        distance += self.next;

        if distance >= self.buffer.token_count() {
            return <Self::Token as Token>::eoi();
        }

        let peek_site = unsafe { *self.buffer.sites.inner().get_unchecked(distance) };

        if peek_site > self.end_site {
            return <Self::Token as Token>::eoi();
        }

        *unsafe { self.buffer.tokens.inner().get_unchecked(distance) }
    }

    #[inline]
    fn site(&mut self, mut distance: TokenCount) -> Option<Site> {
        distance += self.next;

        if distance >= self.buffer.token_count() {
            return None;
        }

        let peek_site = unsafe { *self.buffer.sites.inner().get_unchecked(distance) };

        if peek_site > self.end_site {
            return None;
        }

        Some(peek_site)
    }

    #[inline]
    fn length(&mut self, mut distance: TokenCount) -> Option<Length> {
        distance += self.next;

        if distance >= self.buffer.token_count() {
            return None;
        }

        let peek_site = unsafe { *self.buffer.sites.inner().get_unchecked(distance) };

        if peek_site > self.end_site {
            return None;
        }

        Some(*unsafe { self.buffer.spans.inner().get_unchecked(distance) })
    }

    #[inline]
    fn string(&mut self, mut distance: TokenCount) -> Option<&'code str> {
        distance += self.next;

        if distance >= self.buffer.token_count() {
            return None;
        }

        let peek_site = unsafe { *self.buffer.sites.inner().get_unchecked(distance) };

        if peek_site > self.end_site {
            return None;
        }

        let inner = self.buffer.indices.inner();
        let text = self.buffer.text.as_str();

        let start = *unsafe { inner.get_unchecked(distance) };
        let end = inner.get(distance + 1).copied().unwrap_or(text.len());

        Some(unsafe { text.get_unchecked(start..end) })
    }

    #[inline]
    fn token_ref(&mut self, mut distance: TokenCount) -> TokenRef {
        distance += self.next;

        if distance >= self.buffer.token_count() {
            return TokenRef::nil();
        }

        let peek_site = unsafe { *self.buffer.sites.inner().get_unchecked(distance) };

        if peek_site > self.end_site {
            return TokenRef::nil();
        }

        TokenRef {
            id: self.buffer.id(),
            chunk_entry: Sequence::<Self::Token>::entry_of(distance),
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
                if index >= self.buffer.token_count() {
                    self.end_site_ref = SiteRef::new_code_end(self.buffer.id());
                    break;
                }

                let peek_site = unsafe { *self.buffer.sites.inner().get_unchecked(index) };

                if peek_site > self.end_site {
                    self.end_site_ref = TokenRef {
                        id: self.buffer.id(),
                        chunk_entry: Sequence::<Self::Token>::entry_of(index),
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

        while next < buffer.token_count() {
            let site = unsafe { *buffer.sites.inner().get_unchecked(next) };
            let length = unsafe { *buffer.spans.inner().get_unchecked(next) };

            if site + length < span.start {
                next += 1;
                continue;
            }

            break;
        }

        let end_site_ref = match span.end >= buffer.length() {
            true => SiteRef::new_code_end(buffer.id()),
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
