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
    borrow::Borrow,
    cmp::Ordering,
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
    mem::{discriminant, take},
};

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Paren,
    Error,
    Result,
};

use crate::{
    node::regex::{inline::Inline, operator::RegexOperator, Regex},
    utils::{debug_panic, ExpressionOperand, PredictableCollection, Set},
};

#[derive(Clone)]
pub(in crate::node) enum RegexOperand {
    Unresolved {
        name: Ident,
        capture: Option<Ident>,
    },
    Debug {
        span: Span,
        inner: Box<Regex>,
    },
    Token {
        name: TokenLit,
        capture: Option<Ident>,
    },
    Rule {
        name: Ident,
        capture: Option<Ident>,
    },
    Exclusion {
        set: TokenLitSet,
        capture: Option<Ident>,
    },
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
            Self::Token {
                name: TokenLit::Ident(ident),
                ..
            } => ident.span(),
            Self::Token {
                name: TokenLit::Other(span),
                ..
            } => *span,
            Self::Rule { name, .. } => name.span(),
            Self::Exclusion { set, .. } => set.span,
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
                    let identifier_b = input.parse::<TokenLit>()?;

                    return Ok(Regex::Operand(RegexOperand::Token {
                        name: identifier_b,
                        capture: Some(identifier_a),
                    }));
                }

                if input.peek(Token![^]) {
                    let _ = input.parse::<Token![^]>()?;
                    let set = input.parse::<TokenLitSet>()?;

                    return Ok(Regex::Operand(RegexOperand::Exclusion {
                        set,
                        capture: Some(identifier_a),
                    }));
                }

                if input.peek(Token![.]) {
                    let span = input.parse::<Token![.]>()?.span;

                    return Ok(Regex::Operand(RegexOperand::Exclusion {
                        set: TokenLitSet {
                            span,
                            set: Set::empty(),
                        },
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
            let identifier = input.parse::<TokenLit>()?;

            return Ok(Regex::Operand(RegexOperand::Token {
                name: identifier,
                capture: None,
            }));
        }

        if input.peek(Token![^]) {
            let _ = input.parse::<Token![^]>()?;
            let set = input.parse::<TokenLitSet>()?;

            return Ok(Regex::Operand(RegexOperand::Exclusion {
                set,
                capture: None,
            }));
        }

        if input.peek(Token![.]) {
            let span = input.parse::<Token![.]>()?.span;

            return Ok(Regex::Operand(RegexOperand::Exclusion {
                set: TokenLitSet {
                    span,
                    set: Set::empty(),
                },
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

#[derive(Clone)]
pub(in crate::node) enum TokenLit {
    Ident(Ident),
    Other(Span),
}

impl Display for TokenLit {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ident(ident) => formatter.write_fmt(format_args!("${ident}")),
            Self::Other(..) => formatter.write_str("$_"),
        }
    }
}

impl Hash for TokenLit {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        discriminant(self).hash(state);

        match self {
            Self::Ident(ident) => ident.hash(state),
            Self::Other(..) => (),
        }
    }
}

impl PartialEq for TokenLit {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ident(this), Self::Ident(other)) => this.eq(other),
            (Self::Other(..), Self::Other(..)) => true,
            _ => false,
        }
    }
}

impl Eq for TokenLit {}

impl PartialOrd for TokenLit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TokenLit {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Ident(this), Self::Ident(other)) => this.cmp(other),
            (Self::Other(..), Self::Other(..)) => Ordering::Equal,
            (Self::Ident(..), ..) => Ordering::Less,
            (Self::Other(..), ..) => Ordering::Greater,
        }
    }
}

impl Parse for TokenLit {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<Token![$]>()?;
        let ident = input.parse::<Ident>()?;

        Ok(Self::Ident(ident))
    }
}

impl ToTokens for TokenLit {
    #[inline]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(ident) => ident.to_tokens(tokens),
            Self::Other(..) => {
                todo!("Exclusion Operator is incomplete feature.")
            }
        }
    }
}

impl TokenLit {
    #[inline]
    pub(in crate::node) fn set_span(&mut self, span: Span) {
        match self {
            Self::Ident(ident) => ident.set_span(span),
            Self::Other(other) => *other = span,
        }
    }

    #[inline]
    pub(in crate::node) fn string(&self) -> String {
        match self {
            Self::Ident(ident) => ident.to_string(),
            Self::Other(..) => String::from("_"),
        }
    }
}

#[derive(Clone)]
pub(in crate::node) struct TokenLitSet {
    pub(in crate::node) span: Span,
    pub(in crate::node) set: Set<TokenLit>,
}

impl Parse for TokenLitSet {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        bracketed!(content in input);

        let span = content.span();
        let sequence = Punctuated::<TokenLit, Token![|]>::parse_separated_nonempty(&content)?;

        if !content.is_empty() {
            return Err(content.error(
                "Unexpected end of input. Expected a set \
                of tokens [$A | $B | ...].",
            ));
        }

        let mut set = Set::with_capacity(sequence.len());

        for token_lit in sequence {
            if set.contains(&token_lit) {
                return Err(Error::new(token_lit.span(), "Duplicate token."));
            }

            let _ = set.insert(token_lit);
        }

        Ok(Self { span, set })
    }
}

impl TokenLitSet {
    pub(in crate::node) fn set_span(&mut self, span: Span) {
        self.span = span;

        let set = take(&mut self.set)
            .into_iter()
            .map(|mut lit| {
                lit.set_span(span);

                lit
            })
            .collect();

        self.set = set;
    }
}
