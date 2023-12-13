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
    Fields,
    File,
    Generics,
    LitStr,
    Result,
    Type,
    Visibility,
};

use crate::utils::{error, system_panic, Dump, Facade, PredictableCollection, Set};

pub struct FeatureInput {
    ident: Ident,
    generics: Generics,
    vis: Visibility,
    fields: Fields,
    node: Type,
    invalidate: Set<usize>,
    dump: Dump,
}

impl Parse for FeatureInput {
    #[inline(always)]
    fn parse(input: ParseStream) -> Result<Self> {
        let derive_input = input.parse::<DeriveInput>()?;

        Self::try_from(derive_input)
    }
}

impl TryFrom<DeriveInput> for FeatureInput {
    type Error = Error;

    fn try_from(input: DeriveInput) -> Result<Self> {
        let ident = input.ident;
        let generics = input.generics;
        let vis = input.vis;

        let data = match input.data {
            Data::Struct(data) => data,

            other => {
                let span = match other {
                    Data::Enum(data) => data.enum_token.span,
                    Data::Union(data) => data.union_token.span,
                    _ => system_panic!("Unsupported Item format."),
                };

                return Err(error!(
                    span,
                    "Feature must be derived from the struct type.",
                ));
            }
        };

        let fields = data.fields;

        let mut node = None;
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
                "node" => {
                    if node.is_some() {
                        return Err(error!(span, "Duplicate Node attribute.",));
                    }

                    node = Some(attr.parse_args::<Type>()?);
                }

                "invalidate" => {
                    return Err(error!(span, "Invalidate attribute.",));
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

        let node = match node {
            Some(ty) => ty,

            None => {
                return Err(error!(
                    ident.span(),
                    "Node type was not specified.\nUse #[node(<node name>)] \
                    attribute on the derived type to specify Node type.",
                ));
            }
        };

        let mut invalidate = Set::with_capacity(fields.len());

        for (index, field) in fields.iter().enumerate() {
            let mut flag = false;

            for attr in &field.attrs {
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
                    "invalidate" => {
                        if flag {
                            return Err(error!(span, "Duplicate Invalidate attribute.",));
                        }

                        flag = true;
                    }

                    "dump" => {
                        return Err(error!(span, "Dump attribute is not applicable here.",));
                    }

                    _ => continue,
                }
            }

            if flag {
                let _ = invalidate.insert(index);
            }
        }

        let result = Self {
            ident,
            generics,
            vis,
            fields,
            node,
            invalidate,
            dump,
        };

        match dump {
            Dump::None | Dump::Dry(_) => {}

            Dump::Trivia(span) | Dump::Meta(span) => {
                return Err(error!(
                    span,
                    "This type of the dump mode is not applicable to the Feature macros.",
                ));
            }

            Dump::Output(span) => {
                let output = result.to_token_stream();

                let output_string = match parse2::<File>(output.clone()) {
                    Ok(file) => prettyplease::unparse(&file),
                    Err(_) => output.to_string(),
                };

                let ident = &result.ident;

                return Err(error!(
                    span,
                    " -- Macro System Debug Dump --\n\nFeature \"{ident}\" \
                    implementation code:\n\n{output_string}",
                ));
            }
        }

        Ok(result)
    }
}

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
        let option = span.face_option();

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
            let option = span.face_option();

            let invalidate = self.invalidate.contains(&index);

            match ident {
                Some(ident) => {
                    if &field.vis == vis {
                        let span = ident.span();
                        let core = ident.face_core();

                        let literal = LitStr::new(ident.to_string().as_str(), span);

                        getters.push(quote_spanned!(span=>
                            #core::syntax::Key::Index(#index)
                                | #core::syntax::Key::Name(#literal) => #option::Some(&self.#ident)
                        ));
                        keys.push(quote_spanned!(span=> &#core::syntax::Key::Name(#literal)));
                    }

                    constructors.push(quote_spanned!(span=>
                        #ident: <#ty as #core::analysis::Feature>::new_uninitialized(node_ref),
                    ));

                    initializers.push(quote_spanned!(span=>
                        <#ty as #core::analysis::Feature>::initialize(
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
                            #core::syntax::Key::Index(#index) => #option::Some(&self.#ident)
                        ));
                        keys.push(quote_spanned!(span=> &#core::syntax::Key::Index(#index)));
                    }

                    constructors.push(quote_spanned!(span=>
                        <#ty as #core::analysis::Feature>::new_uninitialized(node_ref),
                    ));

                    initializers.push(quote_spanned!(span=>
                        <#ty as #core::analysis::Feature>::initialize(
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
                    static NIL_REF: #core::analysis::AttrRef = #core::analysis::AttrRef::nil();

                    &NIL_REF
                }

                fn feature(&self, key: #core::syntax::Key)
                    -> #option<&dyn #core::analysis::AbstractFeature>
                {
                    match key {
                        #(
                        #getters,
                        )*

                        _ => #option::None,
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

                #[allow(unused_variables)]
                fn new_uninitialized(node_ref: #core::syntax::NodeRef) -> Self {
                    #constructor
                }

                #[allow(unused_variables)]
                fn initialize<S: #core::sync::SyncBuildHasher>(
                    &mut self,
                    initializer: &mut #core::analysis::FeatureInitializer<Self::Node, S>,
                ) {
                    #(
                    #initializers
                    )*
                }

                #[allow(unused_variables)]
                fn invalidate<S: #core::sync::SyncBuildHasher>(
                    &self,
                    #[allow(unused)] invalidator: &mut #core::analysis::FeatureInvalidator<Self::Node, S>,
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
