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
    LitStr,
    Result,
};

use crate::{
    token::{
        automata::{Scope, Terminal, TokenAutomata},
        chars::{CharSet, Class},
        input::{Alphabet, InlineMap, VariantMap},
    },
    utils::{
        dump_kw,
        error,
        expect_some,
        system_panic,
        Applicability,
        AutomataContext,
        Expression,
        ExpressionOperand,
        ExpressionOperator,
        PredictableCollection,
        Set,
        SetImpl,
        Strategy,
    },
};

pub(super) type Regex = Expression<Operator>;

impl RegexImpl for Regex {
    fn name(&self) -> Option<String> {
        match self {
            Self::Operand(Operand::Class(_, Class::Char(ch)))
                if !ch.is_ascii_control() && *ch != ' ' =>
            {
                Some(String::from(*ch))
            }

            Self::Operand(Operand::Dump(_, inner)) => inner.name(),

            Self::Operand(_) => None,

            Self::Binary(left, Operator::Concat, right) => {
                let mut left = left.name()?;
                let right = right.name()?;

                left += right.as_str();

                Some(left)
            }

            Self::Binary(_, _, _) => None,

            Self::Unary(_, _) => None,
        }
    }

    fn alphabet(&self) -> Alphabet {
        match self {
            Self::Operand(Operand::Unresolved(_)) => system_panic!("Unresolved operand."),

            Self::Operand(Operand::Dump(_, inner)) => inner.alphabet(),

            Self::Operand(Operand::Class(_, Class::Char(ch))) => Set::new([*ch]),

            Self::Operand(Operand::Class(_, _)) => Set::empty(),

            Self::Operand(Operand::Exclusion(set)) => set
                .classes
                .clone()
                .into_iter()
                .filter_map(|class| match class {
                    Class::Char(ch) => Some(ch),
                    _ => None,
                })
                .collect(),

            Self::Binary(left, _, right) => left.alphabet().merge(right.alphabet()),

            Self::Unary(_, inner) => inner.alphabet(),
        }
    }

    fn expand(&mut self, alphabet: &Alphabet) {
        match self {
            Self::Operand(Operand::Unresolved(_)) => system_panic!("Unresolved operand."),

            Self::Operand(Operand::Dump(_, inner)) => inner.expand(alphabet),

            Self::Operand(Operand::Exclusion(set)) => {
                let mut alphabet = alphabet.clone();

                let mut upper = true;
                let mut lower = true;
                let mut num = true;
                let mut space = true;

                for exclusion in &set.classes {
                    match exclusion {
                        Class::Char(ch) => {
                            let _ = alphabet.remove(ch);
                        }

                        Class::Upper => {
                            alphabet.retain(|ch| !Class::Upper.includes(ch));
                            upper = false;
                        }

                        Class::Lower => {
                            alphabet.retain(|ch| !Class::Lower.includes(ch));
                            lower = false;
                        }

                        Class::Num => {
                            alphabet.retain(|ch| !Class::Num.includes(ch));
                            num = false;
                        }

                        Class::Space => {
                            alphabet.retain(|ch| !Class::Space.includes(ch));
                            space = false;
                        }

                        Class::Other => {
                            system_panic!("Exclusion contains Other class.");
                        }
                    }
                }

                let mut generics = Vec::with_capacity(4);

                if upper {
                    generics.push(Class::Upper);
                }

                if lower {
                    generics.push(Class::Lower);
                }

                if num {
                    generics.push(Class::Num);
                }

                if space {
                    generics.push(Class::Space);
                }

                let regex = alphabet
                    .into_iter()
                    .map(|ch| Class::Char(ch))
                    .chain(generics)
                    .fold(
                        Self::Operand(Operand::Class(set.span, Class::Other)),
                        |left, right| {
                            let right = Self::Operand(Operand::Class(set.span, right));

                            Self::Binary(Box::new(left), Operator::Union, Box::new(right))
                        },
                    );

                *self = regex;
            }

            Self::Operand(Operand::Class(span, class)) => {
                fn expand_class(span: Span, class: Class, alphabet: &Alphabet) -> Regex {
                    alphabet
                        .iter()
                        .filter_map(|ch| match class.includes(ch) {
                            false => None,
                            true => Some(Class::Char(*ch)),
                        })
                        .fold(
                            Regex::Operand(Operand::Class(span, class)),
                            |left, right| {
                                let right = Regex::Operand(Operand::Class(span, right));

                                Regex::Binary(Box::new(left), Operator::Union, Box::new(right))
                            },
                        )
                }

                match class {
                    Class::Char(_) => (),
                    Class::Upper => *self = expand_class(*span, Class::Upper, alphabet),
                    Class::Lower => *self = expand_class(*span, Class::Lower, alphabet),
                    Class::Num => *self = expand_class(*span, Class::Num, alphabet),
                    Class::Space => *self = expand_class(*span, Class::Space, alphabet),
                    Class::Other => system_panic!("Explicit Other class."),
                }
            }

            Self::Binary(left, _, right) => {
                left.expand(alphabet);
                right.expand(alphabet);
            }

            Self::Unary(_, inner) => inner.expand(alphabet),
        }
    }

