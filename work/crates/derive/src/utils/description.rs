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

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute,
    Error,
    Expr,
    ExprLit,
    Lit,
    LitStr,
    Result,
};

use crate::utils::{error, system_panic};

pub enum Description {
    Unset,
    Short {
        span: Span,
        short: DescriptionExpr,
    },
    Full {
        span: Span,
        short: DescriptionExpr,
        verbose: DescriptionExpr,
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
                    "Expected description in form of `<short>` or \
                    `<short>, <verbose>`,\nwhere `<short>` and `<verbose>` must \
                    be string literals or constant expressions.\n\n\
                    Examples: `#[describe(\"foo\", \"bar\")]`, or \
                    `#[describe(\"foo\", include_str!(\"bar.txt\"))]`.",
                ));
            }

            let short = input.parse::<DescriptionExpr>()?;

            let verbose = match input.peek(Token![,]) {
                false => None,

                true => {
                    let _ = input.parse::<Token![,]>()?;
                    Some(input.parse::<DescriptionExpr>()?)
                }
            };

            if !input.is_empty() {
                return Err(error!(input.span(), "unexpected end of input",));
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
                    short: DescriptionExpr::Literal(literal.clone()),
                    verbose: DescriptionExpr::Literal(literal),
                }
            }

            Self::Short {
                span: attr_span,
                short,
            } => {
                let (span, string) = initializer();

                let verbose = DescriptionExpr::Literal(LitStr::new(string.as_ref(), span));

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
    pub fn short(&self) -> &DescriptionExpr {
        match self {
            Self::Full { short, .. } => short,
            _ => system_panic!("Description is not fully initialized."),
        }
    }

    #[inline(always)]
    pub fn verbose(&self) -> &DescriptionExpr {
        match self {
            Self::Full { verbose, .. } => verbose,
            _ => system_panic!("Description is not fully initialized."),
        }
    }
}

#[derive(Clone)]
pub enum DescriptionExpr {
    Literal(LitStr),
    Custom(Expr),
}

impl Eq for DescriptionExpr {}

impl PartialEq for DescriptionExpr {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Literal(a), Self::Literal(b)) => a == b,
            _ => false,
        }
    }
}

impl Parse for DescriptionExpr {
    #[inline(always)]
    fn parse(input: ParseStream) -> Result<Self> {
        let expr = input.parse::<Expr>()?;

        match expr {
            Expr::Lit(ExprLit {
                attrs,
                lit: Lit::Str(string),
            }) if attrs.is_empty() => Ok(Self::Literal(string)),

            other => Ok(Self::Custom(other)),
        }
    }
}

impl ToTokens for DescriptionExpr {
    #[inline(always)]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Literal(lit) => lit.to_tokens(tokens),
            Self::Custom(expr) => {
                let span = expr.span();

                quote_spanned!(span=> const {#expr}).to_tokens(tokens)
            }
        }
    }
}
