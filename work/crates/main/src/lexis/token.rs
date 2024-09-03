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

extern crate lady_deirdre_derive;

use std::fmt::{Debug, Formatter};

pub use lady_deirdre_derive::Token;

use crate::{
    arena::{Entry, Id, Identifiable},
    lexis::{
        Chunk,
        Length,
        LexisSession,
        Site,
        SiteRef,
        SiteSpan,
        SourceCode,
        ToSpan,
        TokenRule,
        EOI,
    },
    syntax::{NodeRef, PolyRef, PolyVariant, RefKind, NIL_NODE_REF},
    units::CompilationUnit,
};

/// A [TokenRef] reference that does not point to any token.
///
/// The value of this static equals to the [TokenRef::nil] value.
pub static NIL_TOKEN_REF: TokenRef = TokenRef::nil();

/// A number of tokens.
pub type TokenCount = usize;

/// A type of the source code token.
///
/// Typically, this trait should be implemented on the `#[repr(u8)]` and [Copy]
/// enum types, where each enum variant without fields represents an individual
/// token kind.
///
/// The trait provides language-agnostic functions to reveal metadata about the
/// lexis of the language, such as [name](Token::name) to get token's name,
/// or the [eoi](Token::eoi) function that returns a token kind that denoting
/// the end-of-input token of this language.
///
/// The [Token::scan] function serves as the lexical scanner of a single token
/// in the source code text, and the constructor of this token instance.
///
/// Essentially, this trait defines the lexical component of the programming
/// language grammar.
///
/// You are encouraged to use the companion [Token](lady_deirdre_derive::Token)
/// derive macro to implement the lexical grammar on enum types in terms
/// of the scanner's regular expressions.
pub trait Token: Copy + Eq + Send + Sync + Sized + 'static {
    /// Specifies the minimum length of the token in Unicode chars that is
    /// guaranteed to be unambiguous in terms of the preceding tokens.
    ///
    /// Usually, this value equals 1, meaning that any token is unambiguous.
    ///
    /// Zero is an invalid value for LOOKAHEAD, because in Lady Deirdre,
    /// there are no tokens of zero length.
    ///
    /// When using the [Token](lady_deirdre_derive::Token) macro, this value
    /// is either set to 1 as default or overridden by
    /// the `#[lookback(...)]` attriubute:
    ///
    /// ```ignore
    /// #[derive(Token)]
    /// #[lookback(3)]
    /// enum MyToken {}
    /// ```
    const LOOKBACK: Length;

    /// Scans a single token from the beginning of the input text.
    ///
    /// The `session` parameter of type [LexisSession] provides access
    /// to the byte stream of a valid UTF-8 encoded text that needs to be
    /// scanned.
    ///
    /// It returns an instance of the Token that denotes scanning result.
    ///
    /// If the function fails to recognize any token, it returns
    /// the [mismatch](Self::mismatch) token. Otherwise, it returns
    /// a non-mismatch token that denotes the scanned token kind.
    ///
    /// If the function returns a non-mismatch token it must call the
    /// [LexisSession::submit] at least once, and the last call of that function
    /// denotes the end-boundary in the byte stream of where the scan ends.
    ///
    /// The underlying algorithm always scans the maximum-wide token that
    /// matches the beginning of the input stream. Non-mismatched tokens
    /// are tokens of non-zero length.
    ///
    /// Typically, you don't need to call this function manually. It is the
    /// responsibility of the compilation unit manager
    /// (e.g., [Document](crate::units::Document)) to decide when to call this
    /// function.
    ///
    /// For a detailed specification of the lexical parsing process,
    /// refer to the [LexisSession] documentation.
    fn scan(session: &mut impl LexisSession) -> Self;

    /// Returns a Token instance that denotes the end-of-input.
    ///
    /// This is a special kind of token that normally does not exist in the
    /// token streams. It is used by various functions around the crate API to
    /// denote the end of the stream inputs.
    ///
    /// Additionally, the eoi token of any language has a predefined token
    /// [rule](Self::rule) equal to [EOI].
    ///
    /// When using the [Token](lady_deirdre_derive::Token) macro, this token
    /// variant is denoted by the `0` discriminant value:
    ///
    /// ```ignore
    /// #[derive(Token)]
    /// #[repr(u8)]
    /// enum MyToken {
    ///     EOI = 0,
    /// }
    /// ```
    fn eoi() -> Self;

    /// Returns a Token instance that denotes a fragment of the source code text
    /// that does not belong to a known set of lexical tokens of the programming
    /// language.
    ///
    /// In principal, lexical scanning is an infallible process. The lexical
    /// scanner guarantees to be able to split any source code text into
    /// a sequence of tokens. Since a token set of a particular programming
    /// language may be non-exhaustive, this kind of token is used as a sink
    /// for the text fragments that the scanner unable to classify.
    ///
    /// Additionally, the mismatch token of any language has a predefined token
    /// [rule](Self::rule) equal to [MISMATCH](crate::lexis::MISMATCH).
    ///
    /// When using the [Token](lady_deirdre_derive::Token) macro, this token
    /// variant is denoted by the `1` discriminant value:
    ///
    /// ```ignore
    /// #[derive(Token)]
    /// #[repr(u8)]
    /// enum MyToken {
    ///     Mismatch = 1,
    /// }
    /// ```
    fn mismatch() -> Self;

    /// Returns a numeric representation of this token.
    ///
    /// Usually, this value equals the enum's variant discriminant:
    ///
    /// ```ignore
    /// #[derive(Token)]
    /// enum MyToken {
    ///     #[rule()]
    ///     Variant1 = 20, // self.rule() == 20
    ///
    ///     #[rule()]
    ///     Variant2 = 30,  // self.rule() == 30
    /// }
    /// ```
    fn rule(self) -> TokenRule;

    /// A debug name of this token.
    ///
    /// Returns None if this feature is disabled for this token instance.
    ///
    /// When using the [Token](lady_deirdre_derive::Token) macro, this function
    /// returns the stringified variant's name:
    ///
    /// ```ignore
    /// #[derive(Token)]
    /// enum MyToken {
    ///     #[rule()]
    ///     Variant {}, // self.name() == Some("Variant")
    /// }
    /// ```
    #[inline(always)]
    fn name(self) -> Option<&'static str> {
        Self::rule_name(self.rule())
    }

    /// An end-user display description of this token.
    ///
    /// Returns None if this feature is disabled for this token instance.
    ///
    /// This function is intended to be used for the syntax errors formatting.
    ///
    /// When using the [Token](lady_deirdre_derive::Token) macro, this function
    /// returns what you have specified with the `#[describe(...)]` attribute:
    ///
    /// ```ignore
    /// #[derive(Token)]
    /// enum MyToken {
    ///     // self.describe(false) == Some("short")
    ///     // self.describe(true) == Some("verbose")
    ///     #[rule()]
    ///     #[describe("short", "verbose")]
    ///     Variant {},
    /// }
    /// ```
    ///
    /// The difference between the short (`verbose` is false) and verbose
    /// (verbose is `true`) descriptions is that the short version represents
    /// a "class" of tokens, while the verbose version provides a more
    /// detailed text specific to this particular token.
    ///
    /// For example, a short description of the Plus and Mul operator tokens
    /// would simply be "operator", whereas, for verbose versions
    /// this function might returns something like "sum" and "mul".
    #[inline(always)]
    fn describe(self, verbose: bool) -> Option<&'static str> {
        Self::rule_description(self.rule(), verbose)
    }

    /// A debug name of the token rule.
    ///
    /// The returning value is the same as `self.name(self.rule())`.
    ///
    /// See [name](Self::name) for details.
    fn rule_name(rule: TokenRule) -> Option<&'static str>;

    /// An end-user display description of the token rule.
    ///
    /// The returning value is the same as `self.describe(self.rule(), verbose)`.
    ///
    /// See [describe](Self::describe) for details.
    fn rule_description(rule: TokenRule, verbose: bool) -> Option<&'static str>;
}

