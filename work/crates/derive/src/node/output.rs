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

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{spanned::Spanned, LitStr};

use crate::{
    node::{
        globals::{GlobalVar, Globals},
        index::Index,
        rule::Rule,
        token::TokenLit,
        NodeInput,
    },
    utils::{expect_some, Description, Dump, Facade},
};

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
        isolated_session: bool,
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

        let warnings = match allow_warnings {
            true => Some(quote_spanned!(span=>
                #[allow(unused)]
                #[allow(unused_mut)]
                #[allow(unused_assignments)]
                #[allow(unused_variables)]
                #[allow(non_snake_case)]
            )),

            false => None,
        };

        let session_ty = match isolated_session {
            true => quote_spanned!(span=> &impl #core::syntax::SyntaxSession<#code, Node = #this>),
            false => {
                quote_spanned!(span=> &mut impl #core::syntax::SyntaxSession<#code, Node = #this>)
            }
        };

        let result = match result {
            Some(ty) => Some(quote_spanned!(span=> -> #ty)),
            None => None,
        };

        (
            ident.clone(),
            quote_spanned!(span=>
                #warnings
                fn #ident #impl_generics (
                    session: #session_ty,
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
            false,
            vec![],
            None,
            quote_spanned!(span=> #globals #body),
            allow_warnings,
        )
        .1
    }

    fn compile_abstract_feature_impl(&self) -> TokenStream {
        let ident = &self.ident;
        let span = ident.span();
        let core = span.face_core();
        let result = span.face_result();

        let (impl_generics, type_generics, where_clause) = self.generics.ty.split_for_impl();

        let capacity = self.variants.len();

        let mut attr_ref = Vec::with_capacity(capacity);
        let mut feature_getter = Vec::with_capacity(capacity);
        let mut feature_keys = Vec::with_capacity(capacity);

        for variant in self.variants.values() {
            if variant.index.is_none() {
                continue;
            }

            attr_ref.push(variant.inheritance.compile_attr_ref());
            feature_getter.push(variant.inheritance.compile_feature_getter());
            feature_keys.push(variant.inheritance.compile_feature_keys());
        }

        quote_spanned!(span=>
            impl #impl_generics #core::analysis::AbstractFeature for #ident #type_generics
            #where_clause
            {
                fn attr_ref(&self) -> &#core::analysis::AttrRef {
                    match self {
                        #( #attr_ref )*

                        #[allow(unreachable_patterns)]
                        _ => &#core::analysis::NIL_ATTR_REF,
                    }
                }

                #[allow(unused_variables)]
                fn feature(&self, key: #core::syntax::Key)
                    -> #core::analysis::AnalysisResult<&dyn #core::analysis::AbstractFeature>
                {
                    match self {
                        #( #feature_getter )*

                        #[allow(unreachable_patterns)]
                        _ => #result::Err(#core::analysis::AnalysisError::MissingFeature),
                    }
                }

                #[allow(unused_variables)]
                fn feature_keys(&self) -> &'static [&'static #core::syntax::Key] {
                    match self {
                        #( #feature_keys )*

                        #[allow(unreachable_patterns)]
                        _ => &[],
                    }
                }
            }
        )
    }

    fn compile_grammar_impl(&self) -> TokenStream {
        let ident = &self.ident;
        let span = ident.span();
        let core = span.face_core();
        let result = span.face_result();

        let classifier = match &self.classifier {
            Some(ty) => ty.to_token_stream(),
            None => quote_spanned!(span=> #core::analysis::VoidClassifier::<Self>),
        };

        let (impl_generics, type_generics, where_clause) = self.generics.ty.split_for_impl();

        let capacity = self.variants.len();

        let mut initializers = Vec::with_capacity(capacity);
        let mut invalidators = Vec::with_capacity(capacity);
        let mut scope_attr_getter = Vec::with_capacity(capacity);
        let mut is_scope = Vec::with_capacity(capacity);

        for variant in self.variants.values() {
            if variant.index.is_none() {
                continue;
            }

            initializers.push(variant.inheritance.compile_initializer());
            invalidators.push(variant.inheritance.compile_invalidator());
            scope_attr_getter.push(variant.inheritance.compile_scope_attr_getter());

            if variant.scope {
                let variant_ident = &variant.ident;

                is_scope.push(quote_spanned!(span=> Self::#variant_ident {..}))
            }
        }

        let is_scope = match is_scope.is_empty() {
            true => None,
            false => Some(quote_spanned!(span=> #( #is_scope )|* => true,)),
        };

        quote_spanned!(span=>
            impl #impl_generics #core::analysis::Grammar for #ident #type_generics
            #where_clause
            {
                type Classifier = #classifier;

                #[allow(unused_variables)]
                fn init<
                    H: #core::analysis::TaskHandle,
                    S: #core::sync::SyncBuildHasher,
                >(
                    &mut self,
                    #[allow(unused)] initializer: &mut #core::analysis::Initializer<Self, H, S>,
                ) {
                    match self {
                        #( #initializers )*

                        #[allow(unreachable_patterns)]
                        _ => (),
                    }
                }

                #[allow(unused_variables)]
                fn invalidate<
                    H: #core::analysis::TaskHandle,
                    S: #core::sync::SyncBuildHasher,
                >(
                    &self,
                    invalidator: &mut #core::analysis::Invalidator<Self, H, S>,
                ) {
                    match self {
                        #( #invalidators )*

                        #[allow(unreachable_patterns)]
                        _ => (),
                    }
                }

                fn scope_attr(&self) -> #core::analysis::AnalysisResult<&#core::analysis::ScopeAttr<Self>> {
                    match self {
                        #( #scope_attr_getter )*

                        #[allow(unreachable_patterns)]
                        _ => #result::Err(#core::analysis::AnalysisError::MissingSemantics),
                    }
                }

                #[inline(always)]
                fn is_scope(&self) -> bool {
                    match self {
                        #is_scope

                        #[allow(unreachable_patterns)]
                        _ => false,
                    }
                }
            }
        )
    }

    fn compile_abstract_node_impl(&self) -> TokenStream {
        let ident = &self.ident;
        let span = ident.span();
        let core = span.face_core();
        let option = span.face_option();

        let (impl_generics, type_generics, where_clause) = self.generics.ty.split_for_impl();

        let capacity = self.variants.len();

        let mut node_getters = Vec::with_capacity(capacity);
        let mut parent_getters = Vec::with_capacity(capacity);
        let mut parent_setters = Vec::with_capacity(capacity);
        let mut capture_getter = Vec::with_capacity(capacity);
        let mut capture_keys = Vec::with_capacity(capacity);

        for variant in self.variants.values() {
            if variant.index.is_none() {
                continue;
            }

            node_getters.push(variant.inheritance.compile_node_getter());
            parent_getters.push(variant.inheritance.compile_parent_getter());
            parent_setters.push(variant.inheritance.compile_parent_setter());
            capture_getter.push(variant.inheritance.compile_capture_getter());
            capture_keys.push(variant.inheritance.compile_capture_keys());
        }

        let mut descriptions: Vec<(&Index, &Ident, &Description)> = self
            .variants
            .values()
            .map(|variant| {
                if !variant.description.is_set() {
                    return None;
                }

                let index = expect_some!(variant.index.as_ref(), "Description without index",);

                Some((index, &variant.ident, &variant.description))
            })
            .flatten()
            .collect::<Vec<_>>();

        descriptions.sort_by_key(|(index, _, _)| index.get());

        let get_rule = descriptions
            .iter()
            .map(|(index, ident, _)| quote_spanned!(index.span() => Self::#ident { .. } => #index,))
            .collect::<Vec<_>>();

        let (rule_name, rule_description): (Vec<_>, Vec<_>) = descriptions
            .into_iter()
            .map(|(index, ident, description)| {
                let name = LitStr::new(&ident.to_string(), ident.span());
                let short = description.short();
                let verbose = description.verbose();

                (
                    quote_spanned!(index.span() => #index => #option::Some(#name),),
                    match short == verbose {
                        true => quote_spanned!(index.span() => #index => #option::Some(#verbose),),

                        false => quote_spanned!(index.span() =>
                            #index => match verbose {
                                false => #option::Some(#short),
                                true => #option::Some(#verbose),
                            },),
                    },
                )
            })
            .unzip();

        quote_spanned!(span=>
            impl #impl_generics #core::syntax::AbstractNode for #ident #type_generics
            #where_clause
            {
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
                fn name(&self) -> #option<&'static str> {
                    Self::rule_name(self.rule())
                }

                #[inline(always)]
                fn describe(&self, verbose: bool) -> #option<&'static str> {
                    Self::rule_description(self.rule(), verbose)
                }

                fn node_ref(&self) -> #core::syntax::NodeRef {
                    match self {
                        #( #node_getters )*

                        #[allow(unreachable_patterns)]
                        _ => #core::syntax::NodeRef::nil(),
                    }
                }

                fn parent_ref(&self) -> #core::syntax::NodeRef {
                    match self {
                        #( #parent_getters )*

                        #[allow(unreachable_patterns)]
                        _ => #core::syntax::NodeRef::nil(),
                    }
                }

                #[allow(unused_variables)]
                fn set_parent_ref(&mut self, parent_ref: #core::syntax::NodeRef) {
                    match self {
                        #( #parent_setters )*

                        #[allow(unreachable_patterns)]
                        _ => (),
                    }
                }

                #[allow(unused_variables)]
                fn capture(&self, key: #core::syntax::Key) -> #option::<#core::syntax::Capture> {
                    match self {
                        #( #capture_getter )*

                        #[allow(unreachable_patterns)]
                        _ => #option::None,
                    }
                }

                #[allow(unused_variables)]
                fn capture_keys(&self) -> &'static [#core::syntax::Key<'static>] {
                    match self {
                        #( #capture_keys )*

                        #[allow(unreachable_patterns)]
                        _ => &[],
                    }
                }

                #[allow(unused_variables)]
                fn rule_name(rule: #core::syntax::NodeRule) -> #option<&'static str> {
                    match rule {
                        #( #rule_name )*

                        #[allow(unreachable_patterns)]
                        _ => None,
                    }
                }

                #[allow(unused_variables)]
                fn rule_description(rule: #core::syntax::NodeRule, verbose: bool) -> #option<&'static str> {
                    match rule {
                        #( #rule_description )*

                        #[allow(unreachable_patterns)]
                        _ => None,
                    }
                }
            }
        )
    }

    fn compile_node_impl(&self, output_comments: bool) -> TokenStream {
        let ident = &self.ident;
        let span = ident.span();
        let core = span.face_core();
        let unimplemented = span.face_unimplemented();

        let token = &self.token;

        let (impl_generics, type_generics, where_clause) = self.generics.ty.split_for_impl();
        let code = &self.generics.code;

        let mut globals = Globals::default();

        let capacity = self.variants.len();

        let mut functions = Vec::with_capacity(capacity);
        let mut cases = Vec::with_capacity(capacity);

        for variant in self.variants.values() {
            if variant.index.is_none() {
                continue;
            }

            if variant.rule.is_none() {
                continue;
            }

            let index = expect_some!(variant.index.as_ref(), "Parsable rule without index.",);

            let function = expect_some!(
                variant.compile_parser_fn(self, &mut globals, true, false, output_comments, true,),
                "Parsable rule without parser.",
            );

            functions.push(function);

            let ident = variant.parser_fn_ident();

            cases.push(quote_spanned!(span=> #index => #ident(session),));
        }

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
                        #panic("EOI token cannot be used directly.");
                    }
                ))
            })
            .flatten()
            .collect::<Vec<_>>();

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
            }
        )
    }

    fn compile_consts_impl(&self) -> Option<TokenStream> {
        let ident = &self.ident;
        let vis = &self.vis;
        let span = ident.span();

        let (_, type_generics, where_clause) = self.generics.ty.split_for_impl();

        let indices = self
            .variants
            .values()
            .filter_map(|variant| {
                let Some(Index::Named(name, Some(index))) = &variant.index else {
                    return None;
                };

                let span = name.span();
                let core = span.face_core();

                Some(quote_spanned!(span=>
                    #vis const #name: #core::syntax::NodeRule = #index;
                ))
            })
            .collect::<Vec<_>>();

        if indices.is_empty() {
            return None;
        }

        Some(quote_spanned!(span=>
            impl #ident #type_generics #where_clause
            {
            #(
                #indices
            )*
            }
        ))
    }
}

impl ToTokens for NodeInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Dump::Dry(..) = self.dump {
            return;
        }

        let output_comments = match self.dump {
            Dump::None | Dump::Decl(..) => false,
            Dump::Dry(..) => return,
            _ => true,
        };

        self.compile_abstract_feature_impl().to_tokens(tokens);
        self.compile_grammar_impl().to_tokens(tokens);
        self.compile_abstract_node_impl().to_tokens(tokens);
        self.compile_node_impl(output_comments).to_tokens(tokens);
        self.compile_consts_impl().to_tokens(tokens);
    }
}
