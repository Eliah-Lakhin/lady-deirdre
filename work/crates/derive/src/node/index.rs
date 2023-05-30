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

use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    LitInt,
    Result,
};

use crate::utils::error;

#[derive(Clone, Copy)]
pub(super) enum Index {
    Generated(Span, u16),
    Overridden(Span, u16),
}

impl PartialEq for Index {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for Index {}

impl Hash for Index {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

impl Ord for Index {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.get().cmp(&other.get())
    }
}

impl PartialOrd for Index {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Parse for Index {
    fn parse(input: ParseStream) -> Result<Self> {
        let index = input.parse::<LitInt>()?;
        let span = index.span();
        let index = index.base10_parse::<u16>()?;

        if index == 0 {
            return Err(error!(span, "Zero index reserved for the Root rule.",));
        }

        Ok(Index::Overridden(span, index))
    }
}

impl ToTokens for Index {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Index::Generated(span, index) => quote_spanned!(*span=> #index).to_tokens(tokens),
            Index::Overridden(span, index) => quote_spanned!(*span=> #index).to_tokens(tokens),
        }
    }
}

impl Index {
    #[inline(always)]
    pub(super) fn get(&self) -> u16 {
        match self {
            Self::Generated(_, index) => *index,
            Self::Overridden(_, index) => *index,
        }
    }
}
