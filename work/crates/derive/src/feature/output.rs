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

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{spanned::Spanned, Fields, LitStr};

use crate::{
    feature::FeatureInput,
    utils::{Dump, Facade},
};

impl ToTokens for FeatureInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Dump::Dry(..) = self.dump {
            return;
        }

        let ident = &self.ident;
        let node = &self.node;
        let vis = &self.vis;

        let span = ident.span();
        let core = span.face_core();
        let result = span.face_result();

        let mut getters = Vec::with_capacity(self.fields.len());
        let mut keys = Vec::with_capacity(self.fields.len());
        let mut constructors = Vec::with_capacity(self.fields.len());
        let mut initializers = Vec::with_capacity(self.fields.len());
        let mut invalidators = Vec::with_capacity(self.fields.len());

        for (index, field) in self.fields.iter().enumerate() {
            let ident = &field.ident;
            let ty = &field.ty;

            let span = ty.span();
            let core = span.face_core();
            let result = span.face_result();

            let invalidate = self.invalidate.contains(&index);

            match ident {
                Some(ident) => {
                    if &field.vis == vis {
                        let span = ident.span();
                        let core = ident.face_core();

                        let literal = LitStr::new(ident.to_string().as_str(), span);

                        getters.push(quote_spanned!(span=>
                            #core::syntax::Key::Index(#index)
                                | #core::syntax::Key::Name(#literal) => #result::Ok(&self.#ident)
                        ));
                        keys.push(quote_spanned!(span=> &#core::syntax::Key::Name(#literal)));
                    }

                    constructors.push(quote_spanned!(span=>
                        #ident: <#ty as #core::analysis::Feature>::new(node_ref),
                    ));

                    initializers.push(quote_spanned!(span=>
                        <#ty as #core::analysis::Feature>::init(
                            &mut self.#ident,
                            initializer,
                        );
                    ));

                    if invalidate {
                        invalidators.push(quote_spanned!(span=>
                            <#ty as #core::analysis::Feature>::invalidate(
                                &self.#ident,
                                invalidator,
                            );
                        ));
                    }
                }

                None => {
                    if &field.vis == vis {
                        getters.push(quote_spanned!(span=>
                            #core::syntax::Key::Index(#index) => #result::Ok(&self.#ident)
                        ));
                        keys.push(quote_spanned!(span=> &#core::syntax::Key::Index(#index)));
                    }

                    constructors.push(quote_spanned!(span=>
                        <#ty as #core::analysis::Feature>::new(node_ref),
                    ));

                    initializers.push(quote_spanned!(span=>
                        <#ty as #core::analysis::Feature>::init(
                            &mut self.#index,
                            initializer,
                        );
                    ));

                    if invalidate {
                        invalidators.push(quote_spanned!(span=>
                            <#ty as #core::analysis::Feature>::invalidate(
                                &self.#index,
                                invalidator,
                            );
                        ));
                    }
                }
            }
        }

        let constructor = match self.fields {
            Fields::Named(_) => quote_spanned!(span=> Self {
                #(
                #constructors
                )*
            }),

            Fields::Unnamed(_) => quote_spanned!(span=> Self(
                #(
                #constructors
                )*
            )),

            Fields::Unit => quote_spanned!(span=> Self),
        };

        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();

        quote_spanned!(span=>
            impl #impl_generics #core::analysis::AbstractFeature for #ident #type_generics
            #where_clause
            {
                #[inline(always)]
                fn attr_ref(&self) -> &#core::analysis::AttrRef {
                    &#core::analysis::NIL_ATTR_REF
                }

                fn feature(&self, key: #core::syntax::Key)
                    -> #core::analysis::AnalysisResult<&dyn #core::analysis::AbstractFeature>
                {
                    match key {
                        #(
                        #getters,
                        )*

                        _ => #result::Err(#core::analysis::AnalysisError::MissingFeature),
                    }
                }

                #[inline(always)]
                fn feature_keys(&self) -> &'static [&'static #core::syntax::Key] {
                    &[#( #keys ),*]
                }
            }

            impl #impl_generics #core::analysis::Feature for #ident #type_generics
            #where_clause
            {
                type Node = #node;

                #[inline(always)]
                #[allow(unused_variables)]
                fn new(node_ref: #core::syntax::NodeRef) -> Self {
                    #constructor
                }

                #[inline(always)]
                #[allow(unused_variables)]
                fn init<
                    H: #core::analysis::TaskHandle,
                    S: #core::sync::SyncBuildHasher,
                >(
                    &mut self,
                    initializer: &mut #core::analysis::Initializer<Self::Node, H, S>,
                ) {
                    #(
                    #initializers
                    )*
                }

                #[inline(always)]
                #[allow(unused_variables)]
                fn invalidate<
                    H: #core::analysis::TaskHandle,
                    S: #core::sync::SyncBuildHasher
                >(
                    &self,
                    #[allow(unused)] invalidator: &mut #core::analysis::Invalidator<Self::Node, H, S>,
                ) {
                    #(
                    #invalidators
                    )*
                }
            }
        )
        .to_tokens(tokens);
    }
}
