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
    token::{
        regex::{Operand, Operator, Regex},
        ucd::{Char, CharProperties},
    },
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

            let mut props = CharProperties::new();

            if lookahead.peek(syn::token::Brace) {
                let content;
                braced!(content in input);

                let span = content.span();

                let components =
                    Punctuated::<CharProp, Token![|]>::parse_separated_nonempty(&content)?;

                for prop in components {
                    prop.append_to(&mut props);
                }

                return Ok(Self {
                    span,
                    classes: Set::new([Class::Props(props)]),
                });
            }

            let prop = CharProp::lookahead(input, lookahead)?;
            let span = prop.span();

            prop.append_to(&mut props);

            return Ok(Self {
                span,
                classes: Set::new([Class::Props(props)]),
            });
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

    pub(super) fn parse_brackets(input: ParseStream, exclusion: bool) -> Result<Self> {
        let content;
        bracketed!(content in input);

        let span = content.span();

        let components = Punctuated::<Self, Token![,]>::parse_separated_nonempty(&content)?;

        if exclusion {
            for component in &components {
                for class in &component.classes {
                    let Class::Props(_) = &class else {
                        continue;
                    };

                    if !exclusion {
                        return Err(error!(
                            component.span,
                            "Property classes in the exclusion syntax are forbidden.",
                        ));
                    }
                }
            }
        }

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
    Props(CharProperties),
    Other,
}

impl Display for Class {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Char(ch) => formatter.write_fmt(format_args!("{:?}", ch)),
            Self::Props(props) => Display::fmt(props, formatter),
            Self::Other => formatter.write_str("_"),
        }
    }
}

impl Class {
    #[inline(always)]
    pub(super) fn includes(&self, ch: &char) -> bool {
        match self {
            Self::Char(this) => this == ch,
            Self::Props(props) => ch.has_properties(props),
            Self::Other => true,
        }
    }
}

enum CharProp {
    Alpha(Span),
    Lower(Span),
    Num(Span),
    Space(Span),
    Upper(Span),
    XidContinue(Span),
    XidStart(Span),
}

impl Parse for CharProp {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();

        Self::lookahead(input, lookahead)
    }
}

impl CharProp {
    #[inline(always)]
    fn lookahead(input: ParseStream, lookahead: Lookahead1) -> Result<Self> {
        if lookahead.peek(char_kw::alpha) {
            let span = input.parse::<char_kw::alpha>()?.span;
            return Ok(Self::Alpha(span));
        }

        if lookahead.peek(char_kw::lower) {
            let span = input.parse::<char_kw::lower>()?.span;
            return Ok(Self::Lower(span));
        }

        if lookahead.peek(char_kw::num) {
            let span = input.parse::<char_kw::num>()?.span;
            return Ok(Self::Num(span));
        }

        if lookahead.peek(char_kw::space) {
            let span = input.parse::<char_kw::space>()?.span;
            return Ok(Self::Space(span));
        }

        if lookahead.peek(char_kw::upper) {
            let span = input.parse::<char_kw::upper>()?.span;
            return Ok(Self::Upper(span));
        }

        if lookahead.peek(char_kw::xid_continue) {
            let span = input.parse::<char_kw::xid_continue>()?.span;
            return Ok(Self::XidContinue(span));
        }

        if lookahead.peek(char_kw::xid_start) {
            let span = input.parse::<char_kw::xid_start>()?.span;
            return Ok(Self::XidStart(span));
        }

        Err(lookahead.error())
    }

    #[inline(always)]
    fn append_to(self, props: &mut CharProperties) {
        match self {
            Self::Alpha(_) => props.alpha = true,
            Self::Lower(_) => props.lower = true,
            Self::Num(_) => props.num = true,
            Self::Space(_) => props.space = true,
            Self::Upper(_) => props.upper = true,
            Self::XidContinue(_) => props.xid_continue = true,
            Self::XidStart(_) => props.xid_start = true,
        }
    }

    #[inline(always)]
    fn span(&self) -> Span {
        match self {
            Self::Alpha(span) => *span,
            Self::Lower(span) => *span,
            Self::Num(span) => *span,
            Self::Space(span) => *span,
            Self::Upper(span) => *span,
            Self::XidContinue(span) => *span,
            Self::XidStart(span) => *span,
        }
    }
}

mod char_kw {
    syn::custom_keyword!(alpha);
    syn::custom_keyword!(lower);
    syn::custom_keyword!(num);
    syn::custom_keyword!(space);
    syn::custom_keyword!(upper);
    syn::custom_keyword!(xid_continue);
    syn::custom_keyword!(xid_start);
}