/// A globally unique reference of the [token](Token) in the source code.
///
/// Each [source code](SourceCode) token could be uniquely addressed within
/// a pair of the [Id] and [Entry], where the identifier uniquely addresses
/// a specific compilation unit instance (source code), and
/// the entry part addresses a token within this code.
///
/// Essentially, TokenRef is a composite index.
///
/// Both components of this index form a unique pair
/// (within the current process), because each compilation unit has a unique
/// identifier, and the tokens within the source code always receive unique
/// [Entry] indices within the source code.
///
/// If the token instance has been removed from the source code over time,
/// new tokens within this source code will never occupy the same TokenRef
/// object, but the TokenRef referred to the removed Token would
/// become _invalid_.
///
/// The [nil](TokenRef::nil) TokenRefs are special references that are considered
/// to be always invalid (they intentionally don't refer to any token within
/// any source code).
///
/// Two distinct instances of the nil TokenRef are always equal.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenRef {
    /// An identifier of the source code.
    pub id: Id,

    /// A versioned index of the token instance within the source code.
    pub entry: Entry,
}

impl Debug for TokenRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        match self.is_nil() {
            false => formatter.write_fmt(format_args!(
                "TokenRef(id: {:?}, entry: {:?})",
                self.entry, self.id,
            )),
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

impl Default for TokenRef {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl PolyRef for TokenRef {
    #[inline(always)]
    fn kind(&self) -> RefKind {
        RefKind::Token
    }

    #[inline(always)]
    fn is_nil(&self) -> bool {
        self.id.is_nil() || self.entry.is_nil()
    }

    #[inline(always)]
    fn as_variant(&self) -> PolyVariant {
        PolyVariant::Token(*self)
    }

    #[inline(always)]
    fn as_token_ref(&self) -> &TokenRef {
        self
    }

