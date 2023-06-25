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

extern crate lady_deirdre_derive;

pub use lady_deirdre_derive::Token;

use crate::{
    arena::{Id, Identifiable, Ref},
    lexis::{ChunkRef, Length, LexisSession, Site, SiteRef, SourceCode, TokenIndex},
    std::*,
};

/// A number of Tokens.
pub type TokenCount = usize;

/// A trait that specifies Token's kind and provides a lexical grammar parser.
///
/// An API user implements this trait to specify Programming Language lexical grammar and the
/// lexis unit type(a "Token").
///
/// This trait is supposed to be implemented on the Rust enum type with variants representing
/// token kinds, but this is not a strict requirement. From the functional sense the main purpose
/// of the Token implementation is to provide a lexical parser that will re-parse sequences of
/// Unicode character by interacting with arbitrary [LexisSession](crate::lexis::LexisSession)
/// interface that, in turn, manages parsing process.
///
/// An API user is encouraged to implement this trait using helper
/// [Token](::lady_deirdre_derive::Token) macro-derive on enum types by specifying lexical
/// grammar directly on enum variants through the macros attributes.
///
/// ```rust
/// use lady_deirdre::lexis::{Token, TokenBuffer, CodeContent, ChunkRef};
///
/// #[derive(Token, Clone, Copy, PartialEq, Eq, Debug)]
/// #[repr(u8)]
/// enum MyToken {
///     EOI = 0,
///
///     // Character sequences that don't fit this grammar.
///     Mismatch = 1,
///
///     // Exact string "FOO".
///     #[rule("FOO")]
///     Foo,
///
///     // Exact string "bar".
///     #[rule("bar")]
///     Bar,
///
///     // An unlimited non empty sequence of '_' characters.
///     #[rule('_'+)]
///     LowDashes,
/// }
///
/// let mut buf = TokenBuffer::<MyToken>::from("FOO___bar_mismatch__FOO");
///
/// assert_eq!(
///     buf.chunks(..).map(|chunk: ChunkRef<'_, MyToken>| chunk.token).collect::<Vec<_>>(),
///     vec![
///         MyToken::Foo,
///         MyToken::LowDashes,
///         MyToken::Bar,
///         MyToken::LowDashes,
///         MyToken::Mismatch,
///         MyToken::LowDashes,
///         MyToken::Foo,
///     ],
/// );
/// ```
///
/// The Token enum object may keep additional semantic metadata inside variants' fields, but
/// optimization-wise you will gain the best performance if the Token would require as little
/// allocated memory as possible(ideally one byte).
///
/// An API user can implement the Token trait manually too. For example, using 3rd party lexical
/// scanner libraries. See [`Token::new`](crate::lexis::Token::parse) function specification for
/// details.
pub trait Token: Copy + Eq + Sized + 'static {
    /// Parses a single token from the source code text, and returns a Token instance that
    /// represents this token kind.
    ///
    /// This is a low-level API function.
    ///
    /// An API user encouraged to use [Token](::lady_deirdre_derive::Token) macro-derive to
    /// implement this trait automatically based on a set of Regular Expressions,
    /// but you can implement it manually too.
    ///
    /// You need to call this function manually only if you want to implement an extension API to
    /// this crate. In this case you should also prepare a custom implementation of the LexisSession
    /// trait. See [LexisSession](crate::lexis::LexisSession) specification for details.
    ///
    /// The function implements a
    /// [Finite-State Machine](https://en.wikipedia.org/wiki/Finite-state_machine) that reads
    /// as many [characters](char) from input sequence of [String](::std::ops::String) as needed to
    /// decide about the read substring token kind. Each time the function reads the next character,
    /// it advances `session` internal cursor.
    ///
    /// As the function implements a FSM it should not look of more than a single character ahead
    /// to make a decision on each Algorithm step. Failure to do so could lead to logical
    /// errors during incremental re-parsing.
    ///
    /// **Algorithm Specification:**
    ///   - The Algorithm invokes [`session.character()`](crate::lexis::LexisSession::character)
    ///     function to fetch the character that the Session's internal cursor currently looking at.
    ///     This function does not advance internal cursor. In the beginning the cursor points to
    ///     the first character of input String.
    ///
    ///     If this function returns a Null-character(`'\0'`) that means that the Session cursor has
    ///     reached the end of input. This character is not a part of the input sequence, an
    ///     Algorithm should ignore this character, but it should make a final decision and return
    ///     a Token instance. Note that if the original input sequence(a source code text) contains
    ///     Null-character, the `session.character()` yields a
    ///     [replacement character](::std::char::REPLACEMENT_CHARACTER) instead.
    ///   - The Algorithm invokes [`session.advance()`](crate::lexis::LexisSession::advance)
    ///     function to advance the Session's internal cursor to the next character in the input
    ///     character sequence.
    ///   - If the Algorithm decides that the input substring prior to the current character
    ///     contains complete token, it should invoke a
    ///     [`session.submit()`](crate::lexis::LexisSession::submit) function. In this case the
    ///     Algorithm could either return a Token instance, or to continue scanning process. The
    ///     LexisSession ignores all calls of Submit function happening before the last one.
    ///
    ///     Note that "submitted" character is not going to be a part of the parsed token substring.
    ///     By calling a Submit function the Algorithm submits the character sequence prior
    ///     to the current character.
    ///   - It is assumed that the Token type defines special kind of Token that would specify
    ///     a lexically-unrecognizable input sequences. If the Algorithm cannot recognize a
    ///     lexically valid token it should never call a `session.submit()` function, and in the end
    ///     it should return an instance of such "mismatched" Token.
    ///   - The Algorithm can optionally obtain a slice of the input string from the beginning of
    ///     the scanning session till submitted character exclusively by calling a
    ///     [`session.substring()`](crate::lexis::LexisSession::substring) function.The Algorithm
    ///     can use this substring to analise and store additional metadata inside the returning
    ///     Token instance.
    ///
    /// ```rust
    /// use lady_deirdre::lexis::{
    ///     Token,
    ///     TokenBuffer,
    ///     TokenIndex,
    ///     CodeContent,
    ///     LexisSession,
    ///     ChunkRef,
    /// };
    ///
    /// // Represents integer numbers or lower case alphabetic words.
    /// #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    /// #[repr(u8)]
    /// enum NumOrWord {
    ///     EOI = 0,
    ///     Mismatch = 1,
    ///     Num = 2,
    ///     Word = 3,
    /// }
    ///
    /// impl Token for NumOrWord {
    ///     fn parse(session: &mut impl LexisSession) -> Self {
    ///         if session.advance() == 0xFF {
    ///             return Self::Mismatch;
    ///         }
    ///
    ///         // Safety: Because the LexisSession guarantees to provide valid
    ///         //         UTF-8 sequence of bytes, it is OK to decode each
    ///         //         incoming code point until the 0xFF byte reached.
    ///         let ch = unsafe { session.read() };
    ///
    ///         if ch >= 'a' && ch <= 'z' {
    ///             loop {
    ///                 if session.advance() == 0xFF { break; }
    ///                 let ch = unsafe { session.read() };
    ///
    ///                 if ch < 'a' || ch > 'z' { break; }
    ///
    ///                 // Safety: The scanner walks through the decoded code
    ///                 //         points only, so each next byte is the beginning
    ///                 //         of the next code point, or the end of input.
    ///                 unsafe { session.submit() };
    ///             }
    ///
    ///             return Self::Word;
    ///         }
    ///
    ///         if ch == '0' {
    ///             unsafe { session.submit() };
    ///             return Self::Num;
    ///         }
    ///
    ///         if ch >= '1' && ch <= '9' {
    ///             loop {
    ///                 if session.advance() == 0xFF { break; }
    ///                 let ch = unsafe { session.read() };
    ///
    ///                 if ch < '0' || ch > '9' { break; }
    ///
    ///                 unsafe { session.submit() };
    ///             }
    ///
    ///             return Self::Num;
    ///         }
    ///
    ///         Self::Mismatch
    ///     }
    ///
    ///     fn eoi() -> Self {
    ///         Self::EOI
    ///     }
    ///
    ///     fn mismatch() -> Self {
    ///         Self::Mismatch
    ///     }
    ///
    ///     fn index(self) -> TokenIndex {
    ///         self as u8
    ///     }
    ///
    ///     fn describe(index: TokenIndex) -> Option<&'static str> {
    ///         match index {
    ///             0 => Some("<mismatch>"),
    ///             1 => Some("<num>"),
    ///             2 => Some("<word>"),
    ///             _ => None,
    ///         }
    ///     }
    /// }
    ///
    /// let buf = TokenBuffer::<NumOrWord>::from("foo123_bar");
    ///
    /// assert_eq!(
    ///     buf
    ///         .chunks(..)
    ///         .map(|chunk_ref: ChunkRef<NumOrWord>| chunk_ref.token)
    ///         .collect::<Vec<_>>(),
    ///     vec![
    ///         NumOrWord::Word,
    ///         NumOrWord::Num,
    ///         NumOrWord::Mismatch,
    ///         NumOrWord::Word,
    ///     ],
    /// );
    /// ```
    fn parse(session: &mut impl LexisSession) -> Self;

    fn eoi() -> Self;

    fn mismatch() -> Self;

    fn index(self) -> TokenIndex;

    fn describe(index: TokenIndex) -> Option<&'static str>;
}

