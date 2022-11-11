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

pub(in crate::node) mod case;
pub(in crate::node) mod constructor;
pub(in crate::node) mod delimiters;
pub(in crate::node) mod function;
pub(in crate::node) mod generics;
pub(in crate::node) mod inserts;
pub(in crate::node) mod transitions;
pub(in crate::node) mod variables;

use proc_macro2::{Ident, TokenStream};

use crate::{
    node::{
        builder::{kind::VariantKind, Builder},
        compiler::{
            function::Function,
            generics::{GenericsExt, GenericsSplit},
        },
    },
    utils::{Facade, Map, PredictableCollection},
};

pub(in crate::node) struct Compiler<'a> {
    facade: Facade,
    builder: &'a Builder,
    node_type: TokenStream,
    generics: GenericsSplit,
    kind_map: Map<&'a Ident, usize>,
    cases: Map<usize, TokenStream>,
    functions: Map<usize, TokenStream>,
}

impl<'a> Compiler<'a> {
    pub(in crate::node) fn compile(builder: &'a Builder) -> TokenStream {
        let node_name = builder.node_name();
        let token_type = builder.token_type();
        let error_type = builder.error_type();

        let generics = builder.generics().to_split();

        let node_type = {
            let node_type_generics = generics.node_type_generics();
            let turbofish = node_type_generics.as_turbofish();

            quote! {
                #node_name #turbofish
            }
        };

        let variants_count = builder.variants_count();

        let kind_map = {
            let mut kind = 0;

            builder
                .into_iter()
                .filter_map(|name| {
                    let variant = builder.variant(name);

                    match variant.kind() {
                        VariantKind::Unspecified(..) => None,

                        VariantKind::Root(..) => Some((name, 0)),

                        VariantKind::Comment(..) | VariantKind::Sentence(..) => {
                            kind += 1;

                            Some((name, kind))
                        }
                    }
                })
                .collect()
        };

        let mut compiler = Compiler {
            facade: Facade::new(),
            builder,
            node_type,
            generics,
            kind_map,
            cases: Map::with_capacity(variants_count),
            functions: Map::with_capacity(variants_count),
        };

        for variant_name in builder {
            Function::compile_case(&mut compiler, variant_name);
            Function::compile_variant_function(&mut compiler, variant_name);
        }

        let skip = Function::compile_skip_function(&mut compiler);

        let node_impl_generics = compiler.generics.node_impl_generics();
        let node_type_generics = compiler.generics.node_type_generics();
        let node_where_clause = compiler.generics.node_where_clause();
        let code_lifetime = compiler.generics.code_lifetime();

        let cases = {
            let mut cases = compiler.cases.into_iter().collect::<Vec<_>>();

            cases.sort_by(|a, b| a.0.cmp(&b.0));

            cases.into_iter().map(|(_, body)| body)
        };

        let functions = {
            let mut functions = compiler.functions.into_iter().collect::<Vec<_>>();

            functions.sort_by(|a, b| a.0.cmp(&b.0));

            functions.into_iter().map(|(_, body)| body)
        };

        let core = compiler.facade.core_crate();
        let unimplemented = compiler.facade.unimplemented();

        quote! {
            impl #node_impl_generics #core::syntax::Node for #node_name #node_type_generics
            #node_where_clause
            {
                type Token = #token_type;
                type Error = #error_type;

                #[inline(always)]
                fn new<#code_lifetime>(
                    rule: #core::syntax::SyntaxRule,
                    session: &mut impl #core::syntax::SyntaxSession<#code_lifetime, Node = Self>,
                ) -> Self
                {
                    #( #functions )*

                    #skip

                    match rule {
                        #( #cases, )*

                        other => #unimplemented("Unsupported rule {}.", other),
                    }
                }
            }
        }
    }

    #[inline(always)]
    pub(in crate::node) fn facade(&self) -> &Facade {
        &self.facade
    }

    #[inline(always)]
    pub(in crate::node) fn builder(&self) -> &Builder {
        &self.builder
    }

    #[inline(always)]
    pub(in crate::node) fn generics(&self) -> &GenericsSplit {
        &self.generics
    }

    #[inline(always)]
    pub(in crate::node) fn node_type(&self) -> &TokenStream {
        &self.node_type
    }

    #[inline(always)]
    pub(in crate::node) fn add_case(&mut self, kind: usize, body: TokenStream) {
        assert!(
            self.cases.insert(kind, body).is_none(),
            "internal error. Duplicate case.",
        );
    }

    #[inline(always)]
    pub(in crate::node) fn add_function(&mut self, kind: usize, body: TokenStream) {
        assert!(
            self.functions.insert(kind, body).is_none(),
            "internal error. Duplicate function.",
        );
    }

    #[inline(always)]
    pub(in crate::node) fn kind_of(&self, variant_name: &Ident) -> usize {
        *self
            .kind_map
            .get(variant_name)
            .expect("Internal error. Missing variant kind.")
    }

    #[inline(always)]
    pub(in crate::node) fn function_of(&self, variant_name: &Ident) -> Ident {
        Ident::new(&format!("parse_{}", variant_name), variant_name.span())
    }
}
