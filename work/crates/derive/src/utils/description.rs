////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
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
