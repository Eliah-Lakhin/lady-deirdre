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
    mem::take,
    time::{Duration, Instant},
};

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span};
use syn::{spanned::Spanned, AttrStyle, Error, Expr, ExprLit, Lit, LitInt, Result, Variant};

use crate::{
    token::{
        automata::TokenAutomata,
        regex::{Regex, RegexImpl},
    },
    utils::{error, Description},
};

pub(super) type TokenRule = u8;

pub(super) const EOI: TokenRule = 0;
pub(super) const MISMATCH: TokenRule = 1;

pub(super) struct TokenVariant {
    pub(super) ident: Ident,
    pub(super) index: Option<u8>,
    pub(super) rule: Option<(Span, Regex)>,
    pub(super) automata: Option<TokenAutomata>,
    pub(super) constructor: Option<Expr>,
    pub(super) priority: isize,
    pub(super) description: Description,
    pub(super) time: Duration,
}

impl TryFrom<Variant> for TokenVariant {
    type Error = Error;

    fn try_from(mut variant: Variant) -> Result<Self> {
        let ident = variant.ident.clone();

        let index = match take(&mut variant.discriminant) {
            None => None,

            Some((_, expr)) => match expr {
                Expr::Lit(ExprLit { lit, .. }) => match lit {
                    Lit::Byte(lit) => Some(lit.value()),
                    Lit::Int(lit) => Some(lit.base10_parse::<u8>()?),

                    other => {
                        return Err(error!(
                            other.span(),
                            "Expected integer literal that represents byte value.",
                        ));
                    }
                },

                other => {
                    return Err(error!(
                        other.span(),
                        "Expected integer literal that represents byte value.",
                    ));
                }
            },
        };

        let mut rule = None;
        let mut constructor = None;
        let mut description = Description::Unset;
        let mut priority = None;
        let mut time = Duration::default();

        for attr in take(&mut variant.attrs) {
            match attr.style {
                AttrStyle::Inner(_) => continue,
                AttrStyle::Outer => (),
            }

            let name = match attr.meta.path().get_ident() {
                Some(ident) => ident,
                None => continue,
            };

            let span = attr.span();

            match name.to_string().as_str() {
                "rule" => {
                    if rule.is_some() {
                        return Err(error!(span, "Duplicate Rule attribute.",));
                    }

                    let start = Instant::now();
                    rule = Some((span, attr.parse_args::<Regex>()?));
                    time += start.elapsed();
                }

                "constructor" => {
                    if constructor.is_some() {
                        return Err(error!(span, "Duplicate Constructor attribute.",));
                    }

                    constructor = Some(attr.parse_args::<Expr>()?);
                }

                "describe" => {
                    if description.is_set() {
                        return Err(error!(span, "Duplicate Describe attribute.",));
                    }

                    description = Description::try_from(attr)?;
                }

                "priority" => {
                    if priority.is_some() {
                        return Err(error!(span, "Duplicate Priority attribute.",));
                    }

                    priority = Some((span, attr.parse_args::<LitInt>()?.base10_parse::<isize>()?));
                }

                "dump" => {
                    return Err(error!(span, "Dump attribute is not applicable here.",));
                }

                _ => continue,
            }
        }

        match index {
            Some(EOI) => {
                if let Some((span, _)) = &rule {
                    return Err(error!(
                        *span,
                        "Variant with index {EOI} may not have explicit \
                        rule.\nThis variant is reserved to indicate the end \
                        of input.",
                    ));
                }
            }

            Some(MISMATCH) => {
                if let Some((span, _)) = &rule {
                    return Err(error!(
                        *span,
                        "Variant with index {MISMATCH} may not have explicit \
                        rule.\nThis variant serves as a fallback token where \
                        the lexer sinks mismatched text sequences.",
                    ));
                }
            }

            _ => (),
        }

        if let Some(expr) = &constructor {
            if rule.is_none() {
                return Err(error!(
                    expr.span(),
                    "Constructor attribute is not applicable to unparseable \
                    variants.\nTo make the variant parsable annotate this \
                    variant with #[rule(...)] attribute.",
                ));
            }
        }

        let description = description.complete(|| {
            (
                ident.span(),
                match rule.as_ref().map(|(_, regex)| regex.name()).flatten() {
                    None => format!(
                        "<{}>",
                        ident.to_string().to_case(Case::Title).to_lowercase()
                    ),
                    Some(rule) => rule,
                },
            )
        });

        let priority = match priority {
            None => 0,

            Some((span, priority)) => {
                if rule.is_none() {
                    return Err(error!(
                        span,
                        "Priority attribute is not applicable to unparseable \
                        variants.\nTo make the variant parsable annotate this \
                        variant with #[rule(...)] attribute.",
                    ));
                }

                priority
            }
        };

        Ok(Self {
            ident,
            index,
            rule,
            automata: None,
            constructor,
            priority,
            description,
            time,
        })
    }
}
