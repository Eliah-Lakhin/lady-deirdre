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

use proc_macro2::Ident;
use std::time::Instant;
use syn::{
    parse::{Parse, ParseStream},
    AttrStyle, Data, DeriveInput, Error, Generics, Result,
};

use crate::utils::{debug_panic, OptimizationStrategy};
use crate::{
    token::{
        characters::CharacterSet,
        compiler::Compiler,
        regex::{InlineMap, Regex, RegexImpl},
        rule::RuleMeta,
        scope::Scope,
        terminal::Terminal,
        transition::Transition,
        variant::TokenVariant,
    },
    utils::{AutomataContext, Facade, Map, PredictableCollection, Set, SetImpl},
    BENCHMARK,
};

pub struct Token {
    pub(super) token_name: Ident,
    pub(super) generics: Generics,
    pub(super) rules: Vec<RuleMeta>,
    pub(super) mismatch: Ident,
    pub(super) transitions: Vec<Transition>,
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
                    _ => debug_panic!("Unsupported Item format."),
                };

                return Err(Error::new(
                    span,
                    "Token must be derived on the enum type with variants representing \
                    language lexis.",
                ));
            }
        };

        let mut inline_map = InlineMap::empty();

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
                        debug_panic!("Inline expression redefined.");
                    }
                }

                _ => continue,
            }
        }

        let mut mismatch: Option<Ident> = None;
        let mut rules = Vec::with_capacity(data.variants.len());

        for variant in data.variants.into_iter() {
            let variant = TokenVariant::from_variant(variant, rules.len(), &inline_map)?;

            match variant {
                TokenVariant::Mismatch { name } => {
                    if let Some(previous) = &mismatch {
                        return Err(Error::new(
                            name.span(),
                            format!(
                                "The variant {:?} already labeled as mismatch fallback.\nToken \
                                must specify only one mismatch variant.",
                                previous.to_string(),
                            ),
                        ));
                    }

                    mismatch = Some(name);
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
                    _ => debug_panic!("Non-rule variant."),
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

                    _ => debug_panic!("Non-rule variant."),
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

        let mut products = Map::empty();
        let mut matched_products = Set::empty();

        automata.retain(|from, through, _| match through {
            Terminal::Null => debug_panic!("Automata with null transition."),
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

        let result = Ok(Compiler::compile(
            token_name.clone(),
            generics,
            rules,
            mismatch,
            scope,
            automata,
            products,
        ));

        if BENCHMARK {
            println!(
                "Token {} compile time: {:?}",
                token_name,
                compile_start.elapsed(),
            )
        }

        result
    }
}

impl From<Token> for proc_macro::TokenStream {
    fn from(mut input: Token) -> Self {
        let facade = Facade::new();
        let core = facade.core_crate();

        let token_name = input.token_name;
        let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

        let transitions = input
            .transitions
            .iter()
            .map(|transition| transition.output(&facade, &mut input.rules))
            .collect::<Vec<_>>();

        let start = 1usize;
        let mismatch = &input.mismatch;

        let mut token_in_use = false;
        let mut kind_in_use = false;

        for rule in &input.rules {
            token_in_use = token_in_use || rule.uses_token_variable();
            kind_in_use = kind_in_use || rule.uses_kind_variable();
        }

        let token_init = match token_in_use {
            false => None,
            true => Some(quote! {
                let mut token = Self::#mismatch;
            }),
        };

        let kind_init = match kind_in_use {
            false => None,
            true => Some(quote! {
                let mut kind = 0;
            }),
        };

        let result = match kind_in_use {
            false => {
                if token_in_use {
                    quote! { token }
                } else {
                    quote! { Self::#mismatch }
                }
            }

            true => {
                let variants = input.rules.into_iter().map(|rule| {
                    let index = rule.index();
                    let in_place = rule.output_in_place(&facade);

                    quote! { #index => #in_place }
                });

                if token_in_use {
                    quote! {
                        match kind {
                            #( #variants, )*
                            _ => token,
                        }
                    }
                } else {
                    quote! {
                        match kind {
                            #( #variants, )*
                            _ => Self::#mismatch,
                        }
                    }
                }
            }
        };

        let output = quote! {
            impl #impl_generics #core::lexis::Token for #token_name #ty_generics
            #where_clause
            {
                fn new(session: &mut impl #core::lexis::LexisSession) -> Self {
                    #[allow(unused_mut)]
                    let mut state = #start;
                    #token_init;
                    #kind_init;

                    loop {
                        let current = #core::lexis::LexisSession::character(session);
                        #core::lexis::LexisSession::advance(session);
                        let next = #core::lexis::LexisSession::character(session);

                        match (state, current, next) {
                            #( #transitions )*
                            _ => break,
                        }
                    }

                    #result
                }
            }
        };

        output.into()
    }
}
