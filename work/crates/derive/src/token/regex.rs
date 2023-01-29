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
use syn::{
    parse::{Lookahead1, ParseStream},
    spanned::Spanned,
    token::Paren,
    Error,
    LitChar,
    LitStr,
    Result,
};

use crate::{
    token::{characters::CharacterSet, scope::Scope, NULL},
    utils::{
        Applicability,
        Automata,
        AutomataContext,
        Expression,
        ExpressionOperand,
        ExpressionOperator,
        Map,
    },
};

pub(super) type Regex = Expression<Operator>;

impl RegexImpl for Regex {
    fn inline(&mut self, inline_map: &InlineMap) -> Result<()> {
        match self {
            Self::Operand(Operand::Inline { name }) => {
                match inline_map.get(&name.to_string()) {
                    None => {
                        return Err(Error::new(
                            name.span(),
                            "Unknown inline expression.\nEach inline expression name is \
                            case-sensitive and should be defined before use.\nTo define an inline \
                            expression use #[define(name = <expression>)] attribute on the derived \
                            type.",
                        ));
                    }

                    Some(inline) => {
                        *self = inline.clone();
                    }
                };
            }

            Self::Operand(Operand::Debug { inner, .. }) => {
                inner.inline(inline_map)?;
            }

            Self::Unary { inner, .. } => inner.inline(inline_map)?,

            Self::Binary { left, right, .. } => {
                left.inline(inline_map)?;
                right.inline(inline_map)?;
            }

            _ => (),
        }

        Ok(())
    }

    fn alphabet(&self) -> CharacterSet {
        match self {
            Self::Operand(Operand::Inclusion { character_set }) => character_set.clone(),
            Self::Operand(Operand::Debug { inner, .. }) => inner.alphabet(),
            Self::Binary { left, right, .. } => left.alphabet().merge(right.alphabet()),
            Self::Unary { inner, .. } => inner.alphabet(),
            _ => CharacterSet::default(),
        }
    }

    fn encode(&self, scope: &mut Scope) -> Result<Automata<Scope>> {
        Ok(match self {
            Self::Operand(Operand::Any) => scope.any(),

            Self::Operand(Operand::Inline { .. }) => unreachable!("Unresolved inline."),

            Self::Operand(Operand::Debug { span, inner }) => {
                let inner = inner.encode(scope)?;

                return Err(Error::new(
                    *span,
                    format!(
                        "This expression is a subject for debugging.\n\nState machine transitions \
                        are:\n{:#}",
                        inner,
                    ),
                ));
            }

            Self::Operand(Operand::Inclusion { character_set }) => {
                character_set.clone().into_inclusion(scope)
            }

            Self::Operand(Operand::Exclusion { character_set }) => {
                character_set.clone().into_exclusion(scope)?
            }

            Self::Binary {
                operator: Operator::Concat,
                left,
                right,
            } => {
                let left = left.encode(scope)?;
                let right = right.encode(scope)?;

                scope.concatenate(left, right)
            }

            Self::Binary {
                operator: Operator::Union,
                left,
                right,
            } => {
                let left = left.encode(scope)?;
                let right = right.encode(scope)?;

                scope.union(left, right)
            }

            Self::Unary {
                operator: Operator::ZeroOrMore,
                inner,
            } => {
                let inner = inner.encode(scope)?;

                scope.repeat(inner)
            }

            Self::Unary {
                operator: Operator::OneOrMore,
                inner,
            } => {
                let inner = inner.encode(scope)?;

                let left = scope.copy(&inner);
                let right = scope.repeat(inner);

                scope.concatenate(left, right)
            }

            Self::Unary {
                operator: Operator::Optional,
                inner,
            } => {
                let inner = inner.encode(scope)?;

                scope.optional(inner)
            }

            _ => unreachable!("Unsupported operation."),
        })
    }
}

pub(super) trait RegexImpl {
    fn inline(&mut self, inline_map: &InlineMap) -> Result<()>;

    fn alphabet(&self) -> CharacterSet;

    fn encode(&self, scope: &mut Scope) -> Result<Automata<Scope>>;
}

pub(super) type InlineMap = Map<String, Regex>;

#[derive(Clone)]
pub(super) enum Operand {
    Any,
    Inline { name: Ident },
    Debug { span: Span, inner: Box<Regex> },
    Inclusion { character_set: CharacterSet },
    Exclusion { character_set: CharacterSet },
}

