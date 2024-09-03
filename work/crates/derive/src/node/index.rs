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

            if index == u16::MAX {
                return Err(error!(span, "{} index is a marker of non-rule.", u16::MAX));
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

                    if index == u16::MAX {
                        return Err(error!(span, "{} index is a marker of non-rule.", u16::MAX));
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
