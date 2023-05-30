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

use convert_case::{Case, Casing};
use proc_macro2::Ident;
use syn::{spanned::Spanned, AttrStyle, Error, ExprLit, Lit, LitStr, Result, Variant};

use crate::{
    token::{
        regex::{InlineMap, Regex, RegexImpl},
        rule::{RuleIndex, RulePrecedence},
    },
    utils::system_panic,
};

pub(super) enum TokenVariant {
    Rule {
        name: Ident,
        index: RuleIndex,
        precedence: Option<RulePrecedence>,
        constructor: Option<Ident>,
        expression: Regex,
        description: LitStr,
    },
    Mismatch {
        name: Ident,
        description: LitStr,
    },
    Other,
}

impl TokenVariant {
    pub(super) fn from_variant(
        variant: Variant,
        index: RuleIndex,
        inline_map: &InlineMap,
    ) -> Result<Self> {
        let name = variant.ident;

        if !variant.fields.is_empty() {
            return Err(Error::new(
                variant.fields.span(),
                "Variants with fields not allowed.",
            ));
        }

        let mut precedence = None;
        let mut constructor = None;
        let mut mismatch = false;
        let mut expression = None;
        let mut description = None;

        for attribute in variant.attrs {
            match attribute.style {
                AttrStyle::Inner(_) => continue,
                AttrStyle::Outer => (),
            }

            let name = match attribute.path.get_ident() {
                None => continue,
                Some(name) => name,
            };

            match name.to_string().as_str() {
                "precedence" => {
                    if precedence.is_some() {
                        return Err(Error::new(name.span(), "Duplicate Precedence attribute."));
                    }

                    if mismatch {
                        return Err(Error::new(
                            name.span(),
                            "Mismatch rules cannot have precedence.",
                        ));
                    }

                    let expression = attribute.parse_args::<ExprLit>()?;

                    match expression.lit {
                        Lit::Int(literal) => {
                            let value = literal.base10_parse::<usize>()?;

                            if value == 0 {
                                return Err(Error::new(
                                    literal.span(),
                                    "Rule precedence value must be positive. Default \
                                    precedence is \"1\".",
                                ));
                            }

                            precedence = Some(value);
                        }

                        other => {
                            return Err(Error::new(
                                other.span(),
                                "Expected usize numeric literal.",
                            ));
                        }
                    }
                }

                "constructor" => {
                    if constructor.is_some() {
                        return Err(Error::new(
                            attribute.span(),
                            "Duplicate Constructor attribute.",
                        ));
                    }

                    constructor = Some(attribute.parse_args::<Ident>()?);
                }

                "mismatch" => {
                    if mismatch {
                        return Err(Error::new(name.span(), "Duplicate Mismatch attribute."));
                    }

                    if expression.is_some() {
                        return Err(Error::new(
                            name.span(),
                            "Explicit rules cannot serve as a mismatch fallback.",
                        ));
                    }

                    if precedence.is_some() {
                        return Err(Error::new(
                            name.span(),
                            "Variants with precedence cannot be labeled as a mismatch fallback.",
                        ));
                    }

                    if !attribute.tokens.is_empty() {
                        return Err(Error::new(name.span(), "Unexpected attribute parameters."));
                    }

                    mismatch = true;
                }

                "rule" => {
                    if expression.is_some() {
                        return Err(Error::new(name.span(), "Duplicate Rule attribute."));
                    }

                    if mismatch {
                        return Err(Error::new(
                            name.span(),
                            "Mismatch token variant cannot have an explicit rule.",
                        ));
                    }

                    let mut regex = attribute.parse_args::<Regex>()?;

                    regex.inline(inline_map)?;

                    expression = Some(regex);
                }

                "describe" => {
                    if description.is_some() {
                        return Err(Error::new(name.span(), "Duplicate Describe attribute."));
                    }

                    description = Some(attribute.parse_args::<LitStr>()?);
                }

                _ => continue,
            }
        }

        match expression {
            None => {
                if let Some(name) = constructor {
                    return Err(Error::new(
                        name.span(),
                        "Constructor attributes cannot be defined on the non-parsable \
                        variants.\nTo make the variant parsable label it with \
                        #[rule(<expression>)] attribute.",
                    ));
                }

                Ok(match mismatch {
                    true => {
                        let description = description.unwrap_or_else(|| {
                            LitStr::new(
                                &format!(
                                    "<{}>",
                                    name.to_string().to_case(Case::Title).to_lowercase()
                                ),
                                name.span(),
                            )
                        });

                        Self::Mismatch { name, description }
                    }

                    false => {
                        if let Some(description) = description {
                            return Err(Error::new(
                                description.span(),
                                "Constructor attributes cannot be defined on the non-parsable \
                                variants.\nTo make the variant parsable label it with \
                                #[rule(<expression>)] attribute.",
                            ));
                        }

                        Self::Other
                    }
                })
            }

            Some(expression) => {
                let description = description
                    .or_else(|| {
                        expression
                            .name()
                            .map(|string| LitStr::new(&string, name.span()))
                    })
                    .unwrap_or_else(|| {
                        LitStr::new(
                            &format!("<{}>", name.to_string().to_case(Case::Title).to_lowercase()),
                            name.span(),
                        )
                    });

                Ok(Self::Rule {
                    name,
                    index,
                    precedence,
                    constructor,
                    expression,
                    description,
                })
            }
        }
    }

    #[inline(always)]
    pub(super) fn rule_name(&self) -> &Ident {
        match self {
            TokenVariant::Rule { name, .. } => name,
            _ => system_panic!("Non-rule variant."),
        }
    }

    #[inline(always)]
    pub(super) fn rule_precedence(&self) -> RulePrecedence {
        match self {
            TokenVariant::Rule { precedence, .. } => precedence.clone().unwrap_or(1),
            _ => system_panic!("Non-rule variant."),
        }
    }
}