/// A weak reference of the [Token] and its [Chunk](crate::lexis::Chunk) metadata inside the source
/// code.
///
/// This objects represents a long-lived lifetime independent and type independent cheap to
/// [Copy](::std::marker::Copy) safe weak reference into the source code lexical structure.
///
/// TokenRef is capable to survive source code incremental changes happening aside of the referred
/// Token.
///
/// ```rust
/// use lady_deirdre::{
///     Document,
///     lexis::{TokenRef, SimpleToken, SourceCode, TokenCursor, CodeContent},
///     syntax::NoSyntax,
/// };
///
/// let mut doc = Document::<NoSyntax<SimpleToken>>::from("foo bar baz");
///
/// // Reference to the "bar" token.
/// let bar_token: TokenRef = doc.cursor(..).token_ref(2);
///
/// assert!(bar_token.is_valid_ref(&doc));
/// assert_eq!(bar_token.deref(&doc).unwrap(), SimpleToken::Identifier);
/// assert_eq!(bar_token.string(&doc).unwrap(), "bar");
///
/// // Prepend the source code text.
/// doc.write(0..0, "123");
/// assert_eq!(doc.substring(..), "123foo bar baz");
///
/// // "bar" token is still dereferancible since the changes has happened aside of this token.
/// assert_eq!(bar_token.string(&doc).unwrap(), "bar");
///
/// // Writing inside of the "bar" token will obsolete prior TokenRef.
/// doc.write(7..8, "B");
/// assert_eq!(doc.substring(..), "123foo Bar baz");
///
/// assert!(!bar_token.is_valid_ref(&doc));
/// ```
///
/// An API user normally does not need to inspect TokenRef inner fields manually or to construct
/// a TokenRef manually unless you are working on the Crate API Extension.
///
/// For details on the Weak references framework design see [Arena](crate::arena) module
/// documentation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenRef {
    /// An [identifier](crate::arena::Id) of the [SourceCode](crate::lexis::SourceCode) instance
    /// this weakly referred Token belongs to.
    pub id: Id,

    /// An internal weak reference of the token's Chunk into
    /// the [SourceCode](crate::lexis::SourceCode) instance.
    ///
    /// This low-level [Ref](crate::arena::Ref) object used by the TokenRef under the hood to
    /// fetch particular values from the SourceCode dereferencing functions(e.g.
    /// [`SourceCode::get_token`](crate::lexis::SourceCode::get_token)).
    pub chunk_ref: Ref,
}

