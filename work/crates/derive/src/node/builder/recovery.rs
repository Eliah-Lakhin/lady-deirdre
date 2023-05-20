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

use proc_macro2::{Ident, Span};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute,
    Error,
    LitInt,
    Result,
};

use crate::{
    token::Token,
    utils::{Map, PredictableCollection, Set},
};

#[derive(Default)]
pub(in crate::node) struct Recovery {
    balance: Map<Ident, Ident>,
    delimiters: Set<Ident>,
}

impl<'a> TryFrom<&'a Attribute> for Recovery {
    type Error = Error;

    fn try_from(attribute: &'a Attribute) -> Result<Self> {
        enum Specification {
            Balance { open: Ident, close: Ident },
            Delimiters { set: Set<Ident> },
        }

        impl Parse for Specification {
            fn parse(input: ParseStream) -> Result<Self> {
                let lookahead = input.lookahead1();

                if lookahead.peek(keyword::balance) {
                    let _ = input.parse::<keyword::balance>()?;

                    let content;
                    bracketed!(content in input);

                    let open = content.parse::<TokenLit>()?;
                    let _ = content.parse::<Token![..]>()?;
                    let close = content.parse::<TokenLit>()?;

                    if open.0 == close.0 {
                        return Err(Error::new(
                            close.0.span(),
                            "Close balance token must distinct with the Open token.",
                        ));
                    }

                    if !content.is_empty() {
                        return Err(input.error("Unexpected token."));
                    }

                    return Ok(Self::Balance {
                        open: open.0,
                        close: close.0,
                    });
                }

                if lookahead.peek(keyword::delimiters) {
                    let _ = input.parse::<keyword::delimiters>()?;
                    let set = input.parser::<TokeLitSet>()?;

                    return Ok(Self::Delimiters { set: set.set });
                }

                Err(lookahead.error())
            }
        }

        attribute.parse_args_with(|input: ParseStream| {
            let specifications = Punctuated::<Specification, Token![;]>::parse_terminated(input)?;

            if specifications.is_empty() {
                return Err(input.error(
                    "Expected \"balance [$start..$end]\" or \"delimiters [$A | $B | ...]\" \
                    specifications separated with \";\".",
                ));
            }

            let mut balance = Map::empty();
            let mut balance_rev = Map::empty();
            let mut delimiters = Set::empty();

            for spec in specifications {
                match spec {
                    Specification::Balance { open, close } => {
                        if let Some(previous) = balance.get(&open) {
                            return Err(Error::new(
                                open.span(),
                                format!(
                                    "This token already used in the \
                                    previous {open}..{previous} balance pair. \
                                    All balance tokens must be unique."
                                ),
                            ));
                        }

                        if let Some(previous) = balance_rev.get(&close) {
                            return Err(Error::new(
                                open.span(),
                                format!(
                                    "This token already used in the \
                                    previous ${previous}..${close} balance pair. \
                                    All balance tokens must be unique."
                                ),
                            ));
                        }

                        if delimiters.contains(&open) {
                            return Err(Error::new(
                                open.span(),
                                "This token already used in the set \
                                of delimiters.\nBalance tokens don't need \
                                to be enumerated in delimiters, because \
                                they serve as delimiters by themself.",
                            ));
                        }

                        let _ = balance.insert(open.clone(), close.clone());
                        let _ = balance_rev.insert(close, open);
                    }

                    Specification::Delimiters { set } => {
                        for token in set {
                            if delimiters.contains(&token) {
                                return Err(Error::new(token.span(), "Duplicate delimiter token."));
                            }

                            if let Some(close) = balance.get(&token) {
                                return Err(Error::new(
                                    token.span(),
                                    format!(
                                        "This token already used in the \
                                        {token}..{close} balance pair.\n\
                                        Balance tokens don't need \
                                        to be enumerated in delimiters, because \
                                        they serve as delimiters by themself."
                                    ),
                                ));
                            }

                            if let Some(open) = balance_rev.get(&token) {
                                return Err(Error::new(
                                    token.span(),
                                    format!(
                                        "This token already used in the \
                                        {open}..{token} balance pair.\n\
                                        Balance tokens don't need \
                                        to be enumerated in delimiters, because \
                                        they serve as delimiters by themself."
                                    ),
                                ));
                            }

                            let _ = delimiters.insert(token);
                        }
                    }
                }
            }

            Ok(Self {
                balance,
                delimiters,
            })
        })
    }
}

mod keyword {
    syn::custom_keyword!(balance);
    syn::custom_keyword!(delimiters);
}
