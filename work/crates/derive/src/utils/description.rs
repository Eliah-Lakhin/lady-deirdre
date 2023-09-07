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

use crate::utils::{error, system_panic};

pub enum Description {
    Unset,
    Short {
        span: Span,
        short: LitStr,
    },
    Full {
        span: Span,
        short: LitStr,
        verbose: LitStr,
    },
}

impl TryFrom<Attribute> for Description {
    type Error = Error;

    fn try_from(attr: Attribute) -> Result<Self> {
        let span = attr.span();

        attr.parse_args_with(|input: ParseStream| {
            if input.is_empty() {
                return Err(error!(
                    input.span(),
                    "Expected description strings in form of `\"<short>\"` or \
                    `\"<short>\", \"<verbose>\"`.",
                ));
            }

            let short = input.parse::<LitStr>()?;

            let verbose = match input.peek(Token![,]) {
                false => None,

                true => {
                    let _ = input.parse::<Token![,]>()?;
                    Some(input.parse::<LitStr>()?)
                }
            };

            if !input.is_empty() {
                return Err(error!(input.span(), "unexpected end of input.",));
            }

            match verbose {
                None => Ok(Self::Short { span, short }),

                Some(verbose) => Ok(Self::Full {
                    span,
                    short,
                    verbose,
                }),
            }
        })
    }
}

impl Description {
    pub fn complete(self, initializer: impl FnOnce() -> (Span, String)) -> Self {
        match self {
            Self::Unset => {
                let (span, string) = initializer();

                let literal = LitStr::new(string.as_ref(), span);

                Self::Full {
                    span,
                    short: literal.clone(),
                    verbose: literal,
                }
            }

            Self::Short {
                span: attr_span,
                short,
            } => {
                let (span, string) = initializer();

                let verbose = LitStr::new(string.as_ref(), span);

                Self::Full {
                    span: attr_span,
                    short,
                    verbose,
                }
            }

            result @ Self::Full { .. } => result,
        }
    }

    #[inline(always)]
    pub fn is_set(&self) -> bool {
        match self {
            Self::Unset => false,
            _ => true,
        }
    }

    #[inline(always)]
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Unset => None,
            Self::Short { span, .. } => Some(*span),
            Self::Full { span, .. } => Some(*span),
        }
    }

    #[inline(always)]
    pub fn short(&self) -> &LitStr {
        match self {
            Self::Full { short, .. } => short,
            _ => system_panic!("Description is not fully initialized."),
        }
    }

    #[inline(always)]
    pub fn verbose(&self) -> &LitStr {
        match self {
            Self::Full { verbose, .. } => verbose,
            _ => system_panic!("Description is not fully initialized."),
        }
    }
}
