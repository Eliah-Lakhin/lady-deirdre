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

use std::{mem::take, time::Instant};

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
    File,
    Result,
    Type,
    Visibility,
};

use crate::{
    node::{
        automata::{NodeAutomataImpl, Scope},
        generics::ParserGenerics,
        globals::Globals,
        index::Index,
        recovery::Recovery,
        regex::{Regex, RegexImpl},
        rule::Rule,
        token::TokenLit,
        variant::{NodeVariant, VariantTrivia},
    },
    utils::{error, expect_some, system_panic, Dump, Map, PredictableCollection, Set, SetImpl},
};

pub(super) type VariantMap = Map<Ident, NodeVariant>;

pub struct NodeInput {
    pub(super) ident: Ident,
    pub(super) vis: Visibility,
    pub(super) generics: ParserGenerics,
    pub(super) token: Type,
    pub(super) classifier: Option<Type>,
    pub(super) error: Type,
    pub(super) trivia: Option<Rule>,
    pub(super) recovery: Option<Recovery>,
    pub(crate) dump: Dump,
    pub(super) variants: VariantMap,
    pub(super) alphabet: Set<TokenLit>,
}

impl Parse for NodeInput {
    #[inline(always)]
    fn parse(input: ParseStream) -> Result<Self> {
        let derive_input = input.parse::<DeriveInput>()?;

        Self::try_from(derive_input)
    }
}

impl TryFrom<DeriveInput> for NodeInput {
    type Error = Error;

