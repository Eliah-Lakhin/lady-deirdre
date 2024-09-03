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
    fmt::{Debug, Display, Formatter},
    iter::FusedIterator,
    marker::PhantomData,
};

use crate::lexis::{Token, TokenCount};

/// A numeric representation of the [Token].
///
/// When the Token trait is implemented on the enum type, usually this number
/// is equal to the variant discriminant.
pub type TokenRule = u8;

/// Denotes the end of the token stream.
pub const EOI: TokenRule = 0;

/// Denotes a [Token] that does not belong to any class of the lexical grammar
/// of a programming language.
pub const MISMATCH: TokenRule = 1;

/// A static set of tokens without entries.
///
/// The value of this static equals to the [TokenSet::empty] value.
pub static EMPTY_TOKEN_SET: TokenSet = TokenSet::empty();

/// A static set of tokens that includes all programming language tokens except
/// the [EOI] token.
///
/// The value of this static equals to the [TokenSet::all] value.
pub static FULL_TOKEN_SET: TokenSet = TokenSet::all();

/// A set of lexical token [rules](TokenRule) of fixed size.
///
/// The set stores all entries in place, and the set object has a fixed size.
/// This object is capable of addressing any possible sets of tokens of any
/// programming language.
///
/// Under the hood, this object is a bit mask of 256 bits, since Lady Deirdre
/// allows specifying up to 254 unique tokens plus [EOI] and [MISMATCH] tokens.
///
/// Therefore, the size of this object is 32 bytes, and most methods of
/// this set are const functions of `O(1)` complexity.
///
/// The object is assumed to be constructed in a const context as a static value
/// upfront to reduce the runtime overhead.
#[repr(transparent)]
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenSet {
    mask: [u8; 32],
}

impl Debug for TokenSet {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.debug_set().entries(self.into_iter()).finish()
    }
}

impl<'a> IntoIterator for &'a TokenSet {
    type Item = TokenRule;
    type IntoIter = TokenSetIterator<'a>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        TokenSetIterator {
            token: Some(0),
            set: self,
        }
    }
}

impl FromIterator<TokenRule> for TokenSet {
    #[inline]
    fn from_iter<I: IntoIterator<Item = TokenRule>>(iter: I) -> Self {
        let mut result = Self::empty();

        for token in iter {
            let (index, bit) = Self::index_of(token);

            result.mask[index] |= bit;
        }

        result
    }
}

impl<T: Token> FromIterator<T> for TokenSet {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut result = Self::empty();

        for token in iter {
            let (index, bit) = Self::index_of(token.rule());

            result.mask[index] |= bit;
        }

        result
    }
}

impl TokenSet {
    /// Creates a token set without entries.
    ///
    /// If you need just a static empty token set, use the predefined
    /// [EMPTY_TOKEN_SET] static.
    #[inline(always)]
    pub const fn empty() -> Self {
        Self { mask: [0; 32] }
    }

    /// Creates a token set that includes all possible entries of any
    /// programming language except the [EOI] token.
    ///
    /// If you need just a static full token set, use the predefined
    /// [FULL_TOKEN_SET] static.
    #[inline(always)]
    pub const fn all() -> Self {
        Self { mask: [0xFF; 32] }.exclude(EOI)
    }

    /// Creates a token set that includes all tokens within the specified
    /// `tokens` slice.
    pub const fn inclusive(tokens: &[TokenRule]) -> Self {
        let mut set = Self::empty();

        let mut slice_index = 0;

        while slice_index < tokens.len() {
            let token = tokens[slice_index];

            let (index, bit) = Self::index_of(token);

            set.mask[index] |= bit;

            slice_index += 1;
        }

        set
    }

    /// Creates a token set that includes any token within any programming
    /// language except the [EOI] token, and the tokens within the specified
    /// `tokens` slice.
    pub const fn exclusive(tokens: &[TokenRule]) -> Self {
        let mut set = Self::all();

        let mut slice_index = 0;

        while slice_index < tokens.len() {
            let token = tokens[slice_index];

            let (index, bit) = Self::index_of(token);

            set.mask[index] &= 0xFF ^ bit;

            slice_index += 1;
        }

        set
    }

    /// Returns true if the token set contains the specified `token`.
    pub const fn contains(&self, token: TokenRule) -> bool {
        let (index, bit) = Self::index_of(token);

        self.mask[index] & bit > 0
    }

    /// Consumes this TokenSet instance and returns a new token set that
    /// includes the `token`.
    #[inline(always)]
    pub const fn include(mut self, token: TokenRule) -> Self {
        let (index, bit) = Self::index_of(token);

        self.mask[index] |= bit;

        self
    }

