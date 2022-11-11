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

use proc_macro2::{Ident, Span};
use syn::{parse::ParseStream, spanned::Spanned, token::Paren, Result};

use crate::{
    node::regex::{inline::Inline, operator::RegexOperator, Regex},
    utils::ExpressionOperand,
};

#[derive(Clone)]
pub(in crate::node) enum RegexOperand {
    Unresolved { name: Ident, capture: Option<Ident> },
    Debug { span: Span, inner: Box<Regex> },
    Token { name: Ident, capture: Option<Ident> },
    Rule { name: Ident, capture: Option<Ident> },
}

impl Default for RegexOperand {
    #[inline(always)]
    fn default() -> Self {
        Self::Unresolved {
            name: Ident::new("_", Span::call_site()),
            capture: None,
        }
    }
}

impl Spanned for RegexOperand {
    #[inline(always)]
    fn span(&self) -> Span {
        match self {
            Self::Unresolved { name, .. } => name.span(),
            Self::Debug { span, .. } => *span,
            Self::Token { name, .. } => name.span(),
            Self::Rule { name, .. } => name.span(),
        }
    }
}

impl ExpressionOperand<RegexOperator> for RegexOperand {
    fn parse(input: ParseStream) -> Result<Regex> {
        let lookahead = input.lookahead1();

        if lookahead.peek(syn::Ident) {
            let identifier_a = input.parse::<Ident>()?;
            let identifier_a_string = identifier_a.to_string();

            if identifier_a_string == "debug" && input.peek(Paren) {
                let content;

                parenthesized!(content in input);

                let inner = content.parse::<Regex>()?;

                if !content.is_empty() {
                    return Err(content.error("Unexpected expression end."));
                }

                return Ok(Regex::Operand(RegexOperand::Debug {
                    span: identifier_a.span(),
                    inner: Box::new(inner),
                }));
            }

            if input.peek(Token![:]) {
                let _ = input.parse::<Token![:]>()?;

                let lookahead = input.lookahead1();

                if input.peek(Token![$]) {
                    let _ = input.parse::<Token![$]>()?;

                    let identifier_b = input.parse::<Ident>()?;

                    return Ok(Regex::Operand(RegexOperand::Token {
                        name: identifier_b,
                        capture: Some(identifier_a),
                    }));
                }

                if lookahead.peek(syn::Ident) {
                    let identifier_b = input.parse::<Ident>()?;

                    return Ok(Regex::Operand(RegexOperand::Unresolved {
                        name: identifier_b,
                        capture: Some(identifier_a),
                    }));
                }

                if lookahead.peek(syn::token::Paren) {
                    let content;

                    parenthesized!(content in input);

                    let mut result = content.parse::<Regex>()?;

                    if !content.is_empty() {
                        return Err(content.error("Unexpected expression end."));
                    }

                    result.capture(&identifier_a)?;

                    return Ok(result);
                }

                return Err(lookahead.error());
            }

            return Ok(Regex::Operand(RegexOperand::Unresolved {
                name: identifier_a,
                capture: None,
            }));
        }

        if input.peek(Token![$]) {
            let _ = input.parse::<Token![$]>()?;

            let identifier = input.parse::<Ident>()?;

            return Ok(Regex::Operand(RegexOperand::Token {
                name: identifier,
                capture: None,
            }));
        }

        if lookahead.peek(syn::token::Paren) {
            let content;

            parenthesized!(content in input);

            let result = content.parse::<Regex>()?;

            if !content.is_empty() {
                return Err(content.error("Unexpected expression end."));
            }

            return Ok(result);
        }

        Err(lookahead.error())
    }
}
