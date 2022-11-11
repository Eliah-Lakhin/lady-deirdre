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

use syn::parse::{Lookahead1, ParseStream, Result};

use crate::{
    node::regex::{operand::RegexOperand, Regex},
    utils::{Applicability, ExpressionOperator},
};

#[derive(Clone)]
pub(in crate::node) enum RegexOperator {
    Union,
    Concat,
    OneOrMore { separator: Option<Box<Regex>> },
    ZeroOrMore { separator: Option<Box<Regex>> },
    Optional,
}

impl ExpressionOperator for RegexOperator {
    type Operand = RegexOperand;

    #[inline]
    fn enumerate() -> Vec<Self> {
        vec![
            Self::Union,
            Self::Concat,
            Self::OneOrMore { separator: None },
            Self::ZeroOrMore { separator: None },
            Self::Optional,
        ]
    }

    #[inline(always)]
    fn binding_power(&self) -> u8 {
        match self {
            Self::Union => 10,
            Self::Concat => 20,
            Self::OneOrMore { .. } => 30,
            Self::ZeroOrMore { .. } => 40,
            Self::Optional => 50,
        }
    }

    #[inline]
    fn peek(&self, lookahead: &Lookahead1) -> Applicability {
        match self {
            Self::Union if lookahead.peek(Token![|]) => Applicability::Binary,
            Self::Concat if lookahead.peek(Token![&]) => Applicability::Binary,
            Self::OneOrMore { .. } if lookahead.peek(Token![+]) => Applicability::Unary,
            Self::ZeroOrMore { .. } if lookahead.peek(Token![*]) => Applicability::Unary,
            Self::Optional if lookahead.peek(Token![?]) => Applicability::Unary,

            _ => Applicability::Mismatch,
        }
    }

    #[inline]
    fn parse(&mut self, input: ParseStream) -> Result<()> {
        match self {
            Self::Union => drop(input.parse::<Token![|]>()?),

            Self::Concat => drop(input.parse::<Token![&]>()?),

            Self::OneOrMore { separator } => {
                let _ = input.parse::<Token![+]>()?;

                if input.peek(syn::token::Brace) {
                    let content;

                    braced!(content in input);

                    *separator = Some(Box::new(content.parse::<Regex>()?));
                }
            }

            Self::ZeroOrMore { separator } => {
                let _ = input.parse::<Token![*]>()?;

                if input.peek(syn::token::Brace) {
                    let content;

                    braced!(content in input);

                    *separator = Some(Box::new(content.parse::<Regex>()?));
                }
            }

            Self::Optional => drop(input.parse::<Token![?]>()?),
        };

        Ok(())
    }
}
