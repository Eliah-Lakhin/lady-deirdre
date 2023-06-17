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
use syn::{parse::ParseStream, spanned::Spanned, Attribute, Error, Meta, Result};

use crate::utils::dump_kw;

//todo consider improving automata displaying (maybe in pictures?)
#[derive(Clone, Copy, Default)]
pub enum Dump {
    #[default]
    None,
    Output(Span),
    Trivia(Span),
    Meta(Span),
    Dry(Span),
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

            return Err(lookahead.error());
        })
    }
}

impl Dump {
    #[inline(always)]
    pub fn span(self) -> Option<Span> {
        match self {
            Dump::None => None,
            Dump::Output(span) => Some(span),
            Dump::Trivia(span) => Some(span),
            Dump::Meta(span) => Some(span),
            Dump::Dry(span) => Some(span),
        }
    }
}
