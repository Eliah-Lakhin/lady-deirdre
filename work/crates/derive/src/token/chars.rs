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

use std::fmt::{Display, Formatter};

use proc_macro2::Span;
use syn::{
    parse::{Lookahead1, Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    LitChar,
    Result,
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
                    classes: Set::new([Class::Upper, Class::Lower]),
                });
            }

            if lookahead.peek(char_kw::alphanum) {
                return Ok(Self {
                    span: input.parse::<char_kw::alphanum>()?.span,
                    classes: Set::new([Class::Upper, Class::Lower, Class::Num]),
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
        // if lookahead.peek(char_kw::alpha) {
        // }
        //
        // if lookahead.peek(char_kw::alphanum) {
        //     return true;
        // }
        //
        // if lookahead.peek(char_kw::upper) {
        //     return true;
        // }
        //
        // if lookahead.peek(char_kw::lower) {
        //     return true;
        // }
        //
        // if lookahead.peek(char_kw::num) {
        //     return true;
        // }
        //
        // if lookahead.peek(char_kw::space) {
        //     return true;
        // }

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
