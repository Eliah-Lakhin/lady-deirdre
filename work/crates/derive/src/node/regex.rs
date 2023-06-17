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

use std::mem::take;

use proc_macro2::{Ident, Span};
use syn::{
    parse::{Lookahead1, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Result,
};

use crate::{
    node::{
        automata::{NodeAutomata, NodeAutomataImpl, Scope, Terminal},
        input::VariantMap,
        leftmost::Leftmost,
        token::TokenLit,
        variables::VariableMap,
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
        Map,
        PredictableCollection,
        Set,
        SetImpl,
        Strategy,
    },
};

pub(super) type Regex = Expression<Operator>;
pub(super) type InlineMap = Map<Ident, Regex>;

impl RegexImpl for Regex {
    fn alphabet(&self) -> Set<TokenLit> {
        match self {
            Self::Operand(Operand::Unresolved { .. }) => system_panic!("Unresolved operand."),

            Self::Operand(Operand::Dump(_, inner)) => inner.alphabet(),

            Self::Operand(Operand::Token(_, lit)) => Set::new([lit.clone()]),

            Self::Operand(Operand::Rule(..)) => Set::empty(),

            Self::Operand(Operand::Exclusion(_, _, lits)) => lits.clone(),

            Self::Binary(left, _, right) => left.alphabet().merge(right.alphabet()),

            Self::Unary(op, inner) => {
                let inner = inner.alphabet();

                match op {
                    Operator::OneOrMore(Some(sep)) => sep.alphabet().merge(inner),
                    Operator::ZeroOrMore(Some(sep)) => sep.alphabet().merge(inner),
                    _ => inner,
                }
            }
        }
    }

    fn expand(&mut self, alphabet: &Set<TokenLit>) {
        match self {
            Self::Operand(Operand::Exclusion(capture, span, lits)) => {
                let mut rest = alphabet.clone();

                let _ = rest.insert(TokenLit::Other(*span));

                for excluded in lits.iter() {
                    let _ = rest.remove(&excluded);
                }

                let regex = rest.into_iter().fold(None, |result, mut token| {
                    token.set_span(*span);

                    let right = Regex::Operand(Operand::Token(capture.clone(), token));

                    Some(match result {
                        None => right,
                        Some(left) => {
                            Regex::Binary(Box::new(left), Operator::Union, Box::new(right))
                        }
                    })
                });

                *self = expect_some!(regex, "Exclusion is void.",);
            }

            Self::Operand(Operand::Dump(_, inner)) => inner.expand(alphabet),

            Self::Operand(..) => (),

            Self::Binary(left, _, right) => {
                left.expand(alphabet);
                right.expand(alphabet);
            }

            Self::Unary(op, inner) => {
                inner.expand(alphabet);

                match op {
                    Operator::OneOrMore(Some(sep)) => sep.expand(alphabet),
                    Operator::ZeroOrMore(Some(sep)) => sep.expand(alphabet),
                    _ => (),
                }
            }
        }
    }

    fn inline(&mut self, map: &InlineMap) -> Result<()> {
        match self {
            Self::Operand(Operand::Unresolved(capture, name)) => {
                match map.get(name) {
                    None => *self = Self::Operand(Operand::Rule(take(capture), name.clone())),

                    Some(inline) => {
                        let mut inline = inline.clone();

                        inline.set_span(name.span());

                        if let Some(target) = capture {
                            inline.set_capture(target)?;
                        }

                        *self = inline;
                    }
                };

                Ok(())
            }

            Self::Operand(Operand::Dump(_, inner)) => inner.inline(map),

            Self::Operand(..) => Ok(()),

            Self::Binary(left, _, right, ..) => {
                left.inline(map)?;
                right.inline(map)?;

                Ok(())
            }

            Self::Unary(op, inner) => {
                match op {
                    Operator::ZeroOrMore(Some(sep)) => sep.inline(map)?,
                    Operator::OneOrMore(Some(sep)) => sep.inline(map)?,
                    _ => (),
                }

                inner.inline(map)
            }
        }
    }

    fn set_capture(&mut self, target: &Ident) -> Result<()> {
        match self {
            Self::Operand(
                Operand::Unresolved(capture, _)
                | Operand::Token(capture, _)
                | Operand::Rule(capture, _)
                | Operand::Exclusion(capture, _, _),
            ) => {
                if let Some(capture) = capture {
                    if capture != target {
                        return Err(error!(
                            target.span(),
                            "Capturing variable \"{target}\" conflicts with \
                            inner capturing variable \"{capture}\".",
                        ));
                    }
                }

                *capture = Some(target.clone());

                Ok(())
            }

            Self::Operand(Operand::Dump(_, inner)) => inner.set_capture(target),

            Self::Binary(left, _, right) => {
                left.set_capture(target)?;
                right.set_capture(target)?;

                Ok(())
            }

            Self::Unary(_, inner) => inner.set_capture(target),
        }
    }

    fn set_span(&mut self, span: Span) {
        match self {
            Self::Operand(Operand::Unresolved(None, name)) => {
                name.set_span(span);
            }

            Self::Operand(Operand::Unresolved(Some(capture), name)) => {
                capture.set_span(span);
                name.set_span(span);
            }

            Self::Operand(Operand::Dump(dump_span, inner)) => {
                *dump_span = span;
                inner.set_span(span);
            }

            Self::Operand(Operand::Token(None, lit)) => {
                lit.set_span(span);
            }

            Self::Operand(Operand::Token(Some(capture), lit)) => {
                capture.set_span(span);
                lit.set_span(span);
            }

            Self::Operand(Operand::Rule(None, name)) => {
                name.set_span(span);
            }

            Self::Operand(Operand::Rule(Some(capture), name)) => {
                capture.set_span(span);
                name.set_span(span);
            }

            Self::Operand(Operand::Exclusion(None, exclusion_span, _)) => {
                *exclusion_span = span;
            }

            Self::Operand(Operand::Exclusion(Some(capture), exclusion_span, _)) => {
                capture.set_span(span);
                *exclusion_span = span;
            }

            Self::Binary(left, _, right) => {
                left.set_span(span);
                right.set_span(span);
            }

            Self::Unary(op, inner) => {
                inner.set_span(span);

                match op {
                    Operator::OneOrMore(Some(sep)) => sep.set_span(span),
                    Operator::ZeroOrMore(Some(sep)) => sep.set_span(span),
                    _ => (),
                }
            }
        }
    }

    fn refs(&self, trivia: bool, map: &VariantMap) -> Result<Set<Ident>> {
        match self {
            Self::Operand(Operand::Unresolved(..)) => system_panic!("Unresolved operand."),

            Self::Operand(Operand::Exclusion(capture, _, _)) => {
                if trivia && capture.is_some() {
                    return Err(error!(capture.span(), "Trivia expressions cannot capture.",));
                }

                Ok(Set::empty())
            }

            Self::Operand(Operand::Dump(_, inner)) => inner.refs(trivia, map),

            Self::Operand(Operand::Rule(capture, name)) => {
                if trivia && capture.is_some() {
                    return Err(error!(capture.span(), "Trivia expressions cannot capture.",));
                }

                return match map.get(name) {
                    None => Err(error!(
                        name.span(),
                        "Reference to unknown rule \"{name}\".\nTry to \
                        introduce an enum variant with this name.",
                    )),

                    Some(variant) if variant.rule.is_none() => Err(error!(
                        name.span(),
                        "Reference to unparseable rule \"{name}\".\n\
                        Annotate that enum variant with #[rule(...)] attribute.",
                    )),

                    Some(variant) if variant.root.is_some() => Err(error!(
                        name.span(),
                        "Reference \"{name}\" points to the root rule.\nRoot \
                        rule cannot be referred.",
                    )),

                    _ => Ok(Set::new([name.clone()])),
                };
            }

            Self::Operand(Operand::Token(capture, _)) => {
                if trivia && capture.is_some() {
                    return Err(error!(capture.span(), "Trivia expressions cannot capture.",));
                }

                Ok(Set::empty())
            }

            Self::Binary(left, _, right) => {
                let left = left.refs(trivia, map)?;
                let right = right.refs(trivia, map)?;

                Ok(left.merge(right))
            }

            Self::Unary(op, inner) => {
                let inner = inner.refs(trivia, map)?;

                Ok(match op {
                    Operator::OneOrMore(Some(sep)) => inner.merge(sep.refs(trivia, map)?),
                    Operator::ZeroOrMore(Some(sep)) => inner.merge(sep.refs(trivia, map)?),
                    _ => inner,
                })
            }
        }
    }

    fn encode(&self, scope: &mut Scope) -> Result<NodeAutomata> {
        match self {
            Self::Operand(Operand::Unresolved(..)) => system_panic!("Unresolved operand."),

            Self::Operand(Operand::Exclusion(..)) => system_panic!("Unresolved exclusion."),

            Self::Operand(Operand::Dump(span, inner)) => {
                let leftmost = Leftmost::from(inner);
                scope.set_strategy(Strategy::CANONICALIZE);
                let mut automata = inner.encode(scope)?;

                automata.merge_captures(scope)?;

                let variables = VariableMap::try_from(&automata)?;

                return Err(error!(
                    *span,
                    " -- Macro System Debug Dump --\n\nThis expression is a \
                    subject for debugging.\n\nCapturing variables \
                    are:\n{variables:#}\nState machine transitions \
                    are:\n{automata:#}\nLeftmost set is:\n{leftmost:#}\n",
                ));
            }

            Self::Operand(Operand::Token(capture, lit)) => {
                Ok(scope.terminal(Set::new([Terminal::Token(capture.clone(), lit.clone())])))
            }

            Self::Operand(Operand::Rule(capture, name)) => {
                Ok(scope.terminal(Set::new([Terminal::Node(capture.clone(), name.clone())])))
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
                    Operator::OneOrMore(None) => Ok(scope.repeat_one(inner)),

                    Operator::OneOrMore(Some(sep)) => {
                        let separator = sep.encode(scope)?;

                        let rest = {
                            let inner = scope.copy(&inner);
                            scope.concatenate(separator, inner)
                        };
                        let repeat_rest = scope.repeat_zero(rest);

                        Ok(scope.concatenate(inner, repeat_rest))
                    }

                    Operator::ZeroOrMore(None) => Ok(scope.repeat_zero(inner)),

                    Operator::ZeroOrMore(Some(sep)) => {
                        let sep = sep.encode(scope)?;

                        let rest = {
                            let inner = scope.copy(&inner);
                            scope.concatenate(sep, inner)
                        };
                        let repeat_rest = scope.repeat_zero(rest);
                        let one_or_more = scope.concatenate(inner, repeat_rest);

                        Ok(scope.optional(one_or_more))
                    }

                    Operator::Optional => Ok(scope.optional(inner)),

                    _ => system_panic!("Unsupported Unary operator."),
                }
            }
        }
    }
}

