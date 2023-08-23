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

use std::{cmp::Ordering, mem::take, time::Instant};

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse2,
    spanned::Spanned,
    AttrStyle,
    Data,
    DeriveInput,
    Error,
    File,
    Generics,
    LitStr,
    Result,
};

use crate::{
    token::{
        automata::{AutomataImpl, Scope, Terminal, TokenAutomata},
        opt::Opt,
        output::Output,
        regex::{Regex, RegexImpl},
        variant::{TokenVariant, EOI, MISMATCH},
    },
    utils::{
        error,
        expect_some,
        system_panic,
        AutomataContext,
        Dump,
        Facade,
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
    generics: Generics,
    eoi: Ident,
    mismatch: Ident,
    pub(super) automata: TokenAutomata,
    pub(super) variants: Variants,
    pub(super) products: ProductMap,
    pub(super) alphabet: Alphabet,
    pub(super) dump: Dump,
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
                    "Token must be derived on the enum type with \
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

                    regex.inline(&inline_map, &variant_map)?;

                    let _ = inline_map.insert(name, regex);
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

        scope.set_strategy(opt.into_strategy());
        scope.optimize(&mut automata);

        scope.reset();
        automata = scope.copy(&automata);

        let products = automata.filter_out(&variants)?;

        let analysis = start.elapsed();

        let mut result = Self {
            ident,
            generics,
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

            report += " -- Macro System Debug Dump --\n\n";
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
                " -- Macro System Debug Dump --\n\nToken \"{ident}\" \
                implementation code:\n\n{output_string}",
            ));
        }

        Ok(result)
    }
}

impl TokenInput {
    fn compile_parse_fn(&self) -> TokenStream {
        let span = self.ident.span();
        let core = span.face_core();
        let panic = span.face_panic();

        let mismatch = &self.mismatch;
        let start = self.automata.start();

        let buffer = match self
            .variants
            .iter()
            .any(|variant| variant.constructor.is_some())
        {
            false => None,
            true => {
                let string = span.face_string();

                Some(quote_spanned!(span =>
                    #[allow(unused_mut)]
                    let mut buffer = #string::new();
                ))
            }
        };

        let transitions = Output::compile(self, buffer.is_some());

        quote_spanned!(span=>
            fn parse(session: &mut impl #core::lexis::LexisSession) -> Self {
                #[allow(unused_mut)]
                let mut state = #start;

                #[allow(unused_mut)]
                let mut token = Self::#mismatch;

                #buffer

                loop {
                    let byte = #core::lexis::LexisSession::advance(session);

                    if byte == 0xFF {
                        break;
                    }

                    match state {
                        #(
                        #transitions
                        )*

                        #[cfg(not(debug_assertions))]
                        _ => (),

                        #[cfg(debug_assertions)]
                        state => #panic("Invalid state {state}."),
                    }
                }

                token
            }
        )
    }

    fn compile_eoi_fn(&self) -> TokenStream {
        let eoi = &self.eoi;
        let span = eoi.span();

        quote_spanned!(span=>
            #[inline(always)]
            fn eoi() -> Self {
                Self::#eoi
            }
        )
    }

    fn compile_mismatch_fn(&self) -> TokenStream {
        let mismatch = &self.mismatch;
        let span = mismatch.span();

        quote_spanned!(span=>
            #[inline(always)]
            fn mismatch() -> Self {
                Self::#mismatch
            }
        )
    }

    fn compile_rule_fn(&self) -> TokenStream {
        let span = self.ident.span();
        let core = span.face_core();

        quote_spanned!(span=>
            #[inline(always)]
            fn rule(self) -> #core::lexis::TokenRule {
                self as u8
            }
        )
    }

    fn compile_name_fn(&self) -> TokenStream {
        let span = self.ident.span();
        let core = span.face_core();
        let option = span.face_option();

        let names = self.variants.iter().map(|variant| {
            let ident = &variant.ident;
            let span = ident.span();
            let option = span.face_option();
            let name = LitStr::new(ident.to_string().as_str(), span);

            quote_spanned!(span=>
                if Self::#ident as u8 == rule {
                    return #option::Some(#name);
                }
            )
        });

        quote_spanned!(span=>
            #[inline(always)]
            fn name(rule: #core::lexis::TokenRule) -> #option<&'static str> {
                #(#names)*

                None
            }
        )
    }

    fn compile_description_fn(&self) -> TokenStream {
        let span = self.ident.span();
        let core = span.face_core();
        let option = span.face_option();

        let descriptions = self.variants.iter().map(|variant| {
            let ident = &variant.ident;
            let span = ident.span();
            let option = span.face_option();
            let description = &variant.description;

            quote_spanned!(span=>
                if Self::#ident as u8 == rule {
                    return #option::Some(#description);
                }
            )
        });

        quote_spanned!(span=>
            #[inline(always)]
            fn describe(rule: #core::lexis::TokenRule) -> #option<&'static str> {
                #(#descriptions)*

                None
            }
        )
    }
}

impl ToTokens for TokenInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Dump::Dry(..) = self.dump {
            return;
        }

        let ident = &self.ident;
        let span = ident.span();
        let core = span.face_core();

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let parse = self.compile_parse_fn();
        let eoi = self.compile_eoi_fn();
        let mismatch = self.compile_mismatch_fn();
        let rule = self.compile_rule_fn();
        let name = self.compile_name_fn();
        let description = self.compile_description_fn();

        quote_spanned!(span=>
            impl #impl_generics #core::lexis::Token for #ident #ty_generics
            #where_clause
            {
                #parse
                #eoi
                #mismatch
                #rule
                #name
                #description
            }
        )
        .to_tokens(tokens)
    }
}