impl ExpressionOperand<Operator> for Operand {
    fn parse(input: ParseStream) -> Result<Regex> {
        let lookahead = input.lookahead1();

        if lookahead.peek(syn::LitChar) {
            let literal = input.parse::<LitChar>()?;

            if literal.value() == NULL {
                return Err(Error::new(literal.span(), "Null characters forbidden."));
            }

            return Ok(Expression::Operand(Operand::Inclusion {
                character_set: CharacterSet::from(literal),
            }));
        }

        if lookahead.peek(syn::LitStr) {
            let literal = input.parse::<LitStr>()?;
            let string = literal.value();

            return string
                .chars()
                .try_fold(None, |accumulator, character| {
                    if character == NULL {
                        return Err(Error::new(literal.span(), "Null characters forbidden."));
                    }

                    let right = Expression::Operand(Operand::Inclusion {
                        character_set: CharacterSet::from(LitChar::new(character, literal.span())),
                    });

                    Ok(Some(match accumulator {
                        None => right,
                        Some(left) => Expression::Binary {
                            operator: Operator::Concat,
                            left: Box::new(left),
                            right: Box::new(right),
                        },
                    }))
                })?
                .ok_or_else(|| Error::new(literal.span(), "Empty strings are forbidden."));
        }

        if lookahead.peek(Token![.]) {
            let _ = input.parse::<Token![.]>()?;

            return Ok(Expression::Operand(Operand::Any));
        }

        if lookahead.peek(syn::token::Bracket) {
            let content;

            bracketed!(content in input);

            let character_set = content.parse::<CharacterSet>()?;

            return Ok(Expression::Operand(Operand::Inclusion { character_set }));
        }

        if lookahead.peek(Token![^]) {
            let _ = input.parse::<Token![^]>()?;

            let content;

            bracketed!(content in input);

            let character_set = content.parse::<CharacterSet>()?;

            return Ok(Expression::Operand(Operand::Exclusion { character_set }));
        }

        if lookahead.peek(syn::Ident) {
            let identifier = input.parse::<Ident>()?;

            if identifier.to_string() == "debug" && input.peek(Paren) {
                let content;

                parenthesized!(content in input);

                return Ok(Expression::Operand(Operand::Debug {
                    span: identifier.span(),
                    inner: Box::new(content.parse::<Regex>()?),
                }));
            }

            return Ok(Expression::Operand(Operand::Inline { name: identifier }));
        }

        if lookahead.peek(syn::token::Paren) {
            let content;

            parenthesized!(content in input);

            return content.parse::<Regex>();
        }

        Err(lookahead.error())
    }
}

#[derive(Clone, Copy)]
pub(super) enum Operator {
    Union = 10,
    Concat = 20,
    OneOrMore = 30,
    ZeroOrMore = 40,
    Optional = 50,
}

impl ExpressionOperator for Operator {
    type Operand = Operand;

    #[inline]
    fn enumerate() -> Vec<Self> {
        vec![
            Self::Union,
            Self::Concat,
            Self::OneOrMore,
            Self::ZeroOrMore,
            Self::Optional,
        ]
    }

    #[inline(always)]
    fn binding_power(&self) -> u8 {
        *self as u8
    }

    #[inline]
    fn peek(&self, lookahead: &Lookahead1) -> Applicability {
        match self {
            Self::Union if lookahead.peek(Token![|]) => Applicability::Binary,
            Self::Concat if lookahead.peek(Token![&]) => Applicability::Binary,
            Self::OneOrMore if lookahead.peek(Token![+]) => Applicability::Unary,
            Self::ZeroOrMore if lookahead.peek(Token![*]) => Applicability::Unary,
            Self::Optional if lookahead.peek(Token![?]) => Applicability::Unary,

            _ => Applicability::Mismatch,
        }
    }

    #[inline]
    fn parse(&mut self, input: ParseStream) -> Result<()> {
        match self {
            Self::Union => drop(input.parse::<Token![|]>()?),
            Self::Concat => drop(input.parse::<Token![&]>()?),
            Self::OneOrMore => drop(input.parse::<Token![+]>()?),
            Self::ZeroOrMore => drop(input.parse::<Token![*]>()?),
            Self::Optional => drop(input.parse::<Token![?]>()?),
        };

        Ok(())
    }
}
