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

use std::{cmp::Ordering, mem::take, time::Instant};

use proc_macro2::Ident;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse2,
    spanned::Spanned,
    AttrStyle,
    Data,
    DeriveInput,
    Error,
    Expr,
    File,
    Generics,
    Result,
};

use crate::{
    token::{
        automata::{AutomataImpl, Scope, Terminal, TokenAutomata},
        opt::Opt,
        regex::{Regex, RegexImpl, TransformConfig},
        variant::{TokenVariant, EOI, MISMATCH},
    },
    utils::{
        error,
        expect_some,
        system_panic,
        AutomataContext,
        Dump,
        Map,
        PredictableCollection,
        Set,
        SetImpl,
        State,
        Strategy,
    },
};

pub(super) type InlineMap = Map<Ident, Regex>;
pub(super) type VariantMap = Map<Ident, usize>;
pub(super) type Variants = Vec<TokenVariant>;
pub(super) type ProductMap = Map<State, usize>;
pub(super) type Alphabet = Set<char>;

pub struct TokenInput {
    pub(super) ident: Ident,
    pub(super) generics: Generics,
    pub(super) lookback: Option<Expr>,
    pub(super) eoi: Ident,
    pub(super) mismatch: Ident,
    pub(super) automata: TokenAutomata,
    pub(super) variants: Variants,
    pub(super) products: ProductMap,
    pub(super) alphabet: Alphabet,
    pub(crate) dump: Dump,
}

impl Parse for TokenInput {
    #[inline(always)]
    fn parse(input: ParseStream) -> Result<Self> {
        let derive_input = input.parse::<DeriveInput>()?;

        Self::try_from(derive_input)
    }
}

impl TryFrom<DeriveInput> for TokenInput {
    type Error = Error;