    fn inline(&mut self, inline_map: &InlineMap, variant_map: &VariantMap) -> Result<()> {
        match self {
            Self::Operand(Operand::Unresolved(ident)) if variant_map.contains_key(ident) => {
                return Err(error!(
                    ident.span(),
                    "\"{ident}\" refers enum variant.\nRule references not \
                    allowed in the lexis grammar.",
                ));
            }

            Self::Operand(Operand::Unresolved(ident)) if inline_map.contains_key(ident) => {
                let mut inline = expect_some!(inline_map.get(ident), "Missing inline.",).clone();

                inline.set_span(ident.span());

                *self = inline;
            }

            Self::Operand(Operand::Unresolved(ident)) => {
                if ident == "alpha" {
                    return Err(error!(
                        ident.span(),
                        "Unknown inline expression \"{ident}\". Maybe you \
                        mean ${ident} - a class of all alphabetic \
                        characters?\nOtherwise, annotate enum type with \
                        #[define({ident} = ...)] attribute to introduce \
                        corresponding inline expression.",
                    ));
                }

                if ident == "alphanum" {
                    return Err(error!(
                        ident.span(),
                        "Unknown inline expression \"{ident}\". Maybe you \
                        mean ${ident} - a class of all alphabetic and numeric \
                        characters?\nOtherwise, annotate enum type with \
                        #[define({ident} = ...)] attribute to introduce \
                        corresponding inline expression.",
                    ));
                }

                if ident == "upper" {
                    return Err(error!(
                        ident.span(),
                        "Unknown inline expression \"{ident}\". Maybe you \
                        mean ${ident} - a class of all upper-cased alphabetic \
                        characters?\nOtherwise, annotate enum type with \
                        #[define({ident} = ...)] attribute to introduce \
                        corresponding inline expression.",
                    ));
                }

                if ident == "lower" {
                    return Err(error!(
                        ident.span(),
                        "Unknown inline expression \"{ident}\". Maybe you \
                        mean ${ident} - a class of all lower-cased alphabetic \
                        characters?\nOtherwise, annotate enum type with \
                        #[define({ident} = ...)] attribute to introduce \
                        corresponding inline expression.",
                    ));
                }

                if ident == "num" {
                    return Err(error!(
                        ident.span(),
                        "Unknown inline expression \"{ident}\". Maybe you \
                        mean ${ident} - a class of all numeric \
                        characters?\nOtherwise, annotate enum type with \
                        #[define({ident} = ...)] attribute to introduce \
                        corresponding inline expression.",
                    ));
                }

                if ident == "space" {
                    return Err(error!(
                        ident.span(),
                        "Unknown inline expression \"{ident}\". Maybe you \
                        mean ${ident} - a class of all whitespace \
                        characters?\nOtherwise, annotate enum type with \
                        #[define({ident} = ...)] attribute to introduce \
                        corresponding inline expression.",
                    ));
                }

                return Err(error!(
                    ident.span(),
                    "Unknown inline expression \"{ident}\".\nAnnotate \
                    enum type with #[define({ident} = ...)] attribute to \
                    introduce corresponding inline expression.",
                ));
            }

            Self::Operand(Operand::Dump(_, inner)) => {
                inner.inline(inline_map, variant_map)?;
            }

            Self::Operand(Operand::Class(_, _)) => (),

            Self::Operand(Operand::Exclusion(_)) => (),

            Self::Binary(left, _, right) => {
                left.inline(inline_map, variant_map)?;
                right.inline(inline_map, variant_map)?;
            }

            Self::Unary(_, inner) => {
                inner.inline(inline_map, variant_map)?;
            }
        }

        Ok(())
    }