    fn try_from(input: DeriveInput) -> Result<Self> {
        let start = Instant::now();

        let ident = input.ident;
        let vis = input.vis;
        let generics = ParserGenerics::new(input.generics);

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
                    "Node must be derived from the enum type with \
                    variants representing syntax variants.",
                ));
            }
        };

        let mut variants = data
            .variants
            .into_iter()
            .map(|variant| {
                let variant = NodeVariant::try_from(variant)?;

                Ok((variant.ident.clone(), variant))
            })
            .collect::<Result<Map<Ident, NodeVariant>>>()?;

        let mut inlines = Map::empty();

        let mut token = None;
        let mut classifier = None;
        let mut error = None;
        let mut trivia = None;
        let mut recovery = None;
        let mut dump = Dump::None;

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
                "token" => {
                    if token.is_some() {
                        return Err(error!(span, "Duplicate Token attribute.",));
                    }

                    token = Some(attr.parse_args::<Type>()?);
                }

                "classifier" => {
                    if classifier.is_some() {
                        return Err(error!(span, "Duplicate Classifier attribute.",));
                    }

                    classifier = Some(attr.parse_args::<Type>()?);
                }

                "error" => {
                    if error.is_some() {
                        return Err(error!(span, "Duplicate Token attribute.",));
                    }

                    error = Some(attr.parse_args::<Type>()?);
                }

                "trivia" => {
                    if trivia.is_some() {
                        return Err(error!(span, "Duplicate Trivia attribute.",));
                    }

                    trivia = Some(Rule::try_from(attr)?.zero_or_more());
                }

                "recovery" => {
                    if recovery.is_some() {
                        return Err(error!(span, "Duplicate Recovery attribute.",));
                    }

                    recovery = Some({
                        let recovery = attr.parse_args::<Recovery>()?;

                        if recovery.is_empty() {
                            return Err(error!(
                                recovery.span(),
                                "Explicitly specified Recovery cannot be empty.",
                            ));
                        }

                        recovery
                    });
                }

                "define" => {
                    let (name, mut regex) = attr.parse_args_with(|input: ParseStream| {
                        let name = input.parse::<Ident>()?;

                        let _ = input.parse::<Token![=]>()?;

                        let expression = input.parse::<Regex>()?;

                        Ok((name, expression))
                    })?;

                    if variants.contains_key(&name) {
                        return Err(error!(
                            name.span(),
                            "Enum variant with this name already exists.",
                        ));
                    }

                    if inlines.contains_key(&name) {
                        return Err(error!(
                            name.span(),
                            "Inline expression with this name already exists.",
                        ));
                    }

                    regex.inline(&inlines)?;

                    let _ = inlines.insert(name, regex);
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

        let token = match token {
            Some(ty) => ty,

            None => {
                return Err(error!(
                    ident.span(),
                    "Token type was not specified.\nUse #[token(<token type>)] \
                    attribute on the derived type to specify Token type.",
                ));
            }
        };

        let error = match error {
            Some(ty) => ty,

            None => {
                return Err(error!(
                    ident.span(),
                    "Error type was not specified.\nUse #[error(<error type>)] \
                    attribute on the derived type to specify Error type.",
                ));
            }
        };

        let mut root = None;
        let mut alphabet = Set::empty();

        for (_, variant) in &mut variants {
            if let Some(span) = variant.root {
                if root.is_some() {
                    return Err(error!(
                        span,
                        "Duplicate Root rule.\nThe syntax must specify only \
                        one Root rule.",
                    ));
                }

                root = Some(variant.ident.clone());
            }

            if let Some(rule) = &mut variant.rule {
                rule.regex.inline(&inlines)?;
                alphabet = alphabet.merge(rule.regex.alphabet());
            }

            if let Some(trivia) = variant.trivia.rule_mut() {
                trivia.regex.inline(&inlines)?;
                alphabet = alphabet.merge(trivia.regex.alphabet());
            }
        }

        if root.is_none() {
            return Err(error!(
                ident.span(),
                "Node syntax must specify a Root rule.\nAnnotate one of the \
                enum variants with #[root] attribute.",
            ));
        };

        let mut scope = Scope::default();

        if let Some(trivia) = &mut trivia {
            trivia.regex.inline(&inlines)?;
            alphabet = alphabet.merge(trivia.regex.alphabet());
            trivia.regex.expand(&alphabet);
            trivia.encode(&mut scope)?;
        }

        let mut indices = Set::empty();
        let mut index_map = Map::empty();
        let mut pending = Vec::with_capacity(variants.len() * 2 + 1);

        for (_, variant) in &mut variants {
            if let Some(index) = &variant.index {
                if let Some(previous) = index_map.insert(index.key(), variant.ident.clone()) {
                    return Err(error!(
                        index.span(),
                        "Rule \"{previous}\" has the same index.\nRule indices \
                        must be unique.",
                    ));
                }

                match index {
                    Index::Generated(_, index)
                    | Index::Overridden(_, index)
                    | Index::Named(_, Some(index)) => {
                        let _ = indices.insert(*index);
                    }

                    _ => (),
                }

                let _ = pending.push(variant.ident.clone());
            }

            match &mut variant.rule {
                None => continue,
                Some(rule) => {
                    rule.regex.expand(&alphabet);
                    rule.encode(&mut scope)?;

                    if let Some(constructor) = &variant.constructor {
                        let variables =
                            expect_some!(rule.variables.as_ref(), "Missing variable map.",);

                        constructor.fits(variables)?;
                    }
                }
            }

            if let Some(trivia) = variant.trivia.rule_mut() {
                trivia.regex.expand(&alphabet);
                trivia.encode(&mut scope)?;
            }
        }

        if let Some(trivia) = &trivia {
            for reference in trivia.regex.refs(true, &variants)? {
                pending.push(reference);
            }
        }

        let mut visited = Set::empty();
        let mut next_index = 1;

        while let Some(ident) = pending.pop() {
            if visited.contains(&ident) {
                continue;
            }

            let variant = expect_some!(variants.get(&ident), "Missing variant.",);

            if let Some(rule) = variant.rule.as_ref() {
                for reference in rule.regex.refs(false, &variants)? {
                    pending.push(reference);
                }

                if let Some(trivia) = variant.trivia.rule() {
                    for reference in trivia.regex.refs(true, &variants)? {
                        pending.push(reference);
                    }
                }
            }

            match &variant.index {
                None => (),

                Some(Index::Named(name, index)) => {
                    if variants.contains_key(name) {
                        return Err(error!(
                            name.span(),
                            "This index name already used as a Variant \
                            name.\nIndex names must be unique in the \
                            type namespace.",
                        ));
                    }

                    if index.is_some() {
                        let _ = visited.insert(ident);
                        continue;
                    }
                }

                _ => {
                    let _ = visited.insert(ident);
                    continue;
                }
            }

            while !indices.insert(next_index) {
                next_index += 1;
            }

            let variant = expect_some!(variants.get_mut(&ident), "Missing variant.",);

            match &mut variant.index {
                Some(Index::Named(_, index @ None)) => *index = Some(next_index),

                index @ None => *index = Some(Index::Generated(ident.span(), next_index)),

                _ => system_panic!("Inconsistent indices."),
            }

            let _ = visited.insert(ident);
        }

        for (_, variant) in &variants {
            if variant.root.is_some() {
                continue;
            }

            let rule = match &variant.rule {
                None => continue,
                Some(rule) => rule,
            };

            if !visited.contains(&variant.ident) {
                return Err(error!(
                    rule.span,
                    "This rule is abandoned.\n\nEach parsable rule except the \
                    Root rule (annotated with #[root]), trivia \
                    expressions,\nand the rules with Overridden index \
                    (annotated with #[index(...)]) must be referred\ndirectly or \
                    indirectly from the Root rule, or trivia expressions.\n\n\
                    If this is intending (e.g. if you want to descend into \
                    this rule manually),\nconsider annotating this rule with \
                    #[index(<number>)] attribute.\nLater on you can \
                    descend into this rule using that index number.",
                ));
            }

            let leftmost = expect_some!(rule.leftmost.as_ref(), "Missing leftmost",);

            if leftmost.is_optional() {
                return Err(error!(
                    rule.span,
                    "This rule can match empty token sequence.\nOnly the Root \
                    rule or trivia expressions allowed to match empty \
                    sequences.",
                ));
            }

            let mut trace = Vec::with_capacity(variants.len());
            trace.push(&variant.ident);

            if leftmost.is_self_recursive(&variants, &mut trace) {
                return match trace.len() > 2 {
                    false => Err(error!(
                        rule.span,
                        "This rule is self-recursive in its leftmost \
                        position.\nLeft recursion forbidden.",
                    )),

                    true => {
                        let trace = trace
                            .into_iter()
                            .map(|ident| ident.to_string())
                            .collect::<Vec<_>>()
                            .join(" \u{2192} ");

                        Err(error!(
                            rule.span,
                            "This rule is indirectly self-recursive in its \
                            leftmost position.\nRecursion trace: \
                            {trace}\nLeft recursion forbidden.",
                        ))
                    }
                };
            }
        }

        for ident in variants.keys().cloned().collect::<Vec<_>>() {
            let variant = expect_some!(variants.get(&ident), "Missing variant.",);

            let rule = match &variant.rule {
                Some(rule) => rule,
                None => continue,
            };

            let leftmost = expect_some!(rule.leftmost.as_ref(), "Missing leftmost.",);

            if leftmost.matches().is_some() {
                continue;
            }

            let variant = expect_some!(variants.get_mut(&ident), "Missing variant.",);
            let rule = expect_some!(variant.rule.as_mut(), "Missing rule.",);
            let mut leftmost = expect_some!(take(&mut rule.leftmost), "Missing leftmost.",);
            leftmost.resolve_matches(&mut variants);

            let variant = expect_some!(variants.get_mut(&ident), "Missing variant.",);
            let rule = expect_some!(variant.rule.as_mut(), "Missing rule.",);
            rule.leftmost = Some(leftmost);
        }

        for (_, variant) in &variants {
            if variant.root.is_some() {
                continue;
            }

            let rule = match &variant.rule {
                None => continue,
                Some(rule) => rule,
            };

            let automata = expect_some!(rule.automata.as_ref(), "Missing automata",);

            let trivia = match &variant.trivia {
                VariantTrivia::Inherited => trivia.as_ref(),
                VariantTrivia::Empty(..) => None,
                VariantTrivia::Rule(rule) => Some(rule),
            };

            automata.check_conflicts(trivia, &variants)?;
        }

        let parent_required = variants
            .values()
            .any(|variant| variant.index.is_some() && variant.inheritance.has_parent());

        let node_required = variants
            .values()
            .any(|variant| variant.index.is_some() && variant.inheritance.has_node());

        let semantics_required = variants
            .values()
            .any(|variant| variant.index.is_some() && variant.inheritance.has_semantics());

        for (_, variant) in &variants {
            if variant.index.is_none() {
                continue;
            }

            if parent_required && !variant.inheritance.has_parent() {
                return Err(error!(
                    variant.ident.span(),
                    "Missing parent node reference. Introduce a field with #[parent] annotation.",
                ));
            }

            if node_required && !variant.inheritance.has_node() {
                return Err(error!(
                    variant.ident.span(),
                    "Missing self node reference. Introduce a field with #[node] annotation.",
                ));
            }

            if semantics_required && !variant.inheritance.has_semantics() {
                return Err(error!(
                    variant.ident.span(),
                    "Missing semantics field. Introduce a field with #[semantics] annotation.",
                ));
            }
        }

        let analysis = start.elapsed();

        let result = Self {
            ident,
            vis,
            generics,
            token,
            classifier,
            error,
            trivia,
            recovery,
            dump,
            variants,
            alphabet,
        };

        if let Dump::Meta(span) = dump {
            let start = Instant::now();
            let output = result.to_token_stream();
            let build = start.elapsed();

            let output_string = match parse2::<File>(output.clone()) {
                Ok(file) => prettyplease::unparse(&file),
                Err(_) => output.to_string(),
            };

            let lines = output_string.lines().count();

            let ident = &result.ident;

            return Err(error!(
                span,
                " -- Macro System Debug Dump --\n\nNode \"{ident}\" \
                metadata:\nAnalysis time: {analysis:?}.\nCode generation \
                time: {build:?}.\nLines of code: {lines}.\n",
            ));
        }

        if let Dump::Trivia(span) = dump {
            let trivia = match &result.trivia {
                Some(trivia) => trivia,
                None => {
                    return Err(error!(
                        span,
                        "Trivia dump is not applicable here, because global \
                        Trivia expression is not specified.\nUse \
                        #[trivia(...)] attribute to specify trivia expression.",
                    ))
                }
            };

            let mut globals = Globals::default();

            let output = result.compile_skip_fn(
                &mut globals,
                trivia,
                &Index::Generated(span, 0),
                true,
                true,
                false,
            );

            let output_string = match parse2::<File>(output.clone()) {
                Ok(file) => prettyplease::unparse(&file),
                Err(_) => output.to_string(),
            };

            let node = &result.ident;

            return Err(error!(
                span,
                " -- Macro System Debug Dump --\n\nNode \"{node}\" global \
                trivia parser function is:\n\n{output_string}",
            ));
        }

        if let Dump::Output(span) = dump {
            let output = result.to_token_stream();

            let output_string = match parse2::<File>(output.clone()) {
                Ok(file) => prettyplease::unparse(&file),
                Err(_) => output.to_string(),
            };

            let ident = &result.ident;

            return Err(error!(
                span,
                " -- Macro System Debug Dump --\n\nNode \"{ident}\" \
                implementation code:\n\n{output_string}",
            ));
        }

        for (ident, variant) in &result.variants {
            match variant.dump {
                Dump::Trivia(span) => {
                    let trivia = expect_some!(variant.trivia.rule(), "Missing trivia rule.",);
                    let context = expect_some!(variant.index.as_ref(), "Missing rule index.",);

                    let mut globals = Globals::default();

                    let output =
                        result.compile_skip_fn(&mut globals, trivia, context, true, true, false);

                    let output_string = match parse2::<File>(output.clone()) {
                        Ok(file) => prettyplease::unparse(&file),
                        Err(_) => output.to_string(),
                    };

                    let node = &result.ident;

                    return Err(error!(
                        span,
                        " -- Macro System Debug Dump --\n\nRule \
                        \"{node}::{ident}\" trivia parser function \
                        is:\n\n{output_string}",
                    ));
                }

                Dump::Output(span) => {
                    let mut globals = Globals::default();

                    let output = expect_some!(
                        variant.compile_parser_fn(&result, &mut globals, false, true, true, false),
                        "Parser function generation failure.",
                    );

                    let output_string = match parse2::<File>(output.clone()) {
                        Ok(file) => prettyplease::unparse(&file),
                        Err(_) => output.to_string(),
                    };

                    let node = &result.ident;

                    return Err(error!(
                        span,
                        " -- Macro System Debug Dump --\n\nRule \
                        \"{node}::{ident}\" parser function \
                        is:\n\n{output_string}",
                    ));
                }

                _ => (),
            }
        }

        Ok(result)
    }
}
