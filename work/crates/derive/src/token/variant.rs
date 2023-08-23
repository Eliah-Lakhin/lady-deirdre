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
    mem::take,
    time::{Duration, Instant},
};

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span};
use syn::{
    spanned::Spanned,
    AttrStyle,
    Error,
    Expr,
    ExprLit,
    Lit,
    LitInt,
    LitStr,
    Result,
    Variant,
};

use crate::{
    token::{
        automata::TokenAutomata,
        regex::{Regex, RegexImpl},
    },
    utils::error,
};

pub(super) type TokenRule = u8;

pub(super) const EOI: TokenRule = 0;
pub(super) const MISMATCH: TokenRule = 1;

pub(super) struct TokenVariant {
    pub(super) ident: Ident,
    pub(super) index: Option<u8>,
    pub(super) rule: Option<(Span, Regex)>,
    pub(super) automata: Option<TokenAutomata>,
    //todo turn to Expr
    pub(super) constructor: Option<Ident>,
    pub(super) priority: isize,
    pub(super) description: LitStr,
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
        let mut description = None;
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

                    constructor = Some(attr.parse_args::<Ident>()?);
                }

                "describe" => {
                    if description.is_some() {
                        return Err(error!(span, "Duplicate Describe attribute.",));
                    }

                    description = Some(attr.parse_args::<LitStr>()?);
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

        if let Some(ident) = &constructor {
            if rule.is_none() {
                return Err(error!(
                    ident.span(),
                    "Constructor attribute is not applicable to unparseable \
                    variants.\nTo make the variant parsable annotate this \
                    variant with #[rule(...)] attribute.",
                ));
            }
        }

        let description = match description {
            Some(lit) => lit,

            None => {
                let name = match rule.as_ref().map(|(_, regex)| regex.name()).flatten() {
                    None => format!(
                        "<{}>",
                        ident.to_string().to_case(Case::Title).to_lowercase()
                    ),
                    Some(rule) => rule,
                };

                LitStr::new(&name, ident.span())
            }
        };

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