    fn try_from(input: DeriveInput) -> Result<Self> {
        let start = Instant::now();

        let ident = input.ident;
        let generics = input.generics;

        let data = match input.data {
            Data::Enum(data) => data,

            other => {
                let span = match other {
                    Data::Struct(data) => data.struct_token.span,
                    Data::Union(data) => data.union_token.span,
                    _ => system_panic!("Unsupported Item format."),
                };

                return Err(error!(
                    span,
                    "Token must be derived from the enum type with \
                    variants representing lexical rules.",
                ));
            }
        };

        let mut variants = data
            .variants
            .into_iter()
            .map(TokenVariant::try_from)
            .collect::<Result<Variants>>()?;

        let mut variant_map = VariantMap::with_capacity(variants.len());

        for (enumerate, variant) in variants.iter().enumerate() {
            let _ = variant_map.insert(variant.ident.clone(), enumerate);
        }

        let mut inline_map = InlineMap::empty();
        let mut lookback = None;
        let mut opt = None;
        let mut dump = Dump::None;
        let mut repr = false;

        for attr in input.attrs {
            match attr.style {
                AttrStyle::Inner(_) => continue,
                AttrStyle::Outer => (),
            }

            let name = match attr.meta.path().get_ident() {
                Some(ident) => ident,
                None => continue,
            };

            let span = attr.span();

            match name.to_string().as_str() {
                "repr" => {
                    attr.parse_args_with(|input: ParseStream| {
                        let repr_kind = input.parse::<Ident>()?;

                        if repr_kind != "u8" {
                            return Err(error!(
                                repr_kind.span(),
                                "Token type must be #[repr(u8)].",
                            ));
                        }

                        if !input.is_empty() {
                            return Err(error!(input.span(), "Token type must be #[repr(u8)].",));
                        }

                        Ok(())
                    })?;

                    repr = true;
                }

                "define" => {
                    let (name, mut regex) = attr.parse_args_with(|input: ParseStream| {
                        let name = input.parse::<Ident>()?;

                        let _ = input.parse::<Token![=]>()?;

                        let expression = input.parse::<Regex>()?;

                        Ok((name, expression))
                    })?;

                    if variant_map.contains_key(&name) {
                        return Err(error!(
                            name.span(),
                            "Enum variant with this name already exists.",
                        ));
                    }

                    if inline_map.contains_key(&name) {
                        return Err(error!(
                            name.span(),
                            "Inline expression with this name already exists.",
                        ));
                    }

                    regex.transform(&TransformConfig::default());
                    regex.inline(&inline_map, &variant_map)?;

                    let _ = inline_map.insert(name, regex);
                }

                "lookback" => {
                    if lookback.is_some() {
                        return Err(error!(span, "Duplicate Lookback attribute.",));
                    }

                    lookback = Some(attr.parse_args::<Expr>()?);
                }

                "opt" => {
                    if opt.is_some() {
                        return Err(error!(span, "Duplicate Opt attribute.",));
                    }

                    opt = Some(attr.parse_args::<Opt>()?);
                }

                "dump" => {
                    if dump.span().is_some() {
                        return Err(error!(span, "Duplicate Dump attribute.",));
                    }

                    dump = Dump::try_from(attr)?;
                }

                _ => continue,
            }
        }

        let opt = opt.unwrap_or_default();

        if !repr {
            return Err(error!(
                ident.span(),
                "Token type must represent u8 type.\nAnnotate this enum type \
                with #[repr(u8)].",
            ));
        }

        match dump {
            Dump::Trivia(span) => {
                return Err(error!(
                    span,
                    "Lexical grammar does not have trivia expressions.",
                ));
            }

            _ => (),
        }

        let mut eoi = None;
        let mut mismatch = None;
        let mut parsable = 0;
        let mut alphabet = Alphabet::empty();

        for variant in &mut variants {
            if let Some(index) = variant.index {
                if index == EOI {
                    eoi = Some(variant.ident.clone());
                }

                if index == MISMATCH {
                    mismatch = Some(variant.ident.clone());
                }
            }

            if let Some((_, rule)) = &mut variant.rule {
                parsable += 1;
                rule.transform(&TransformConfig::default());
                rule.inline(&inline_map, &variant_map)?;
                alphabet.append(rule.alphabet());
            }
        }

        let eoi = match eoi {
            Some(ident) => ident,

            None => {
                return Err(error!(
                    ident.span(),
                    "End-of-Input variant is missing.\nOne of the enum \
                    variants without #[rule(...)] annotation must have \
                    explicit discriminant equal to {EOI}: \"EOI = \
                    {EOI},\"\nThis variant will serve as a source code end of \
                    input marker.",
                ));
            }
        };

        let mismatch = match mismatch {
            Some(ident) => ident,

            None => {
                return Err(error!(
                    ident.span(),
                    "Mismatch fallback variant is missing.\nOne of the enum \
                    variants without #[rule(...)] annotation must have \
                    explicit discriminant equal to {MISMATCH}: \"Mismatch = \
                    {MISMATCH},\"\nAll character sequences not covered by the \
                    lexis grammar will sink into this token variant.",
                ));
            }
        };

        if parsable == 0 {
            return Err(error!(
                ident.span(),
                "At least one of the enum variants must represent a parsable \
                rule.\nAssociate one of the variants with #[rule(...)] \
                attribute to make it parsable.",
            ));
        }

        let mut scope = Scope::new();

        for variant in &mut variants {
            let start = Instant::now();

            let (span, regex) = match &mut variant.rule {
                None => continue,
                Some((span, regex)) => {
                    regex.expand(&alphabet);

                    (*span, regex)
                }
            };

            let index = expect_some!(
                variant_map.get(&variant.ident).copied(),
                "Missing product index.",
            ) as u8;

            let automata = {
                scope.set_strategy(Strategy::CANONICALIZE);
                let rule = regex.encode(&mut scope)?;

                if rule.accepts_null() {
                    return Err(error!(span, "This rule expression accepts empty string.",));
                }

                let product = scope.terminal(Set::new([Terminal::Product(index)]));

                scope.set_strategy(Strategy::DETERMINIZE);
                scope.concatenate(rule, product)
            };

            variant.time += start.elapsed();

            variant.automata = Some(automata);
        }

        let mut automata = {
            scope.set_strategy(Strategy::DETERMINIZE);

            let mut ordered = variants
                .iter_mut()
                .filter_map(|variant| match variant.automata.is_some() {
                    false => None,
                    true => Some(variant),
                })
                .collect::<Vec<_>>();

            ordered.sort_by(|a, b| match a.priority.cmp(&b.priority) {
                Ordering::Less => Ordering::Greater,
                Ordering::Greater => Ordering::Less,
                Ordering::Equal => {
                    let a_automata = expect_some!(a.automata.as_ref(), "Automata is missing.",);
                    let b_automata = expect_some!(b.automata.as_ref(), "Automata is missing.",);

                    a_automata
                        .transitions()
                        .len()
                        .cmp(&b_automata.transitions().len())
                }
            });

            let automata = ordered.into_iter().fold(None, |acc, next| {
                let automata = expect_some!(take(&mut next.automata), "Automata is missing",);

                match acc {
                    None => Some(automata),
                    Some(acc) => {
                        let start = Instant::now();
                        let result = Some(scope.union(acc, automata));
                        next.time += start.elapsed();
                        result
                    }
                }
            });

            expect_some!(automata, "Empty automata.",)
        };

        automata.merge(&mut scope, &variants)?;

        automata.check_property_conflicts(ident.span())?;

        scope.set_strategy(opt.into_strategy());
        scope.optimize(&mut automata);

        scope.reset();
        automata = scope.copy(&automata);

        let products = automata.filter_out(&variants)?;

        let analysis = start.elapsed();

        let mut result = Self {
            ident,
            generics,
            lookback,
            eoi,
            mismatch,
            automata,
            variants,
            products,
            alphabet,
            dump,
        };

        if let Dump::Meta(span) = result.dump {
            let start = Instant::now();
            let output = result.to_token_stream();
            let build = start.elapsed();

            let output_string = match parse2::<File>(output.clone()) {
                Ok(file) => ::prettyplease::unparse(&file),
                Err(_) => output.to_string(),
            };

            let lines = output_string.lines().count();
            let transitions = result.automata.transitions().len();
            let states = result
                .automata
                .transitions()
                .view()
                .iter()
                .map(|(from, to)| to.iter().map(|(_, to)| [*from, *to].into_iter()))
                .flatten()
                .flatten()
                .collect::<Set<State>>()
                .len();

            let ident = &result.ident;

            let mut report = String::with_capacity(1024 * 10);

            report += " -- Macro Debug Dump --\n\n";
            report += &format!("Token \"{ident}\" analysis:\n");
            report += &format!("    Optimization: {opt:?}.\n");
            report += &format!("    Total analysis time: {analysis:?}.\n");
            report += &format!("    Total code generation time: {build:?}.\n");
            report += &format!("    States count: {states}.\n");
            report += &format!("    Transitions count: {transitions}.\n");
            report += &format!("    Lines of code: {lines}.\n");

            report += "\nRules analysis:\n";

            result.variants.sort_by_key(|automata| automata.time);

            let align = result
                .variants
                .iter()
                .filter_map(|variant| {
                    if variant.rule.is_none() {
                        return None;
                    }

                    Some(variant.ident.to_string().len())
                })
                .max()
                .unwrap_or(0)
                + 1;

            for variant in result.variants.iter().rev() {
                if variant.rule.is_none() {
                    continue;
                }

                let ident = &variant.ident;
                let time = variant.time;

                report += &format!("    {ident: <align$} {time:?}.\n");
            }

            return Err(error!(span, "{report}",));
        }

        if let Dump::Output(span) = result.dump {
            let output = result.to_token_stream();

            let output_string = match parse2::<File>(output.clone()) {
                Ok(file) => ::prettyplease::unparse(&file),
                Err(_) => output.to_string(),
            };

            let ident = &result.ident;

            return Err(error!(
                span,
                " -- Macro Debug Dump --\n\nToken \"{ident}\" \
                implementation code:\n\n{output_string}",
            ));
        }

        Ok(result)
    }
}
