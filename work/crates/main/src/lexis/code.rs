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
    arena::{Identifiable, Ref},
    lexis::{Length, Site, SiteRef, ToSpan, Token, TokenCount, TokenCursor},
    std::*,
    syntax::{transduce, Node, Transducer},
};

/// A low-level interface to access and inspect lexical data of the compilation unit.
///
/// SourceCode by convenient should be implemented for the compilation unit management object such
/// as [Document](crate::Document) and [TokenBuffer](crate::lexis::TokenBuffer) objects that
/// supposed to manage code's lexical grammar structure.
///
/// This trait:
///   1. Specifies lexical grammar through the [Token](crate::lexis::SourceCode::Token) associative
///      type.
///   2. Provides general source code meta information such as text's
///      [character count](crate::lexis::SourceCode::length),
///      [token count](crate::lexis::SourceCode::token_count), etc.
///   3. Provides low-level interface to resolve higher-level weak references(such as
///      [TokenRef](crate::lexis::TokenRef) or [SiteRef](crate::lexis::SiteRef)).
///   4. Provides low-level access to the the source code Tokens through the low-level
///      iterator-alike [TokenCursor](crate::lexis::TokenCursor) interface.
///   5. Provides an entry point to the [Transducers](crate::syntax::Transducer) interface.
///
/// In practice an API user interacts with a small subset of this functionality directly.
///
/// To traverse token chunks or to access substrings of arbitrary spans the user can utilize a
/// higher-level [CodeContent](crate::lexis::CodeContent) auto-implemented extension over the
/// SourceCode.
///
/// To implement an extension library to this Crate with the source code storages of alternative
/// designs, you can implement this trait over these objects. In this case these new objects will be
/// able to interact with existing [Token](crate::lexis::Token) implementations, and the weak
/// references belong to them will work transparently with other conventional weak references.
pub trait SourceCode: Identifiable {
    /// Specifies programming language lexical grammar.
    ///
    /// See [Token](crate::lexis::Token) for details.
    type Token: Token;

    /// Specifies a low-level iterator-alike type that traverses through the source code tokens.
    ///
    /// See [TokenCursor](crate::lexis::TokenCursor) for details.
    type Cursor<'code>: TokenCursor<'code, Token = Self::Token>
    where
        Self: 'code;

    /// Returns `true` if the token referred by specified low-level `chunk_ref` weak reference
    /// exists in this source code instance.
    ///
    /// This is a low-level API used by the higher-level [TokenRef](crate::lexis::TokenRef) and
    /// [SiteRef](crate::lexis::SiteRef) weak references under the hood. An API user normally don't
    /// need to call this function directly.
    fn contains(&self, chunk_ref: &Ref) -> bool;

    /// Immutably dereferences a [Token](crate::lexis::Token) instance by specified low-level
    /// `chunk_ref` weak reference.
    ///
    /// Returns [None] if referred Token Chunk does not exist in this instance.
    ///
    /// This is a low-level API used by the higher-level [TokenRef](crate::lexis::TokenRef)
    /// weak reference under the hood. An API user normally does not need to call this function
    /// directly.
    fn get_token(&self, chunk_ref: &Ref) -> Option<&Self::Token>;

    /// Mutably dereferences a [Token](crate::lexis::Token) instance by specified low-level
    /// `chunk_ref` weak reference.
    ///
    /// Returns [None] if referred Token Chunk does not exist in this instance.
    ///
    /// Even though the SourceCode provides a way to mutate Token instances inside the source code
    /// lexical structure, it is recommended to avoid replacing of these instances with the token
    /// instances of different lexical kinds(with different enum variants in particular).
    /// Such replacement is not an undefined behavior, but it could lead to logical errors in
    /// further syntax parsing/re-parsing stages.
    ///
    /// This is a low-level API used by the higher-level [TokenRef](crate::lexis::TokenRef)
    /// weak reference under the hood. An API user normally does not need to call this function
    /// directly.
    fn get_token_mut(&mut self, chunk_ref: &Ref) -> Option<&mut Self::Token>;

    /// Returns absolute character index of the [Token](crate::lexis::Token) substring inside this
    /// source code text by specified low-level `chunk_ref` weak reference.
    ///
    /// Returns [None] if referred Token Chunk does not exist in this instance.
    ///
    /// This is a low-level API used by the higher-level [TokenRef](crate::lexis::TokenRef) and
    /// [SiteRef](crate::lexis::SiteRef) weak reference under the hood. An API user normally does
    /// not need to call this function directly.
    fn get_site(&self, chunk_ref: &Ref) -> Option<Site>;

