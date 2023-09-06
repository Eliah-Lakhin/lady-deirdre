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

use proc_macro2::Span;
use syn::{parse::ParseStream, spanned::Spanned, Attribute, Error, LitStr, Result};

use crate::utils::error;

pub struct Description {
    pub span: Span,
    pub short: LitStr,
    pub verbose: LitStr,
}

impl TryFrom<Attribute> for Description {
    type Error = Error;

    fn try_from(attr: Attribute) -> Result<Self> {
        let span = attr.span();

        attr.parse_args_with(|input: ParseStream| {
            if input.is_empty() {
                return Err(error!(
                    input.span(),
                    "Expected description strings in form of `\"<short>\", \
                    \"<verbose>\"` or `\"<short_and_verbose>\"`.",
                ));
            }

            let short = input.parse::<LitStr>()?;

            let verbose = match input.peek(Token![,]) {
                false => short.clone(),

                true => {
                    let _ = input.parse::<Token![,]>()?;
                    input.parse::<LitStr>()?
                }
            };

            if !input.is_empty() {
                return Err(error!(input.span(), "unexpected end of input.",));
            }

            return Ok(Self {
                span,
                short,
                verbose,
            });
        })
    }
}

impl Description {
    pub fn new(span: Span, string: impl AsRef<str>) -> Self {
        let literal = LitStr::new(string.as_ref(), span);

        Self {
            span,
            short: literal.clone(),
            verbose: literal,
        }
    }
}
