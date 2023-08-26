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

use proc_macro2::{Ident, Span, TokenStream};
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
    LitStr,
    Result,
    Type,
    Visibility,
};

use crate::{
    node::{
        automata::{NodeAutomataImpl, Scope},
        generics::ParserGenerics,
        globals::{GlobalVar, Globals},
        index::Index,
        recovery::Recovery,
        regex::{Regex, RegexImpl},
        rule::Rule,
        token::TokenLit,
        variant::{NodeVariant, VariantTrivia},
    },
    utils::{
        error,
        expect_some,
        system_panic,
        Dump,
        Facade,
        Map,
        PredictableCollection,
        Set,
        SetImpl,
    },
};

pub(super) type VariantMap = Map<Ident, NodeVariant>;

pub struct NodeInput {
    pub(super) ident: Ident,
    pub(super) vis: Visibility,
    pub(super) generics: ParserGenerics,
    pub(super) token: Type,
    pub(super) error: Type,
    pub(super) trivia: Option<Rule>,
    pub(super) recovery: Option<Recovery>,
    pub(super) dump: Dump,
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

                return Err(Error::new(
                    span,
                    "Node must be derived on the enum type with \
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
                    "Token Type is not specified.\nUse #[token(<type name>)] \
                    attribute on the derived type to specify Token type.",
                ));
            }
        };

        let error = match error {
            Some(ty) => ty,

            None => {
                return Err(error!(
                    ident.span(),
                    "Error Type is not specified.\nUse #[error(<error name>)] \
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

        let analysis = start.elapsed();

        let result = Self {
            ident,
            vis,
            generics,
            token,
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
                Ok(file) => ::prettyplease::unparse(&file),
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
                Ok(file) => ::prettyplease::unparse(&file),
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
                Ok(file) => ::prettyplease::unparse(&file),
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
                        Ok(file) => ::prettyplease::unparse(&file),
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
                        Ok(file) => ::prettyplease::unparse(&file),
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

impl ToTokens for NodeInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Dump::Dry(..) = self.dump {
            return;
        }

        let output_comments = match self.dump {
            Dump::None => false,
            Dump::Dry(..) => return,
            _ => true,
        };

        let ident = &self.ident;
        let vis = &self.vis;
        let span = ident.span();
        let core = span.face_core();
        let option = span.face_option();
        let unimplemented = span.face_unimplemented();

        let (impl_generics, type_generics, where_clause) = self.generics.ty.split_for_impl();
        let code = &self.generics.code;

        let token = &self.token;
        let error = &self.error;

        let mut globals = Globals::default();

        let trivia = match &self.trivia {
            None => None,
            Some(trivia) => Some(self.compile_skip_fn(
                &mut globals,
                trivia,
                &Index::Generated(span, 0),
                false,
                output_comments,
                true,
            )),
        };

        let capacity = self.variants.len();

        let mut indices = Vec::with_capacity(capacity);
        let mut functions = Vec::with_capacity(capacity);
        let mut cases = Vec::with_capacity(capacity);
        let mut node_getters = Vec::with_capacity(capacity);
        let mut parent_getters = Vec::with_capacity(capacity);
        let mut parent_setters = Vec::with_capacity(capacity);
        let mut children_getters = Vec::with_capacity(capacity);

        let mut by_index = self
            .variants
            .values()
            .map(|variant| match &variant.index {
                None => None,
                Some(index) => Some((index.get(), variant.ident.clone())),
            })
            .flatten()
            .collect::<Vec<_>>();

        by_index.sort_by_key(|(index, _)| *index);

        for (_, ident) in by_index {
            let variant = match self.variants.get(&ident) {
                None => continue,
                Some(variant) => variant,
            };

            node_getters.push(variant.inheritance.compile_node_getter());
            parent_getters.push(variant.inheritance.compile_parent_getter());
            parent_setters.push(variant.inheritance.compile_parent_setter());
            children_getters.push(variant.inheritance.compile_children_getter());

            if let Some(Index::Named(name, Some(index))) = &variant.index {
                let span = name.span();
                let core = span.face_core();
                indices.push(quote_spanned!(span=>
                    #vis const #name: #core::syntax::NodeRule = #index;
                ))
            }

            if variant.rule.is_none() {
                continue;
            }

            let index = expect_some!(variant.index.as_ref(), "Parsable rule without index.",);

            if let Some(parser) = &variant.parser {
                let span = parser.span();

                cases.push(quote_spanned!(span=> #index => Self::#parser(session),));
                continue;
            }

            let function = expect_some!(
                variant.compile_parser_fn(self, &mut globals, true, false, output_comments, true,),
                "Parsable non-overridden rule without generated parser.",
            );

            functions.push(function);

            let ident = variant.generated_parser_ident();

            cases.push(quote_spanned!(span=> #index => #ident(session),));
        }

        let mut descriptions = self
            .variants
            .values()
            .map(|variant| {
                let description = match &variant.description {
                    None => return None,
                    Some(description) => description,
                };

                let index = expect_some!(variant.index.as_ref(), "Description without index",);

                Some((index, &variant.ident, description))
            })
            .flatten()
            .collect::<Vec<_>>();

        descriptions.sort_by_key(|(index, _, _)| index.get());

        let get_rule = descriptions
            .iter()
            .map(|(index, ident, _)| quote_spanned!(index.span() => Self::#ident { .. } => #index,))
            .collect::<Vec<_>>();

        let (get_name, get_description): (Vec<_>, Vec<_>) = descriptions
            .into_iter()
            .map(|(index, ident, description)| {
                let name = LitStr::new(&ident.to_string(), ident.span());

                (
                    quote_spanned!(index.span() => #index => #option::Some(#name),),
                    quote_spanned!(index.span() => #index => #option::Some(#description),),
                )
            })
            .unzip();

        let globals = globals.compile(span, &self.token);

        let checks = self
            .alphabet
            .iter()
            .map(|lit| {
                let name = match lit {
                    TokenLit::Ident(ident) => ident,
                    _ => return None,
                };

                let span = name.span();
                let core = span.face_core();
                let panic = span.face_panic();

                Some(quote_spanned!(span=>
                    if #token::#name as u8 == #core::lexis::EOI {
                        #panic("EOI token cannot be used explicitly.");
                    }
                ))
            })
            .flatten()
            .collect::<Vec<_>>();

        let indices = match indices.is_empty() {
            true => None,
            false => Some(quote_spanned!(span=>
                impl #ident #type_generics #where_clause {
                #(
                    #indices
                )*
                }
            )),
        };

        let checks = match !checks.is_empty() && cfg!(debug_assertions) {
            false => None,

            true => Some(quote_spanned!(span=>
                #[cfg(debug_assertions)]
                #[allow(dead_code)]
                const CHECK_EOI: () = {
                    #( #checks )*

                    ()
                };
            )),
        };

        quote_spanned!(span=>
            impl #impl_generics #core::syntax::Node for #ident #type_generics
            #where_clause
            {
                type Token = #token;
                type Error = #error;

                #[inline(always)]
                fn parse<#code>(
                    session: &mut impl #core::syntax::SyntaxSession<#code, Node = Self>,
                    rule: #core::syntax::NodeRule,
                ) -> Self
                {
                    #globals

                    #trivia

                    #checks

                    #( #functions )*

                    match rule {
                        #( #cases )*

                        #[allow(unreachable_patterns)]
                        other => #unimplemented("Unsupported rule {}.", other),
                    }
                }

                #[inline(always)]
                fn rule(&self) -> #core::syntax::NodeRule {
                    match self {
                        #(
                        #get_rule
                        )*

                        #[allow(unreachable_patterns)]
                        _ => #core::syntax::NON_RULE,
                    }
                }

                #[inline(always)]
                fn node_ref(&self) -> #core::syntax::NodeRef {
                    match self {
                        #( #node_getters )*

                        #[allow(unreachable_patterns)]
                        _ => #core::syntax::NodeRef::nil(),
                    }
                }

                #[inline(always)]
                fn parent_ref(&self) -> #core::syntax::NodeRef {
                    match self {
                        #( #parent_getters )*

                        #[allow(unreachable_patterns)]
                        _ => #core::syntax::NodeRef::nil(),
                    }
                }

                #[inline(always)]
                #[allow(unused_variables)]
                fn set_parent_ref(&mut self, parent_ref: #core::syntax::NodeRef) {
                    match self {
                        #( #parent_setters )*

                        #[allow(unreachable_patterns)]
                        _ => (),
                    }
                }

                #[inline(always)]
                fn children(&self) -> #core::syntax::Children {
                    #[allow(unused_mut)]
                    let mut children = #core::syntax::Children::with_capacity(#capacity);

                    match self {
                        #( #children_getters )*

                        #[allow(unreachable_patterns)]
                        _ => (),
                    }

                    children
                }

                #[inline(always)]
                fn name(rule: #core::syntax::NodeRule) -> #option<&'static str> {
                    match rule {
                        #(
                        #get_name
                        )*

                        #[allow(unreachable_patterns)]
                        _ => #option::None,
                    }
                }

                #[inline(always)]
                fn describe(rule: #core::syntax::NodeRule) -> #option<&'static str> {
                    match rule {
                        #(
                        #get_description
                        )*

                        #[allow(unreachable_patterns)]
                        _ => #option::None,
                    }
                }
            }

            #indices
        )
        .to_tokens(tokens)
    }
}