    /// Returns a substring of the [Token](crate::lexis::Token) inside this source code text by
    /// specified low-level `chunk_ref` weak reference.
    ///
    /// Returns [None] if referred Token Chunk does not exist in this instance.
    ///
    /// This is a low-level API used by the higher-level [TokenRef](crate::lexis::TokenRef)
    /// weak reference under the hood. An API user normally does not need to call this function
    /// directly.
    fn get_string(&self, chunk_ref: &Ref) -> Option<&str>;

    /// Returns character count of the [Token](crate::lexis::Token) substring inside this
    /// source code text by specified low-level `chunk_ref` weak reference.
    ///
    /// Returns [None] if referred Token Chunk does not exist in this instance.
    ///
    /// This is a low-level API used by the higher-level [TokenRef](crate::lexis::TokenRef)
    /// weak reference under the hood. An API user normally does not need to call this function
    /// directly.
    fn get_length(&self, chunk_ref: &Ref) -> Option<Length>;

    /// Returns a [TokenCursor](crate::lexis::TokenCursor) instance to traverse tokens and
    /// their metadata that "touch" specified `span`.
    ///
    /// Span "touching" means such tokens that their substring characters lie inside, intersect
    /// with, or adjacent to this [Span](crate::lexis::ToSpan).
    ///
    /// ```rust
    /// use lady_deirdre::lexis::{TokenBuffer, SourceCode, SimpleToken, TokenCursor};
    ///
    /// let buf = TokenBuffer::<SimpleToken>::from("foo bar baz");
    ///
    /// // `..` span covers all tokens.
    /// assert_eq!(collect(buf.cursor(..)), vec!["foo", " ", "bar", " ", "baz"]);
    ///
    /// // `0..0` span is adjacent to the first token only.
    /// assert_eq!(collect(buf.cursor(0..0)), vec!["foo"]);
    ///
    /// // `3..5` span is adjacent to the first token, covers the second token, and intersects with
    /// // the third token.
    /// assert_eq!(collect(buf.cursor(3..5)), vec!["foo", " ", "bar"]);
    ///
    /// fn collect(mut cursor: <TokenBuffer<SimpleToken> as SourceCode>::Cursor<'_>) -> Vec<String>
    /// {
    ///     let mut result = Vec::new();
    ///
    ///     while let Some(string) = cursor.string(0) {
    ///         result.push(string.to_string());
    ///         let _ = cursor.advance();
    ///     }
    ///
    ///     result
    /// }
    /// ```
    ///
    /// This is a low-level API function. To iterate through the spanned chunks an API user
    /// encouraged to use a higher-level [CodeContent::chunks](crate::lexis::CodeContent::chunks)
    /// function instead that returns a more convenient iterator over the
    /// [ChunkRef](crate::lexis::ChunkRef) objects.
    ///
    /// ```rust
    /// use lady_deirdre::lexis::{TokenBuffer, CodeContent, SimpleToken, TokenCursor, ChunkRef};
    ///
    /// let buf = TokenBuffer::<SimpleToken>::from("foo bar baz");
    ///
    /// assert_eq!(
    ///     buf
    ///         .chunks(3..5)
    ///         .map(|chunk_ref: ChunkRef<'_, SimpleToken>| chunk_ref.string.to_string())
    ///         .collect::<Vec<_>>(),
    ///     vec!["foo", " ", "bar"],
    /// );
    /// ```
    fn cursor(&self, span: impl ToSpan) -> Self::Cursor<'_>;

    /// Returns a [SiteRef](crate::lexis::SiteRef) instance that always valid and always resolves to
    /// the source code [length](crate::lexis::SourceCode::length) value.
    #[inline(always)]
    fn end_site_ref(&self) -> SiteRef {
        SiteRef::new_code_end(*self.id())
    }

    /// Returns a total number of UTF-8 characters inside the source code text.
    fn length(&self) -> Length;

    /// Returns a total number of tokens inside the source code lexical structure.
    fn token_count(&self) -> TokenCount;

    /// Returns `true` if the source code text is empty string.
    ///
    /// If the source code is empty, there are no tokens held by this instance.
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.length() == 0
    }

    /// Runs provided `transducer` over this SourceCode content returning Transducer's result
    /// value.
    ///
    /// The [Transducers Framework](crate::syntax::Transducer) in particular provides a way to
    /// implement source code formatters.
    #[inline(always)]
    fn transduce<N, R, Tr>(&self, transducer: Tr) -> R
    where
        Self: Sized,
        N: Node<Token = Self::Token>,
        Tr: Transducer<N, Self, R>,
    {
        transduce(self, transducer)
    }
}
