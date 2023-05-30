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

use std::time::Instant;

use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    AttrStyle,
    Data,
    DeriveInput,
    Error,
    Generics,
    LitStr,
    Result,
};

use crate::{
    token::{
        characters::CharacterSet,
        regex::{InlineMap, Regex, RegexImpl},
        rule::{RuleIndex, RuleMeta},
        scope::Scope,
        terminal::Terminal,
        variant::TokenVariant,
    },
    utils::{
        null,
        system_panic,
        Automata,
        AutomataContext,
        Facade,
        Map,
        OptimizationStrategy,
        PredictableCollection,
        Set,
        SetImpl,
        State,
    },
    BENCHMARK,
};

pub struct Token {
    pub(super) token_name: Ident,
    pub(super) generics: Generics,
    pub(super) rules: Vec<RuleMeta>,
    pub(super) mismatch: (Ident, LitStr),
    pub(super) scope: Scope,
    pub(super) automata: Automata<Scope>,
    pub(super) products: Map<State, RuleIndex>,
}

impl Parse for Token {
    fn parse(input: ParseStream) -> Result<Self> {
        let compile_start = Instant::now();

        let input = input.parse::<DeriveInput>()?;

        let token_name = input.ident;
        let generics = input.generics;

        let data = match input.data {
            Data::Enum(data) => data,

            other => {
                let span = match other {
                    Data::Struct(data) => data.struct_token.span,
                    Data::Union(data) => data.union_token.span,
                    _ => system_panic!("Unsupported Item format."),
                };

                return Err(Error::new(
                    span,
                    "Token must be derived on the enum type with variants representing \
                    language lexis.",
                ));
            }
        };

        let mut inline_map = InlineMap::empty();
        let mut repr = false;

        for attribute in input.attrs {
            match attribute.style {
                AttrStyle::Inner(_) => continue,
                AttrStyle::Outer => (),
            }

            let name = match attribute.path.get_ident() {
                None => continue,
                Some(name) => name,
            };

            match name.to_string().as_str() {
                "define" => {
                    let (name, mut expression) =
                        attribute.parse_args_with(|input: ParseStream| {
                            let name = input.parse::<Ident>()?;
                            let _ = input.parse::<Token![=]>()?;
                            let name_string = name.to_string();

                            if inline_map.contains_key(&name_string) {
                                return Err(Error::new(
                                    name.span(),
                                    "Inline expression with this name already defined.",
                                ));
                            }

                            Ok((name_string, input.parse::<Regex>()?))
                        })?;

                    expression.inline(&inline_map)?;

                    if inline_map.insert(name, expression).is_some() {
                        system_panic!("Inline expression redefined.");
                    }
                }

                "repr" => {
                    attribute.parse_args_with(|input: ParseStream| {
                        let repr_kind = input.parse::<Ident>()?;

                        if repr_kind != "u8" {
                            return Err(Error::new(
                                repr_kind.span(),
                                "Token type must be #[repr(u8)].",
                            ));
                        }

                        if !input.is_empty() {
                            return Err(Error::new(
                                input.span(),
                                "Token type must be #[repr(u8)].",
                            ));
                        }

                        Ok(())
                    })?;

                    repr = true;
                }

                _ => continue,
            }
        }

        if !repr {
            return Err(Error::new(
                token_name.span(),
                "Token type must be #[repr(u8)].\nAnnotate this type \
                with #[repr(u8)] inert attribute.",
            ));
        }

        let mut mismatch: Option<(Ident, LitStr)> = None;
        let mut rules = Vec::with_capacity(data.variants.len());

        for variant in data.variants.into_iter() {
            let variant = TokenVariant::from_variant(variant, rules.len(), &inline_map)?;

            match variant {
                TokenVariant::Mismatch { name, description } => {
                    if let Some(previous) = &mismatch {
                        return Err(Error::new(
                            name.span(),
                            format!(
                                "The variant {:?} already labeled as mismatch fallback.\nToken \
                                must specify only one mismatch variant.",
                                previous.0.to_string(),
                            ),
                        ));
                    }

                    mismatch = Some((name, description));
                }

                TokenVariant::Other => (),

                rule @ TokenVariant::Rule { .. } => {
                    rules.push(rule);
                }
            }
        }

        let mismatch = match mismatch {
            Some(mismatch) => mismatch,

            None => {
                return Err(Error::new(
                    token_name.span(),
                    "One of the variants must be labeled as a mismatch fallback.\nUse \
                    #[mismatch] attribute to label such variant.",
                ));
            }
        };

        let alphabet = rules
            .iter()
            .fold(None, |accumulator: Option<CharacterSet>, rule| {
                let alphabet = match rule {
                    TokenVariant::Rule { expression, .. } => expression.alphabet(),
                    _ => system_panic!("Non-rule variant."),
                };

                Some(match accumulator {
                    None => alphabet,
                    Some(accumulator) => accumulator.merge(alphabet),
                })
            })
            .ok_or(Error::new(
                token_name.span(),
                "The enumeration must define at least one variant with definitive rule \
                expression.\nUse #[rule(<expression>)] attributes to label such variants.",
            ))?;

        let mut scope = Scope::new(alphabet);
        scope.set_strategy(OptimizationStrategy::NONE);

        let mut automata = rules
            .iter()
            .try_fold(None, |accumulator, rule| {
                let rule_name;
                let rule_index;
                let rule_expression;

                match rule {
                    TokenVariant::Rule {
                        name,
                        index,
                        expression,
                        ..
                    } => {
                        rule_name = name;
                        rule_index = index;
                        rule_expression = expression;
                    }

                    _ => system_panic!("Non-rule variant."),
                }

                let mut automata = rule_expression.encode(&mut scope)?;

                if automata.accepts_null() {
                    return Err(Error::new(
                        rule_name.span(),
                        "Variant's rule expression can match empty string.\nTokens of empty \
                        strings not allowed.",
                    ));
                }

                let product = scope.terminal(Set::new([Terminal::Product(*rule_index)]));

                automata = scope.concatenate(automata, product);

                Ok(Some(match accumulator {
                    None => automata,
                    Some(accumulator) => scope.union(accumulator, automata),
                }))
            })?
            .expect("Internal error. Empty rule set.");

        scope.set_strategy(OptimizationStrategy::DETERMINE);
        scope.optimize(&mut automata);

        loop {
            let mut has_changes = false;

            automata.try_map(|_, transitions| {
                let mut products = Map::empty();
                let mut conflict = None;

                transitions.retain(|(through, to)| match through {
                    Terminal::Product(index) => {
                        let precedence = rules[*index].rule_precedence();

                        if let Some((previous, _)) = products.insert(precedence, (*index, *to)) {
                            conflict = Some((*index, previous));
                        }

                        false
                    }

                    _ => true,
                });

                if let Some((a, b)) = conflict {
                    let a = rules[a].rule_name();
                    let b = rules[b].rule_name();

                    return Err(Error::new(
                        a.span(),
                        format!(
                            "This rule conflicts with {:?} rule. Both rules can match the same \
                            substring.\nTo resolve ambiguity try to label these rules with \
                            explicit distinct precedences using #[precedence(<number>)] \
                            attribute.\nDefault precedence is 1. Rules with higher precedence \
                            value have priority over the rules with lower precedence value.",
                            b.to_string(),
                        ),
                    ));
                }

                if products.len() > 1 {
                    has_changes = true;
                }

                let product = products.iter().max_by_key(|(precedence, _)| *precedence);

                if let Some((_, (index, to))) = product {
                    assert!(
                        transitions.insert((Terminal::Product(*index), *to)),
                        "Internal error. Duplicate production terminal.",
                    );
                }

                Ok(())
            })?;

            if !has_changes {
                break;
            }

            scope.optimize(&mut automata);
        }

        scope.set_strategy(OptimizationStrategy::CANONICALIZE);
        scope.optimize(&mut automata);

        scope.reset();

        automata = scope.copy(&automata);

        let mut products = Map::empty();
        let mut matched_products = Set::empty();

        automata.retain(|from, through, _| match through {
            Terminal::Null => null!(),
            Terminal::Character(..) => true,
            Terminal::Product(index) => {
                assert!(
                    products.insert(*from, *index).is_none(),
                    "Internal error. Unresolved ambiguity.",
                );

                let _ = matched_products.insert(*index);

                false
            }
        });

        for (index, rule) in rules.iter().enumerate() {
            match rule {
                TokenVariant::Rule { name, .. } if !matched_products.contains(&index) => {
                    return Err(Error::new(
                        name.span(),
                        format!(
                            "Rule {:?} is overlapping by other rules due to a low precedence. This \
                            rule never matches.\nTry to increase rule's precedence by labeling \
                            it with explicit precedence specification using \
                            #[precedence(<number>)] attribute.\nDefault precedence is 1. Rules \
                            with higher precedence value have priority over the rules with lower \
                            precedence value.",
                            name.to_string(),
                        ),
                    ));
                }

                _ => (),
            }
        }

        let rules = rules.into_iter().map(RuleMeta::from).collect();

        if BENCHMARK {
            println!(
                "Token {} build time: {:?}",
                token_name,
                compile_start.elapsed(),
            )
        }

        Ok(Self {
            token_name,
            generics,
            rules,
            mismatch,
            scope,
            automata,
            products,
        })
    }
}

impl From<Token> for proc_macro::TokenStream {
    fn from(token: Token) -> Self {
        let facade = Facade::new();

        let output = token.output(&facade);

        output.into()
    }
}
