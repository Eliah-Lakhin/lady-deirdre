////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

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
    Fields,
    File,
    Generics,
    Result,
    Type,
    Visibility,
};

use crate::utils::{error, system_panic, Dump, PredictableCollection, Set};

pub struct FeatureInput {
    pub(super) ident: Ident,
    pub(super) generics: Generics,
    pub(super) vis: Visibility,
    pub(super) fields: Fields,
    pub(super) node: Type,
    pub(super) invalidate: Set<usize>,
    pub(crate) dump: Dump,
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
                    "Node type was not specified.\nUse #[node(<node type>)] \
                    attribute on the derived type to specify the Node type.",
                ));
            }
        };

        let mut invalidate = Set::with_capacity(fields.len());

        for (index, field) in fields.iter().enumerate() {
            let mut invalidate_flag = false;

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
                    "scoped" => {
                        if invalidate_flag {
                            return Err(error!(span, "Duplicate Scoped attribute.",));
                        }

                        invalidate_flag = true;
                    }

                    "dump" => {
                        return Err(error!(span, "Dump attribute is not applicable here.",));
                    }

                    _ => continue,
                }
            }

            if invalidate_flag {
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
            Dump::None | Dump::Dry(_) | Dump::Decl(_) => {}

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
                    " -- Macro Debug Dump --\n\nFeature \"{ident}\" \
                    implementation code:\n\n{output_string}",
                ));
            }
        }

        Ok(result)
    }
}