pub(super) trait RegexImpl {
    fn alphabet(&self) -> Set<TokenLit>;
    fn expand(&mut self, alphabet: &Set<TokenLit>);
    fn inline(&mut self, map: &InlineMap) -> Result<()>;
    fn set_capture(&mut self, target: &Ident) -> Result<()>;
    fn set_span(&mut self, span: Span);
    fn refs(&self, trivia: bool, map: &VariantMap) -> Result<Set<Ident>>;
    fn encode(&self, scope: &mut Scope) -> Result<NodeAutomata>;
}

#[derive(Clone)]
pub(super) enum Operand {
    Unresolved(Option<Ident>, Ident),
    Dump(Span, Box<Regex>),
    Token(Option<Ident>, TokenLit),
    Rule(Option<Ident>, Ident),
    Exclusion(Option<Ident>, Span, Set<TokenLit>),
}

impl Default for Operand {
    #[inline(always)]
    fn default() -> Self {
        Self::Unresolved(None, Ident::new("_", Span::call_site()))
    }
}

impl ExpressionOperand<Operator> for Operand {
    fn parse(input: ParseStream) -> Result<Regex> {
        fn parse_token_lit_set(input: ParseStream) -> Result<(Span, Set<TokenLit>)> {
            let content;
            bracketed!(content in input);

            let span = content.span();
            let sequence = Punctuated::<TokenLit, Token![|]>::parse_separated_nonempty(&content)?;

            if !content.is_empty() {
                return Err(content.error(
                    "Unexpected end of input.\nExpected a set \
                    of tokens [$A | $B | ...].",
                ));
            }

            let mut lits = Set::with_capacity(sequence.len());

            for token_lit in sequence {
                if lits.contains(&token_lit) {
                    return Err(error!(token_lit.span(), "Duplicate token.",));
                }

                let _ = lits.insert(token_lit);
            }

            Ok((span, lits))
        }

        let lookahead = input.lookahead1();

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

            if input.peek(Token![:]) {
                let _ = input.parse::<Token![:]>()?;

                let lookahead = input.lookahead1();

                if input.peek(Token![$]) {
                    return Ok(Regex::Operand(Operand::Token(
                        Some(ident),
                        input.parse::<TokenLit>()?,
                    )));
                }

                if input.peek(Token![^]) {
                    let _ = input.parse::<Token![^]>()?;
                    let (span, set) = parse_token_lit_set(input)?;

                    return Ok(Regex::Operand(Operand::Exclusion(Some(ident), span, set)));
                }

                if input.peek(Token![.]) {
                    let span = input.parse::<Token![.]>()?.span;

                    return Ok(Regex::Operand(Operand::Exclusion(
                        Some(ident),
                        span,
                        Set::empty(),
                    )));
                }

                if lookahead.peek(syn::Ident) {
                    return Ok(Regex::Operand(Operand::Unresolved(
                        Some(ident),
                        input.parse::<Ident>()?,
                    )));
                }

                if lookahead.peek(syn::token::Paren) {
                    let content;
                    parenthesized!(content in input);

                    let mut result = content.parse::<Regex>()?;

                    if !content.is_empty() {
                        return Err(content.error("Unexpected expression end."));
                    }

                    result.set_capture(&ident)?;

                    return Ok(result);
                }

                return Err(lookahead.error());
            }

            return Ok(Regex::Operand(Operand::Unresolved(None, ident)));
        }

