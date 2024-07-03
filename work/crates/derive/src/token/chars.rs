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

use std::fmt::{Display, Formatter};

use proc_macro2::Span;
use syn::{
    LitChar,
    parse::{Lookahead1, Parse, ParseStream},
    punctuated::Punctuated,
    Result,
    spanned::Spanned,
};

use crate::{
    token::regex::{Operand, Operator, Regex},
    utils::{error, PredictableCollection, Set, SetImpl},
};

#[derive(Clone)]
pub(super) struct CharSet {
    pub(super) span: Span,
    pub(super) classes: Set<Class>,
}

impl Parse for CharSet {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Token![$]) {
            let _ = input.parse::<Token![$]>()?;
            let lookahead = input.lookahead1();

            if lookahead.peek(char_kw::alpha) {
                return Ok(Self {
                    span: input.parse::<char_kw::alpha>()?.span,
                    classes: Set::new([Class::Upper, Class::Lower, Class::Alpha]),
                });
            }

            if lookahead.peek(char_kw::alphanum) {
                return Ok(Self {
                    span: input.parse::<char_kw::alphanum>()?.span,
                    classes: Set::new([Class::Upper, Class::Lower, Class::Alpha, Class::Num]),
                });
            }

            if lookahead.peek(char_kw::upper) {
                return Ok(Self {
                    span: input.parse::<char_kw::upper>()?.span,
                    classes: Set::new([Class::Upper]),
                });
            }

            if lookahead.peek(char_kw::lower) {
                return Ok(Self {
                    span: input.parse::<char_kw::lower>()?.span,
                    classes: Set::new([Class::Lower]),
                });
            }

            if lookahead.peek(char_kw::num) {
                return Ok(Self {
                    span: input.parse::<char_kw::num>()?.span,
                    classes: Set::new([Class::Num]),
                });
            }

            if lookahead.peek(char_kw::space) {
                return Ok(Self {
                    span: input.parse::<char_kw::space>()?.span,
                    classes: Set::new([Class::Space]),
                });
            }

            return Err(lookahead.error());
        }

        if lookahead.peek(syn::LitChar) {
            let start_lit = input.parse::<LitChar>()?;
            let start_char = start_lit.value();

            if input.peek(Token![..]) {
                let span = input.parse::<Token![..]>()?.span();

                let end_lit = input.parse::<LitChar>()?;
                let end_char = end_lit.value();

                if start_char >= end_char {
                    return Err(error!(
                        span,
                        "Range start must be lesser than the range end.",
                    ));
                }

                return Ok(Self {
                    span,
                    classes: (start_char..=end_char)
                        .into_iter()
                        .map(Class::Char)
                        .collect(),
                });
            }

            return Ok(Self {
                span: start_char.span(),
                classes: Set::new([Class::Char(start_char)]),
            });
        }

        Err(lookahead.error())
    }
}

impl CharSet {
    #[inline(always)]
    pub(super) fn empty(span: Span) -> Self {
        Self {
            span,
            classes: Set::empty(),
        }
    }

    pub(super) fn peek(lookahead: &Lookahead1) -> bool {
        if lookahead.peek(Token![$]) {
            return true;
        }

        if lookahead.peek(syn::LitChar) {
            return true;
        }

        false
    }

    pub(super) fn parse_brackets(input: ParseStream) -> Result<Self> {
        let content;
        bracketed!(content in input);

        let span = content.span();

        let components = Punctuated::<Self, Token![,]>::parse_separated_nonempty(&content)?;

        Ok(components.into_iter().fold(
            Self {
                span,
                classes: Set::empty(),
            },
            |mut accumulator, component| {
                accumulator.classes.append(component.classes);

                accumulator
            },
        ))
    }

    pub(super) fn into_expr(self) -> Option<Regex> {
        let span = self.span;

        self.classes.into_iter().fold(None, |accumulator, class| {
            let right = Regex::Operand(Operand::Class(span, class));

            let left = match accumulator {
                Some(accumulator) => accumulator,
                None => return Some(right),
            };

            Some(Regex::Binary(
                Box::new(left),
                Operator::Union,
                Box::new(right),
            ))
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(super) enum Class {
    Char(char),
    Upper,
    Lower,
    Alpha,
    Num,
    Space,
    Other,
}

impl Display for Class {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Char(ch) => formatter.write_fmt(format_args!("{:?}", ch)),
            Self::Upper => formatter.write_str("$Upper"),
            Self::Lower => formatter.write_str("$Lower"),
            Self::Alpha => formatter.write_str("$Alpha"),
            Self::Num => formatter.write_str("$Num"),
            Self::Space => formatter.write_str("$Space"),
            Self::Other => formatter.write_str("_"),
        }
    }
}

impl Class {
    #[inline(always)]
    pub(super) fn includes(&self, ch: &char) -> bool {
        match self {
            Self::Char(this) => this == ch,
            Self::Upper => ch.is_uppercase(),
            Self::Lower => ch.is_lowercase(),
            Self::Alpha => ch.is_alphabetic() && !ch.is_uppercase() && !ch.is_lowercase(),
            Self::Num => ch.is_numeric(),
            Self::Space => ch.is_whitespace(),
            Self::Other => true,
        }
    }
}

mod char_kw {
    syn::custom_keyword!(alpha);
    syn::custom_keyword!(alphanum);
    syn::custom_keyword!(upper);
    syn::custom_keyword!(lower);
    syn::custom_keyword!(num);
    syn::custom_keyword!(space);
}