impl NodeInput {
    pub(super) fn this(&self) -> TokenStream {
        let ident = &self.ident;

        match self.generics.ty.params.is_empty() {
            true => ident.to_token_stream(),

            false => {
                let span = ident.span();
                let (_, generics, _) = self.generics.ty.split_for_impl();
                let generics = generics.as_turbofish();

                quote_spanned!(span=> #ident #generics)
            }
        }
    }

    pub(super) fn make_fn(
        &self,
        ident: Ident,
        params: Vec<TokenStream>,
        result: Option<TokenStream>,
        body: TokenStream,
        allow_warnings: bool,
    ) -> (Ident, TokenStream) {
        let span = ident.span();
        let core = span.face_core();
        let (impl_generics, _, where_clause) = self.generics.func.split_for_impl();
        let code = &self.generics.code;
        let this = self.this();

        let allowed_warnings = match allow_warnings {
            true => Some(Self::base_warnings(span)),
            false => None,
        };

        let result = match result {
            Some(ty) => Some(quote_spanned!(span=> -> #ty)),
            None => None,
        };

        (
            ident.clone(),
            quote_spanned!(span=>
                #allowed_warnings
                fn #ident #impl_generics (
                    session: &mut impl #core::syntax::SyntaxSession<#code, Node = #this>,
                    #(
                    #params,
                    )*
                ) #result #where_clause {
                    #body
                }
            ),
        )
    }

    pub(super) fn compile_skip_fn(
        &self,
        globals: &mut Globals,
        trivia: &Rule,
        context: &Index,
        include_globals: bool,
        output_comments: bool,
        allow_warnings: bool,
    ) -> TokenStream {
        let span = trivia.span;
        let body = trivia.compile(
            self,
            globals,
            context,
            &GlobalVar::UnlimitedRecovery,
            false,
            false,
            output_comments,
        );

        let globals = match include_globals {
            false => None,
            true => Some(globals.compile(span, &self.token)),
        };

        self.make_fn(
            format_ident!("skip_trivia", span = span),
            vec![],
            None,
            quote_spanned!(span=> #globals #body),
            allow_warnings,
        )
        .1
    }

    #[inline]
    pub(super) fn base_warnings(span: Span) -> TokenStream {
        quote_spanned!(span=>
            #[allow(unused)]
            #[allow(unused_mut)]
            #[allow(unused_assignments)]
            #[allow(unused_variables)]
            #[allow(non_snake_case)]
        )
    }
}