        if lookahead.peek(Token![$]) {
            return Ok(Regex::Operand(Operand::Token(
                None,
                input.parse::<TokenLit>()?,
            )));
        }

        if lookahead.peek(Token![^]) {
            let _ = input.parse::<Token![^]>()?;
            let (span, set) = parse_token_lit_set(input)?;

            return Ok(Regex::Operand(Operand::Exclusion(None, span, set)));
        }

        if lookahead.peek(Token![.]) {
            let span = input.parse::<Token![.]>()?.span;

            return Ok(Regex::Operand(Operand::Exclusion(None, span, Set::empty())));
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

    fn test(input: ParseStream) -> bool {
        if input.peek(dump_kw::dump) {
            return true;
        }

        if input.peek(syn::Ident) {
            return true;
        }

        if input.peek(Token![$]) {
            return true;
        }

        if input.peek(Token![^]) {
            return true;
        }

        if input.peek(Token![.]) {
            return true;
        }

        if input.peek(syn::token::Paren) {
            return true;
        }

        false
    }
}

#[derive(Clone)]
pub(super) enum Operator {
    Union,
    Concat,
    Optional,
    OneOrMore(Option<Box<Regex>>),
    ZeroOrMore(Option<Box<Regex>>),
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
            Self::Optional,
            Self::OneOrMore(None),
            Self::ZeroOrMore(None),
        ]
    }

    #[inline(always)]
    fn binding_power(&self) -> u8 {
        match self {
            Self::Union => 10,
            Self::Concat => 20,
            Self::OneOrMore(..) => 30,
            Self::ZeroOrMore(..) => 40,
            Self::Optional => 50,
        }
    }

    #[inline]
    fn test(&self, input: ParseStream, lookahead: &Lookahead1) -> Applicability {
        match self {
            Self::Union if lookahead.peek(Token![|]) => Applicability::Binary,
            Self::Concat if lookahead.peek(Token![&]) => Applicability::Binary,
            Self::Concat if Operand::test(input) => Applicability::Binary,
            Self::Optional if lookahead.peek(Token![?]) => Applicability::Unary,
            Self::OneOrMore(..) if lookahead.peek(Token![+]) => Applicability::Unary,
            Self::ZeroOrMore(..) if lookahead.peek(Token![*]) => Applicability::Unary,

            _ => Applicability::Mismatch,
        }
    }

    #[inline]
    fn parse(&mut self, input: ParseStream) -> Result<()> {
        match self {
            Self::Union => drop(input.parse::<Token![|]>()?),

            Self::Concat => {
                if input.peek(Token![&]) {
                    drop(input.parse::<Token![&]>()?)
                }
            }

            Self::Optional => drop(input.parse::<Token![?]>()?),

            Self::OneOrMore(sep) => {
                let _ = input.parse::<Token![+]>()?;

                if input.peek(syn::token::Brace) {
                    let content;

                    braced!(content in input);

                    *sep = Some(Box::new(content.parse::<Regex>()?));
                }
            }

            Self::ZeroOrMore(sep) => {
                let _ = input.parse::<Token![*]>()?;

                if input.peek(syn::token::Brace) {
                    let content;

                    braced!(content in input);

                    *sep = Some(Box::new(content.parse::<Regex>()?));
                }
            }
        };

        Ok(())
    }
}