    fn set_span(&mut self, span: Span) {
        match self {
            Self::Operand(Operand::Unresolved(ident)) => {
                ident.set_span(span);
            }

            Self::Operand(Operand::Dump(_, inner)) => {
                inner.set_span(span);
            }

            Self::Operand(Operand::Class(op_span, _)) => {
                *op_span = span;
            }

            Self::Operand(Operand::Exclusion(set)) => set.span = span,

            Self::Binary(left, _, right) => {
                left.set_span(span);
                right.set_span(span);
            }

            Self::Unary(_, inner) => {
                inner.set_span(span);
            }
        }
    }

    fn encode(&self, scope: &mut Scope) -> Result<TokenAutomata> {
        match self {
            Self::Operand(Operand::Unresolved(_)) => system_panic!("Unresolved operand."),

            Self::Operand(Operand::Exclusion(_)) => system_panic!("Unresolved exclusion."),

            Self::Operand(Operand::Class(_, class)) => {
                Ok(scope.terminal(Set::new([Terminal::Class(*class)])))
            }

            Self::Operand(Operand::Dump(span, inner)) => {
                scope.set_strategy(Strategy::CANONICALIZE);

                let automata = inner.encode(scope)?;

                Err(error!(
                    *span,
                    " -- Macro System Debug Dump --\n\nThis expression is a \
                    subject for debugging.\nState machine transitions \
                    are:\n{automata:#}\n",
                ))
            }

            Self::Binary(left, op, right) => {
                let left = left.encode(scope)?;
                let right = right.encode(scope)?;

                match op {
                    Operator::Union => Ok(scope.union(left, right)),
                    Operator::Concat => Ok(scope.concatenate(left, right)),
                    _ => system_panic!("Unsupported Binary operator."),
                }
            }

            Self::Unary(op, inner) => {
                let inner = inner.encode(scope)?;

                match op {
                    Operator::OneOrMore => Ok(scope.repeat_one(inner)),
                    Operator::ZeroOrMore => Ok(scope.repeat_zero(inner)),
                    Operator::Optional => Ok(scope.optional(inner)),
                    _ => system_panic!("Unsupported Unary operator."),
                }
            }
        }
    }
}

pub(super) trait RegexImpl {
    fn name(&self) -> Option<String>;

    fn alphabet(&self) -> Alphabet;

    fn expand(&mut self, alphabet: &Alphabet);

    fn inline(&mut self, inline_map: &InlineMap, variant_map: &VariantMap) -> Result<()>;

    fn set_span(&mut self, span: Span);

    fn encode(&self, scope: &mut Scope) -> Result<TokenAutomata>;
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
    fn head() -> Option<Self> {
        Some(Self::Union)
    }

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
    fn test(&self, input: ParseStream, lookahead: &Lookahead1) -> Applicability {
        match self {
            Self::Union if lookahead.peek(Token![|]) => Applicability::Binary,
            Self::Concat if lookahead.peek(Token![&]) => Applicability::Binary,
            Self::Concat if Operand::test(input) => Applicability::Binary,
            Self::OneOrMore if lookahead.peek(Token![+]) => Applicability::Unary,
            Self::ZeroOrMore if lookahead.peek(Token![*]) => Applicability::Unary,
            Self::Optional if lookahead.peek(Token![?]) => Applicability::Unary,

            _ => Applicability::Mismatch,
        }
    }

