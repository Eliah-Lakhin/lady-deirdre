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

use std::mem::take;

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

            Self::Operand(Operand::Transform(_, inner)) => {
                let inner = expect_some!(inner, "Empty transformation.",);

                inner.name()
            }

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

    fn transform(&mut self, config: &TransformConfig) {
        match self {
            Self::Operand(Operand::Unresolved(_)) => system_panic!("Unresolved operand."),

            Self::Operand(Operand::Dump(_, inner)) => inner.transform(config),

            Self::Operand(Operand::Transform(feature, inner)) => {
                let mut inner = expect_some!(take(inner), "Empty transformation.",);

                let config = config.add(feature);

                inner.transform(&config);

                *self = *inner;
            }

            Self::Operand(Operand::Class(span, class)) => {
                if let Some(new_regex) = config.transform_class(span, class) {
                    *self = new_regex;
                }
            }

            Self::Operand(Operand::Exclusion(set)) => {
                config.transform_char_set(set);
            }

            Self::Binary(left, _, right) => {
                left.transform(config);
                right.transform(config);
            }

            Self::Unary(_, inner) => inner.transform(config),
        }
    }

    fn alphabet(&self) -> Alphabet {
        match self {
            Self::Operand(Operand::Unresolved(_)) => system_panic!("Unresolved operand."),

            Self::Operand(Operand::Dump(_, inner)) => inner.alphabet(),

            Self::Operand(Operand::Transform(_, _)) => {
                system_panic!("Unresolved transformation.");
            }

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

            Self::Operand(Operand::Transform(_, _)) => {
                system_panic!("Unresolved transformation.");
            }

            Self::Operand(Operand::Exclusion(set)) => {
                let mut alphabet = alphabet.clone();

                for exclusion in &set.classes {
                    match exclusion {
                        Class::Char(ch) => {
                            let _ = alphabet.remove(ch);
                        }

                        Class::Props(_) => {
                            system_panic!("Exclusion contains Property class.");
                        }

                        Class::Other => {
                            system_panic!("Exclusion contains Other class.");
                        }
                    }
                }

                let regex = alphabet.into_iter().map(|ch| Class::Char(ch)).fold(
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
                    Class::Props(props) => {
                        *self = expand_class(*span, Class::Props(*props), alphabet)
                    }
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

            Self::Operand(Operand::Transform(_, inner)) => {
                let inner = expect_some!(inner, "Empty transformation.",);

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

            Self::Operand(Operand::Transform(_, inner)) => {
                let inner = expect_some!(inner, "Empty transformation.",);

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
                    " -- Macro Debug Dump --\n\nThis expression is \
                    subject to debugging.\nState machine transitions \
                    are:\n{automata:#}\n",
                ))
            }

            Self::Operand(Operand::Transform(_, _)) => {
                system_panic!("Unresolved transformation.");
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

    fn transform(&mut self, config: &TransformConfig);

    fn alphabet(&self) -> Alphabet;

    fn expand(&mut self, alphabet: &Alphabet);

    fn inline(&mut self, inline_map: &InlineMap, variant_map: &VariantMap) -> Result<()>;

    fn set_span(&mut self, span: Span);

    fn encode(&self, scope: &mut Scope) -> Result<TokenAutomata>;
}

#[derive(Clone)]
pub(super) enum TransformFeature {
    CaseInsensitive,
}

#[derive(Default, Clone, Copy)]
pub(super) struct TransformConfig {
    case_insensitive: bool,
}

impl TransformConfig {
    #[inline(always)]
    pub(super) fn add(mut self, feature: &TransformFeature) -> Self {
        match feature {
            TransformFeature::CaseInsensitive => {
                self.case_insensitive = true;
            }
        }

        self
    }

    #[inline(always)]
    pub(super) fn transform_class(&self, span: &Span, class: &Class) -> Option<Regex> {
        let Class::Char(ch) = class else {
            return None;
        };

        let mut transformed = self.transform_char(*ch).into_iter();

        let first = transformed.next()?;

        let mut accumulator = Regex::Operand(Operand::Class(*span, Class::Char(first)));

        for next in transformed {
            accumulator = Regex::Binary(
                Box::new(accumulator),
                Operator::Union,
                Box::new(Regex::Operand(Operand::Class(*span, Class::Char(next)))),
            );
        }

        Some(accumulator)
    }

    #[inline(always)]
    pub(super) fn transform_char_set(&self, char_set: &mut CharSet) {
        if !self.case_insensitive {
            return;
        }

        let mut transformed = Set::empty();

        for class in char_set.classes.iter() {
            let Class::Char(ch) = class else {
                continue;
            };

            transformed.append(self.transform_char(*ch));
        }

        for ch in transformed {
            let _ = char_set.classes.insert(Class::Char(ch));
        }
    }

    #[inline(always)]
    fn transform_char(&self, ch: char) -> Set<char> {
        if !self.case_insensitive {
            return Set::empty();
        }

        let mut uppercase = ch.to_uppercase().collect::<Set<char>>();
        let lowercase = ch.to_lowercase().collect::<Set<char>>();

        if uppercase == lowercase {
            return Set::empty();
        }

        uppercase.append(lowercase);

        let _ = uppercase.insert(ch);

        uppercase
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
    Transform(TransformFeature, Option<Box<Regex>>),
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
            let set = CharSet::parse_brackets(input, false)?;

            let expr = expect_some!(set.into_expr(), "Empty CharSet.",);

            return Ok(expr);
        }

        if lookahead.peek(Token![^]) {
            let _ = input.parse::<Token![^]>()?;
            let set = CharSet::parse_brackets(input, true)?;

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

        if lookahead.peek(functions_kw::i) {
            let _ = input.parse::<functions_kw::i>()?;

            let content;
            parenthesized!(content in input);

            let inner = content.parse::<Regex>()?;

            if !content.is_empty() {
                return Err(content.error("Unexpected expression end."));
            }

            return Ok(Regex::Operand(Operand::Transform(
                TransformFeature::CaseInsensitive,
                Some(Box::new(inner)),
            )));
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

        if input.peek(functions_kw::i) {
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

mod functions_kw {
    syn::custom_keyword!(i);
}