impl Debug for TokenRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self.is_nil() {
            false => formatter.write_fmt(format_args!("TokenRef({:?})", self.id())),
            true => formatter.write_str("TokenRef(Nil)"),
        }
    }
}

impl Identifiable for TokenRef {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl TokenRef {
    /// Returns an invalid instance of the TokenRef.
    ///
    /// This instance never resolves to valid [Token] or [token metadata](crate::lexis::Chunk).
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            chunk_ref: Ref::Nil,
        }
    }

    /// Returns `true` if this instance will never resolve to valid [Token] or
    /// [token metadata](crate::lexis::Chunk).
    ///
    /// It is guaranteed that `TokenRef::nil().is_nil()` is always `true`, but in general if
    /// this function returns `false` it is not guaranteed that provided instance is a valid
    /// reference.
    ///
    /// To determine reference validity per specified [SourceCode](crate::lexis::SourceCode)
    /// instance use [is_valid_ref](crate::lexis::TokenRef::is_valid_ref) function instead.
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.id.is_nil() || self.chunk_ref.is_nil()
    }

    /// Immutably dereferences weakly referred [Token](crate::lexis::Token) of specified
    /// [SourceCode](crate::lexis::SourceCode).
    ///
    /// Returns [None] if this TokenRef is not valid reference for specified `code` instance.
    ///
    /// Use [is_valid_ref](crate::lexis::TokenRef::is_valid_ref) to check TokenRef validity.
    ///
    /// This function uses [`SourceCode::get_token`](crate::lexis::SourceCode::get_token) function
    /// under the hood.
    #[inline(always)]
    pub fn deref<T: Token>(&self, code: &impl SourceCode<Token = T>) -> Option<T> {
        if self.id != code.id() {
            return None;
        }

        code.get_token(&self.chunk_ref)
    }

    /// Returns a [ChunkRef](crate::lexis::ChunkRef) overall token metadata object of the weakly
    /// referred token of specified [SourceCode](crate::lexis::SourceCode).
    ///
    /// Returns [None] if this TokenRef is not valid reference for specified `code` instance.
    ///
    /// Use [is_valid_ref](crate::lexis::TokenRef::is_valid_ref) to check TokenRef validity.
    ///
    /// If an API user needs just a small subset of fields from returning object it is recommended
    /// to use more specialized functions of the TokenRef instead.
    #[inline]
    pub fn chunk<'code, T: Token>(
        &self,
        code: &'code impl SourceCode<Token = T>,
    ) -> Option<ChunkRef<'code, T>> {
        if self.id != code.id() {
            return None;
        }

        let token = code.get_token(&self.chunk_ref)?;
        let site = code.get_site(&self.chunk_ref)?;
        let length = code.get_length(&self.chunk_ref)?;
        let string = code.get_string(&self.chunk_ref)?;

        Some(ChunkRef {
            token,
            site,
            length,
            string,
        })
    }

    /// Returns an absolute Unicode character index of the first character of the weakly referred
    /// token's string into specified [source code text](crate::lexis::SourceCode).
    ///
    /// Returns [None] if this TokenRef is not valid reference for specified `code` instance.
    ///
    /// Use [is_valid_ref](crate::lexis::TokenRef::is_valid_ref) to check TokenRef validity.
    ///
    /// This function uses [`SourceCode::get_site`](crate::lexis::SourceCode::get_site)
    /// function under the hood.
    #[inline(always)]
    pub fn site<T: Token>(&self, code: &impl SourceCode<Token = T>) -> Option<Site> {
        if self.id != code.id() {
            return None;
        }

        code.get_site(&self.chunk_ref)
    }

    /// Returns a token string of the weakly referred token from specified
    /// [source code text](crate::lexis::SourceCode).
    ///
    /// Returns [None] if this TokenRef is not valid reference for specified `code` instance.
    ///
    /// Use [is_valid_ref](crate::lexis::TokenRef::is_valid_ref) to check TokenRef validity.
    ///
    /// This function uses [`SourceCode::get_string`](crate::lexis::SourceCode::get_string)
    /// function under the hood.
    #[inline(always)]
    pub fn string<'code, T: Token>(
        &self,
        code: &'code impl SourceCode<Token = T>,
    ) -> Option<&'code str> {
        if self.id != code.id() {
            return None;
        }

        code.get_string(&self.chunk_ref)
    }

    /// Returns a number of Unicode characters of the string of the weakly referred token from
    /// specified [source code text](crate::lexis::SourceCode).
    ///
    /// Returns [None] if this TokenRef is not valid reference for specified `code` instance.
    ///
    /// Use [is_valid_ref](crate::lexis::TokenRef::is_valid_ref) to check TokenRef validity.
    ///
    /// This function uses [`SourceCode::get_length`](crate::lexis::SourceCode::get_length)
    /// function under the hood.
    #[inline(always)]
    pub fn length<T: Token>(&self, code: &impl SourceCode<Token = T>) -> Option<Length> {
        if self.id != code.id() {
            return None;
        }

        code.get_length(&self.chunk_ref)
    }

    /// Returns `true` if and only if referred weak Token reference belongs to specified
    /// [SourceCode](crate::lexis::SourceCode), and referred Token exists in this SourceCode
    /// instance.
    ///
    /// If this function returns `true`, all dereference function would return meaningful [Some]
    /// values, otherwise these functions return [None].
    ///
    /// This function uses [`SourceCode::contains`](crate::lexis::SourceCode::contains_chunk)
    /// function under the hood.
    #[inline(always)]
    pub fn is_valid_ref(&self, code: &impl SourceCode) -> bool {
        self.id == code.id() && code.contains_chunk(&self.chunk_ref)
    }

    /// Turns this weak reference into the Token string first character weak reference of the
    /// [site index](lexis::crate::ToSite).
    ///
    /// The returning [SiteRef](lexis::crate::SiteRef) weak reference is a valid reference if and
    /// only if TokenRef is a valid weak reference too.
    ///
    /// This function never fails, it is fine to call it on invalid references(and on the
    /// [Nil](crate::lexis::TokenRef::nil) references in particular).
    #[inline(always)]
    pub fn site_ref(self) -> SiteRef {
        SiteRef::new_chunk_start(self)
    }
}