    #[inline]
    fn parse(&mut self, input: ParseStream) -> Result<()> {
        match self {
            Self::Union => {
                let _ = input.parse::<Token![|]>()?;
            }

            Self::Concat => {
                if input.peek(Token![&]) {
                    let _ = input.parse::<Token![&]>()?;
                }
            }

            Self::OneOrMore => {
                let _ = input.parse::<Token![+]>()?;
            }

            Self::ZeroOrMore => {
                let _ = input.parse::<Token![*]>()?;
            }

            Self::Optional => {
                let _ = input.parse::<Token![?]>()?;
            }
        };

        Ok(())
    }
}

#[derive(Clone)]
pub(super) enum Operand {
    Unresolved(Ident),
    Dump(Span, Box<Regex>),
    Class(Span, Class),
    Exclusion(CharSet),
}

impl ExpressionOperand<Operator> for Operand {
    fn parse(input: ParseStream) -> Result<Regex> {
        let lookahead = input.lookahead1();

        if CharSet::peek(&lookahead) {
            let set = input.parse::<CharSet>()?;

            let expr = expect_some!(set.into_expr(), "Empty CharSet.",);

            return Ok(expr);
        }

        if lookahead.peek(syn::LitStr) {
            let lit = input.parse::<LitStr>()?;
            let string = lit.value();
            let span = lit.span();

            return string
                .chars()
                .fold(None, |accumulator, ch| {
                    let right = Regex::Operand(Operand::Class(span, Class::Char(ch)));

                    Some(match accumulator {
                        None => right,
                        Some(left) => {
                            Regex::Binary(Box::new(left), Operator::Concat, Box::new(right))
                        }
                    })
                })
                .ok_or_else(|| error!(lit.span(), "Empty strings forbidden.",));
        }

        if lookahead.peek(Token![.]) {
            return Ok(Regex::Operand(Operand::Exclusion(CharSet::empty(
                input.parse::<Token![.]>()?.span,
            ))));
        }

        if lookahead.peek(syn::token::Bracket) {
            let set = CharSet::parse_brackets(input)?;

            let expr = expect_some!(set.into_expr(), "Empty CharSet.",);

            return Ok(expr);
        }

        if lookahead.peek(Token![^]) {
            let _ = input.parse::<Token![^]>()?;
            let set = CharSet::parse_brackets(input)?;

            return Ok(Regex::Operand(Operand::Exclusion(set)));
        }

        if lookahead.peek(dump_kw::dump) {
            let _ = input.parse::<dump_kw::dump>()?;

            let content;
            parenthesized!(content in input);

            let span = content.span();
            let inner = content.parse::<Regex>()?;

            if !content.is_empty() {
                return Err(content.error("Unexpected expression end."));
            }

            return Ok(Regex::Operand(Operand::Dump(span, Box::new(inner))));
        }

        if lookahead.peek(syn::Ident) {
            let ident = input.parse::<Ident>()?;

            return Ok(Regex::Operand(Operand::Unresolved(ident)));
        }

        if lookahead.peek(syn::token::Paren) {
            let content;

            parenthesized!(content in input);

            return content.parse::<Regex>();
        }

        Err(lookahead.error())
    }

    fn test(input: ParseStream) -> bool {
        if {
            let lookahead = input.lookahead1();
            CharSet::peek(&lookahead)
        } {
            return true;
        }

        if input.peek(syn::LitStr) {
            return true;
        }

        if input.peek(Token![.]) {
            return true;
        }

        if input.peek(syn::token::Bracket) {
            return true;
        }

        if input.peek(Token![^]) {
            return true;
        }

        if input.peek(dump_kw::dump) {
            return true;
        }

        if input.peek(syn::Ident) {
            return true;
        }

        if input.peek(syn::token::Paren) {
            return true;
        }

        false
    }
}
