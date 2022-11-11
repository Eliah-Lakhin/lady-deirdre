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

use proc_macro2::{Ident, Span};
use syn::{
    parse::ParseStream,
    punctuated::Punctuated,
    spanned::Spanned,
    AttrStyle,
    Attribute,
    Error,
    Expr,
    Fields,
    Result,
    Variant,
};

pub(in crate::node) struct Constructor {
    span: Span,
    name: Ident,
    parameters: Vec<Parameter>,
    explicit: bool,
}

impl Spanned for Constructor {
    #[inline(always)]
    fn span(&self) -> Span {
        self.span
    }
}

impl<'a> TryFrom<&'a Attribute> for Constructor {
    type Error = Error;

    fn try_from(attribute: &'a Attribute) -> Result<Self> {
        let span = attribute.span();

        attribute.parse_args_with(|input: ParseStream| {
            let name = input.parse::<Ident>()?;

            let content;
            parenthesized!(content in input);

            let parameters = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?
                .into_iter()
                .map(|name| Parameter {
                    name,
                    default_value: None,
                    default_attribute: None,
                })
                .collect::<Vec<_>>();

            Ok(Self {
                span,
                name,
                parameters,
                explicit: true,
            })
        })
    }
}

impl<'a> TryFrom<&'a Variant> for Constructor {
    type Error = Error;

    fn try_from(variant: &'a Variant) -> Result<Self> {
        match &variant.fields {
            Fields::Unnamed(fields) => {
                return Err(Error::new(
                    fields.span(),
                    "Variants with unnamed fields require explicit constructor.\nAnnotate \
                    this variant with #[constructor(...)] attribute.",
                ));
            }

            _ => (),
        }

        let span = variant.span();
        let name = variant.ident.clone();

        let parameters = variant
            .fields
            .iter()
            .map(|field| {
                let mut default = None;

                for attribute in &field.attrs {
                    match attribute.style {
                        AttrStyle::Inner(_) => continue,
                        AttrStyle::Outer => (),
                    }

                    let name = match attribute.path.get_ident() {
                        None => continue,
                        Some(name) => name,
                    };

                    match name.to_string().as_str() {
                        "default" => {
                            if default.is_some() {
                                return Err(Error::new(
                                    attribute.span(),
                                    "Duplicate Default attribute.",
                                ));
                            }

                            default = Some((attribute.span(), attribute.parse_args::<Expr>()?));
                        }

                        _ => (),
                    }
                }

                let name = field.ident.clone().expect("Internal error. Unnamed field.");

                match default {
                    None => Ok(Parameter {
                        name,
                        default_value: None,
                        default_attribute: None,
                    }),

                    Some((span, value)) => Ok(Parameter {
                        name,
                        default_value: Some(value),
                        default_attribute: Some(span),
                    }),
                }
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            span,
            name,
            parameters,
            explicit: false,
        })
    }
}

impl Constructor {
    #[inline(always)]
    pub(in crate::node) fn name(&self) -> &Ident {
        &self.name
    }

    #[inline(always)]
    pub(in crate::node) fn is_explicit(&self) -> bool {
        self.explicit
    }

    #[inline(always)]
    pub(in crate::node) fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }
}

pub(in crate::node) struct Parameter {
    name: Ident,
    default_value: Option<Expr>,
    default_attribute: Option<Span>,
}

impl Parameter {
    #[inline(always)]
    pub(in crate::node) fn name(&self) -> &Ident {
        &self.name
    }

    #[inline(always)]
    pub(in crate::node) fn is_default(&self) -> bool {
        self.default_value.is_some()
    }

    #[inline(always)]
    pub(in crate::node) fn default_value(&self) -> &Expr {
        self.default_value
            .as_ref()
            .expect("Internal error. Missing default value.")
    }

    #[inline(always)]
    pub(in crate::node) fn default_attribute(&self) -> &Span {
        self.default_attribute
            .as_ref()
            .expect("Internal error. Missing default attribute.")
    }
}
