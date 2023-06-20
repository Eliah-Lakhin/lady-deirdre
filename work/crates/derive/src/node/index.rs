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

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseStream},
    LitInt,
    Result,
};

use crate::utils::{error, system_panic};

#[derive(Clone)]
pub(super) enum Index {
    Generated(Span, u16),
    Overridden(Span, u16),
    Named(Ident, Option<u16>),
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
        let lookahead = input.lookahead1();

        if lookahead.peek(LitInt) {
            let index = input.parse::<LitInt>()?;
            let span = index.span();
            let index = index.base10_parse::<u16>()?;

            if index == 0 {
                return Err(error!(span, "Zero index reserved for the Root rule.",));
            }

            return Ok(Index::Overridden(span, index));
        }

        if lookahead.peek(Ident::peek_any) {
            let name = input.parse::<Ident>()?;

            let index = match input.peek(Token![=]) {
                false => None,

                true => {
                    let _ = input.parse::<Token![=]>();

                    let index = input.parse::<LitInt>()?;
                    let span = index.span();
                    let index = index.base10_parse::<u16>()?;

                    if index == 0 {
                        return Err(error!(span, "Zero index reserved for the Root rule.",));
                    }

                    Some(index)
                }
            };

            return Ok(Index::Named(name, index));
        }

        Err(lookahead.error())
    }
}

impl ToTokens for Index {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Index::Generated(span, index) => quote_spanned!(*span=> #index).to_tokens(tokens),
            Index::Overridden(span, index) => quote_spanned!(*span=> #index).to_tokens(tokens),
            Index::Named(name, index) => {
                let span = name.span();

                match index {
                    None => quote_spanned!(span=> Self::#name).to_tokens(tokens),
                    Some(index) => quote_spanned!(span=> #index).to_tokens(tokens),
                }
            }
        }
    }
}

impl Index {
    #[inline(always)]
    pub(super) fn get(&self) -> u16 {
        match self {
            Self::Generated(_, index) => *index,
            Self::Overridden(_, index) => *index,
            Self::Named(name, index) => match index {
                Some(index) => *index,
                None => system_panic!("Unset index {name}.",),
            },
        }
    }

    #[inline(always)]
    pub(super) fn key(&self) -> String {
        match self {
            Self::Generated(_, index) => index.to_string(),
            Self::Overridden(_, index) => index.to_string(),
            Self::Named(name, index) => match index {
                None => name.to_string(),
                Some(index) => index.to_string(),
            },
        }
    }
}
