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

use proc_macro2::{Ident, Span, TokenStream};
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

use crate::{
    node::{input::NodeInput, variables::VariableMap},
    utils::{error, expect_some},
};

pub(super) struct Constructor {
    span: Span,
    name: Ident,
    parameters: Vec<Parameter>,
    overridden: bool,
}

impl TryFrom<Attribute> for Constructor {
    type Error = Error;

    fn try_from(attr: Attribute) -> Result<Self> {
        let span = attr.span();

        attr.parse_args_with(|input: ParseStream| {
            let name = input.parse::<Ident>()?;

            let content;
            parenthesized!(content in input);

            if !input.is_empty() {
                return Err(error!(input.span(), "Unexpected end of input.",));
            }

            let parameters = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?
                .into_iter()
                .map(|name| Parameter {
                    name,
                    default: None,
                })
                .collect::<Vec<_>>();

            Ok(Self {
                span,
                name,
                parameters,
                overridden: true,
            })
        })
    }
}

impl TryFrom<Variant> for Constructor {
    type Error = Error;

    fn try_from(variant: Variant) -> Result<Self> {
        match &variant.fields {
            Fields::Unnamed(fields) => {
                return Err(error!(
                    fields.span(),
                    "Variants with unnamed fields require overridden \
                    constructor.\nAnnotate this variant with \
                    #[constructor(...)] attribute.",
                ));
            }

            _ => (),
        }

        let span = variant.span();
        let name = variant.ident.clone();

        let parameters = variant
            .fields
            .into_iter()
            .map(|field| {
                let mut default = None;

                for attribute in field.attrs {
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
                                return Err(error!(
                                    attribute.span(),
                                    "Duplicate Default attribute.",
                                ));
                            }

                            default = Some((attribute.span(), attribute.parse_args::<Expr>()?));
                        }

                        _ => (),
                    }
                }

                let name = expect_some!(field.ident, "Unnamed field.",);

                match default {
                    None => Ok(Parameter {
                        name,
                        default: None,
                    }),

                    Some((span, value)) => Ok(Parameter {
                        name,
                        default: Some((span, value)),
                    }),
                }
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            span,
            name,
            parameters,
            overridden: false,
        })
    }
}

impl Constructor {
    #[inline(always)]
    pub(super) fn span(&self) -> Span {
        self.span
    }

    pub(super) fn fits(&self, variables: &VariableMap) -> Result<()> {
        for Parameter { name, default } in &self.parameters {
            if variables.contains(name) {
                if let Some((span, _)) = default {
                    return Err(error!(
                        *span,
                        "Default attribute is not applicable here, because \
                        corresponding variable is explicitly captured in the \
                        rule expression.",
                    ));
                }

                continue;
            }

            if self.overridden {
                return Err(error!(
                    name.span(),
                    "This parameter is missing in the set of the rule \
                    capturing variables.",
                ));
            }

            if default.is_none() {
                return Err(error!(
                    name.span(),
                    "This parameter is missing in the set of the rule \
                    capturing variables.\nIf this is intending, the rule needs \
                    an explicit constructor.\nUse #[constructor(...)] \
                    attribute to specify constructor function.\n\
                    Alternatively, associate this parameter with \
                    #[default(...)] attribute.",
                ));
            }
        }

        for variable in variables {
            let has_corresponding_parameter = self
                .parameters
                .iter()
                .any(|parameter| &parameter.name == variable);

            if has_corresponding_parameter {
                continue;
            }

            if self.overridden {
                return Err(error!(
                    variable.span(),
                    "Capturing \"{variable}\" variable is missing in \
                    constructor's parameters.",
                ));
            }

            return Err(error!(
                variable.span(),
                "Capturing \"{variable}\" variable is missing in the list of \
                variant fields.",
            ));
        }

        Ok(())
    }

    pub(super) fn compile(&self, input: &NodeInput, variables: &VariableMap) -> TokenStream {
        let span = self.name.span();
        let this = input.this();
        let constructor_name = &self.name;

        if self.overridden {
            let parameters = self
                .parameters
                .iter()
                .map(|parameter| variables.get(&parameter.name));

            return quote_spanned!(span=>
                #this::#constructor_name(#( #parameters ),*));
        }

        let parameters = self.parameters.iter().map(|parameter| {
            let key = &parameter.name;
            let span = key.span();

            match &parameter.default {
                None => {
                    let variable = variables.get(key);

                    quote_spanned!(span=> #key: #variable,)
                }

                Some((span, default)) => {
                    quote_spanned!(*span=> #key: #default,)
                }
            }
        });

        quote_spanned!(span=>#this::#constructor_name {#(
            #parameters
        )*})
    }
}

struct Parameter {
    name: Ident,
    default: Option<(Span, Expr)>,
}
