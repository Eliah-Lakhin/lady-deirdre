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
    lexis::{Token, TokenCount},
    std::*,
};

pub type TokenIndex = u8;

pub const EOI: TokenIndex = 0;
pub const MISMATCH: TokenIndex = 1;

pub static EMPTY_TOKEN_SET: TokenSet = TokenSet::empty();
pub static FULL_TOKEN_SET: TokenSet = TokenSet::all();

#[repr(transparent)]
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenSet {
    mask: [u8; 32],
}

impl Debug for TokenSet {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.debug_set().entries(self.into_iter()).finish()
    }
}

impl<'a> IntoIterator for &'a TokenSet {
    type Item = TokenIndex;
    type IntoIter = TokenSetIterator<'a>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        TokenSetIterator {
            token: Some(0),
            set: self,
        }
    }
}

impl FromIterator<TokenIndex> for TokenSet {
    #[inline]
    fn from_iter<I: IntoIterator<Item = TokenIndex>>(iter: I) -> Self {
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
            let (index, bit) = Self::index_of(token.index());

            result.mask[index] |= bit;
        }

        result
    }
}

impl TokenSet {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self { mask: [0; 32] }
    }

    #[inline(always)]
    pub const fn all() -> Self {
        Self { mask: [0xFF; 32] }
    }

    pub const fn inclusive(tokens: &[TokenIndex]) -> Self {
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

    pub const fn exclusive(tokens: &[TokenIndex]) -> Self {
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

    pub const fn contains(&self, token: TokenIndex) -> bool {
        let (index, bit) = Self::index_of(token);

        self.mask[index] & bit > 0
    }

    #[inline(always)]
    pub const fn include(mut self, token: TokenIndex) -> Self {
        let (index, bit) = Self::index_of(token);

        self.mask[index] |= bit;

        self
    }

    #[inline(always)]
    pub const fn include_all(mut self, tokens: &[TokenIndex]) -> Self {
        let mut slice_index = tokens.len();

        while slice_index < tokens.len() {
            let (index, bit) = Self::index_of(tokens[slice_index]);

            self.mask[index] |= bit;

            slice_index += 1;
        }

        self
    }

    #[inline(always)]
    pub const fn exclude(mut self, token: TokenIndex) -> Self {
        let (index, bit) = Self::index_of(token);

        self.mask[index] &= 0xFF ^ bit;

        self
    }

    #[inline(always)]
    pub const fn exclude_all(mut self, tokens: &[TokenIndex]) -> Self {
        let mut slice_index = tokens.len();

        while slice_index < tokens.len() {
            let (index, bit) = Self::index_of(tokens[slice_index]);

            self.mask[index] &= 0xFF ^ bit;

            slice_index += 1;
        }

        self
    }

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

    #[inline(always)]
    pub const fn length(&self) -> TokenCount {
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

    #[inline(always)]
    pub const fn invert(mut self) -> Self {
        let mut high = 0;

        while high < 32 {
            self.mask[high] ^= 0xFF;
            high += 1
        }

        self
    }

    #[inline(always)]
    pub const fn intersect(mut self, other: Self) -> Self {
        let mut high = 0;

        while high < 32 {
            self.mask[high] &= other.mask[high];
            high += 1
        }

        self
    }

    #[inline(always)]
    pub const fn union(mut self, other: Self) -> Self {
        let mut high = 0;

        while high < 32 {
            self.mask[high] |= other.mask[high];
            high += 1
        }

        self
    }

    #[inline(always)]
    pub fn display<T: Token>(&self) -> impl Display + '_ {
        struct DisplayTokenSet<'a, T> {
            set: &'a TokenSet,
            _token: PhantomData<T>,
        }

        impl<'a, T: Token> Display for DisplayTokenSet<'a, T> {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                let mut vector = Vec::with_capacity(0xFF);

                for token in self.set {
                    if let Some(description) = T::describe(token) {
                        vector.push(description);
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
    const fn index_of(token: TokenIndex) -> (usize, u8) {
        (token as usize / 8, 1 << (token % 8))
    }
}

pub struct TokenSetIterator<'a> {
    token: Option<TokenIndex>,
    set: &'a TokenSet,
}

impl<'a> Iterator for TokenSetIterator<'a> {
    type Item = TokenIndex;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(token) = self.token {
            self.token = match token == TokenIndex::MAX {
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