    /// Consumes this TokenSet instance and returns a new token set that
    /// includes all token from the `tokens` slice.
    #[inline(always)]
    pub const fn include_all(mut self, tokens: &[TokenRule]) -> Self {
        let mut slice_index = tokens.len();

        while slice_index < tokens.len() {
            let (index, bit) = Self::index_of(tokens[slice_index]);

            self.mask[index] |= bit;

            slice_index += 1;
        }

        self
    }

    /// Consumes this TokenSet instance and returns a new token set without
    /// the `token`.
    #[inline(always)]
    pub const fn exclude(mut self, token: TokenRule) -> Self {
        let (index, bit) = Self::index_of(token);

        self.mask[index] &= 0xFF ^ bit;

        self
    }

    /// Consumes this TokenSet instance and returns a new token set without
    /// the tokens from the `tokens` slice.
    #[inline(always)]
    pub const fn exclude_all(mut self, tokens: &[TokenRule]) -> Self {
        let mut slice_index = tokens.len();

        while slice_index < tokens.len() {
            let (index, bit) = Self::index_of(tokens[slice_index]);

            self.mask[index] &= 0xFF ^ bit;

            slice_index += 1;
        }

        self
    }

    /// Returns true if the TokenSet has no entries.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        let mut high = 0;

        while high < 32 {
            if self.mask[high] != 0 {
                return false;
            }

            high += 1;
        }

        true
    }

    /// Returns the number of entries in this TokenSet instance.
    #[inline(always)]
    pub const fn len(&self) -> TokenCount {
        let mut length = 0;

        let mut high = 0;
        while high < 32 {
            let mask = self.mask[high];

            let mut low = 0;
            while low < 8 {
                if mask & (1 << low) > 0 {
                    length += 1;
                }

                low += 1;
            }

            high += 1;
        }

        length
    }

    /// Consumes this TokenSet instance and returns a new token set without the
    /// former set entries, but with the entries that have not been included
    /// in the former set.
    #[inline(always)]
    pub const fn invert(mut self) -> Self {
        let mut high = 0;

        while high < 32 {
            self.mask[high] ^= 0xFF;
            high += 1
        }

        self
    }

    /// Consumes two instances of the TokenSet and returns a new token set
    /// that includes all tokens belonging to both instances.
    #[inline(always)]
    pub const fn intersect(mut self, other: Self) -> Self {
        let mut high = 0;

        while high < 32 {
            self.mask[high] &= other.mask[high];
            high += 1
        }

        self
    }

    /// Consumes two instances of the TokenSet and returns a new token set
    /// that includes all tokens the belong to at least one of these instances.
    #[inline(always)]
    pub const fn union(mut self, other: Self) -> Self {
        let mut high = 0;

        while high < 32 {
            self.mask[high] |= other.mask[high];
            high += 1
        }

        self
    }

    /// Returns an object that displays all entries within this token set.
    ///
    /// The `T` generic parameter specifies the lexical grammar of
    /// the programming language (see [Token](crate::lexis::Token)).
    ///
    /// The underlying displaying algorithm uses
    /// the [rule_name](Token::rule_name) function to determine
    /// the tokens' display names.
    #[inline(always)]
    pub fn display<T: Token>(&self) -> impl Debug + Display + '_ {
        struct DisplayTokenSet<'a, T> {
            set: &'a TokenSet,
            _token: PhantomData<T>,
        }

        impl<'a, T: Token> Debug for DisplayTokenSet<'a, T> {
            #[inline(always)]
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                Display::fmt(self, formatter)
            }
        }

        impl<'a, T: Token> Display for DisplayTokenSet<'a, T> {
            fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
                let mut vector = Vec::with_capacity(0xFF);

                for token in self.set {
                    if let Some(name) = T::rule_name(token) {
                        vector.push(name);
                    }
                }

                vector.sort();

                let mut debug_set = formatter.debug_set();

                for description in vector {
                    debug_set.entry(&format_args!("'{description}'"));
                }

                debug_set.finish()
            }
        }

        DisplayTokenSet {
            set: self,
            _token: PhantomData::<T>,
        }
    }

    #[inline(always)]
    const fn index_of(rule: TokenRule) -> (usize, u8) {
        (rule as usize / 8, 1 << (rule % 8))
    }
}

pub struct TokenSetIterator<'a> {
    token: Option<TokenRule>,
    set: &'a TokenSet,
}

impl<'a> Iterator for TokenSetIterator<'a> {
    type Item = TokenRule;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(token) = self.token {
            self.token = match token == TokenRule::MAX {
                true => None,
                false => Some(token + 1),
            };

            if self.set.contains(token) {
                return Some(token);
            }
        }

        None
    }
}

impl<'a> FusedIterator for TokenSetIterator<'a> {}