    #[inline(always)]
    fn as_node_ref(&self) -> &NodeRef {
        &NIL_NODE_REF
    }

    #[inline(always)]
    fn span(&self, unit: &impl CompilationUnit) -> Option<SiteSpan> {
        self.chunk(unit)?.to_site_span(unit)
    }
}

impl TokenRef {
    /// Returns a TokenRef that intentionally does not refer to any token within
    /// any source code.
    ///
    /// If you need just a static reference to the nil TokenRef, use
    /// the predefined [NIL_TOKEN_REF] static.
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            entry: Entry::nil(),
        }
    }

    /// Returns a copy of a source code token referred to by this TokenRef.
    ///
    /// Returns None if this TokenRef is not valid for the specified `code`.
    #[inline(always)]
    pub fn deref<T: Token>(&self, code: &impl SourceCode<Token = T>) -> Option<T> {
        if self.id != code.id() {
            return None;
        }

        code.get_token(&self.entry)
    }

    /// Returns metadata (a "chunk") of the token referred to by this TokenRef.
    ///
    /// The [Chunk] metadata includes a copy of the [Token] instance, its
    /// absolute [site](Site) in the source code, the length (in Unicode
    /// chars) of the token's string, and the reference to the source code
    /// text fragment covered by this token.
    ///
    /// Returns None if this TokenRef is not valid for the specified `code`.
    #[inline]
    pub fn chunk<'code, T: Token>(
        &self,
        code: &'code impl SourceCode<Token = T>,
    ) -> Option<Chunk<'code, T>> {
        if self.id != code.id() {
            return None;
        }

        let token = code.get_token(&self.entry)?;
        let site = code.get_site(&self.entry)?;
        let length = code.get_length(&self.entry)?;
        let string = code.get_string(&self.entry)?;

        Some(Chunk {
            token,
            site,
            length,
            string,
        })
    }

    /// Returns a [site](Site) (an absolute offset in Unicode chars) of a source
    /// code token referred to by this TokenRef.
    ///
    /// Returns None if this TokenRef is not valid for the specified `code`.
    #[inline(always)]
    pub fn site<T: Token>(&self, code: &impl SourceCode<Token = T>) -> Option<Site> {
        if self.id != code.id() {
            return None;
        }

        code.get_site(&self.entry)
    }

    /// Returns a reference to the source code text fragment covered by the
    /// token referred to by this TokenRef.
    ///
    /// Returns None if this TokenRef is not valid for the specified `code`.
    #[inline(always)]
    pub fn string<'code, T: Token>(
        &self,
        code: &'code impl SourceCode<Token = T>,
    ) -> Option<&'code str> {
        if self.id != code.id() {
            return None;
        }

        code.get_string(&self.entry)
    }

    /// Returns the [length](Length) (in Unicode chars) of a source code
    /// fragment covered by the token referred to by this TokenRef.
    ///
    /// Returns None if this TokenRef is not valid for the specified `code`.
    #[inline(always)]
    pub fn length<T: Token>(&self, code: &impl SourceCode<Token = T>) -> Option<Length> {
        if self.id != code.id() {
            return None;
        }

        code.get_length(&self.entry)
    }

    /// Returns a numeric representation of the token referred to by this
    /// TokenRef.
    ///
    /// Returns [EOI] if this TokenRef is not valid for the specified `code`.
    ///
    /// See [Token::rule] for details.
    #[inline(always)]
    pub fn rule(&self, code: &impl SourceCode) -> TokenRule {
        self.deref(code).map(|token| token.rule()).unwrap_or(EOI)
    }

    /// Returns a debug name of the referred token.
    ///
    /// Returns None if this TokenRef is not valid for the specified `code`,
    /// or if the token instance does not have a name.
    ///
    /// See [Token::name] for details.
    #[inline(always)]
    pub fn name<T: Token>(&self, code: &impl SourceCode<Token = T>) -> Option<&'static str> {
        self.deref(code).map(T::name).flatten()
    }

    /// Returns an end-user display description of the referred token.
    ///
    /// Returns None if this TokenRef is not valid for the specified `code`,
    /// or if the token instance does not have a description.
    ///
    /// See [Token::describe] for details.
    #[inline(always)]
    pub fn describe<T: Token>(
        &self,
        code: &impl SourceCode<Token = T>,
        verbose: bool,
    ) -> Option<&'static str> {
        self.deref(code)
            .map(|token| token.describe(verbose))
            .flatten()
    }

    /// Returns true if the token referred to by this TokenRef exists in the specified
    /// `code`.
    #[inline(always)]
    pub fn is_valid_ref(&self, code: &impl SourceCode) -> bool {
        self.id == code.id() && code.has_chunk(&self.entry)
    }

    /// Returns a [SiteRef] site reference of this token reference that points
    /// to the start site of the token.
    #[inline(always)]
    pub fn site_ref(self) -> SiteRef {
        SiteRef::start_of(self)
    }
}
