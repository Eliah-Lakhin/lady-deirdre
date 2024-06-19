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
use syn::{parse::ParseStream, spanned::Spanned, Attribute, Error, Meta, Result};

use crate::utils::dump_kw;

#[derive(Clone, Copy, Default)]
pub enum Dump {
    #[default]
    None,
    Output(Span),
    Trivia(Span),
    Meta(Span),
    Dry(Span),
    Decl(Span),
}

impl TryFrom<Attribute> for Dump {
    type Error = Error;

    fn try_from(attr: Attribute) -> Result<Self> {
        let attr_span = attr.span();

        if let Meta::Path(..) = &attr.meta {
            return Ok(Self::Output(attr_span));
        }

        attr.parse_args_with(|input: ParseStream| {
            if input.is_empty() {
                return Ok(Self::Output(attr_span));
            }

            let lookahead = input.lookahead1();

            if lookahead.peek(dump_kw::output) {
                return Ok(Self::Output(input.parse::<dump_kw::output>()?.span()));
            }

            if lookahead.peek(dump_kw::trivia) {
                return Ok(Self::Trivia(input.parse::<dump_kw::trivia>()?.span()));
            }

            if lookahead.peek(dump_kw::meta) {
                return Ok(Self::Meta(input.parse::<dump_kw::meta>()?.span()));
            }

            if lookahead.peek(dump_kw::dry) {
                return Ok(Self::Dry(input.parse::<dump_kw::dry>()?.span()));
            }

            if lookahead.peek(dump_kw::decl) {
                return Ok(Self::Decl(input.parse::<dump_kw::decl>()?.span()));
            }

            return Err(lookahead.error());
        })
    }
}

impl Dump {
    #[inline(always)]
    pub fn span(self) -> Option<Span> {
        match self {
            Self::None => None,
            Self::Output(span) => Some(span),
            Self::Trivia(span) => Some(span),
            Self::Meta(span) => Some(span),
            Self::Dry(span) => Some(span),
            Self::Decl(span) => Some(span),
        }
    }

    #[inline(always)]
    pub fn is_declarative(&self) -> bool {
        match self {
            Self::Decl(..) => true,
            _ => false,
        }
    }
}
