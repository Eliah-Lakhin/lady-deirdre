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
use quote::ToTokens;
use syn::{
    parse::ParseStream,
    spanned::Spanned,
    AttrStyle,
    Attribute,
    Error,
    Expr,
    Meta,
    Result,
    Type,
    Variant,
};

use crate::{
    node::{input::NodeInput, variables::VariableMap},
    utils::{error, expect_some, system_panic, Facade},
};

pub(super) struct Constructor {
    span: Span,
    mode: Mode,
}

impl<'a> TryFrom<Attribute> for Constructor {
    type Error = Error;

    fn try_from(attr: Attribute) -> Result<Self> {
        let span = attr.span();

        attr.parse_args_with(|input: ParseStream| {
            let expression = input.parse::<Expr>()?;

            Ok(Self {
                span,
                mode: Mode::Overridden { expression },
            })
        })
    }
}

impl TryFrom<Variant> for Constructor {
    type Error = Error;

    fn try_from(variant: Variant) -> Result<Self> {
        let span = variant.span();
        let ident = variant.ident.clone();

        let mut params = Vec::with_capacity(variant.fields.len());

        for field in variant.fields {
            let mut initializer = Initializer::Capture;

            let ident = expect_some!(field.ident, "Unnamed field.",);

            for attr in field.attrs {
                match attr.style {
                    AttrStyle::Inner(_) => continue,
                    AttrStyle::Outer => (),
                }

                let attr_span = attr.span();

                let name = match attr.meta.path().get_ident() {
                    Some(ident) => ident.to_string(),
                    None => continue,
                };

                match name.as_str() {
                    "default" => {
                        match &initializer {
                            Initializer::Capture => (),
                            Initializer::Default(..) | Initializer::Custom(..) => {
                                return Err(error!(attr_span, "Duplicate Default attribute.",));
                            }
                            Initializer::Semantics(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Default attribute conflicts with the Semantics attribute.",
                                ));
                            }
                            Initializer::Node(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Default attribute conflicts with the Node attribute.",
                                ));
                            }
                            Initializer::Parent(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Default attribute conflicts with the Parent attribute.",
                                ));
                            }
                        }

                        if let Meta::Path(..) = &attr.meta {
                            initializer = Initializer::Default(attr_span);
                            continue;
                        }

                        initializer = Initializer::Custom(attr_span, attr.parse_args::<Expr>()?);
                    }

                    "semantics" => {
                        match &initializer {
                            Initializer::Capture => (),
                            Initializer::Default(..) | Initializer::Custom(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Semantics attribute conflicts with the Default attribute.",
                                ));
                            }
                            Initializer::Semantics(..) => {
                                return Err(error!(attr_span, "Duplicate Semantics attribute.",));
                            }
                            Initializer::Node(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Semantics attribute conflicts with the Node attribute.",
                                ));
                            }
                            Initializer::Parent(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Semantics attribute conflicts with the Parent attribute.",
                                ));
                            }
                        }

                        initializer = Initializer::Semantics(attr_span);
                    }

                    "node" => {
                        match &initializer {
                            Initializer::Capture => (),
                            Initializer::Default(..) | Initializer::Custom(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Node attribute conflicts with the Default attribute.",
                                ));
                            }
                            Initializer::Semantics(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Node attribute conflicts with the Semantics attribute.",
                                ));
                            }
                            Initializer::Node(..) => {
                                return Err(error!(attr_span, "Duplicate Node field.",));
                            }
                            Initializer::Parent(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Node attribute conflicts with the Parent attribute.",
                                ));
                            }
                        }

                        initializer = Initializer::Node(attr_span);
                    }

                    "parent" => {
                        match &initializer {
                            Initializer::Capture => (),
                            Initializer::Default(..) | Initializer::Custom(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Parent attribute conflicts with the Default attribute.",
                                ));
                            }
                            Initializer::Semantics(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Parent attribute conflicts with the Semantics attribute.",
                                ));
                            }
                            Initializer::Node(..) => {
                                return Err(error!(
                                    attr_span,
                                    "Parent attribute conflicts with the Node attribute.",
                                ));
                            }
                            Initializer::Parent(..) => {
                                return Err(error!(attr_span, "Duplicate Parent field.",));
                            }
                        }

                        initializer = Initializer::Parent(attr_span);
                    }

                    _ => (),
                }
            }

            params.push(Parameter {
                ident,
                ty: field.ty,
                initializer,
            });
        }

        Ok(Self {
            span,
            mode: Mode::Instance { ident, params },
        })
    }
}

impl Constructor {
    #[inline(always)]
    pub(super) fn span(&self) -> Span {
        self.span
    }

    pub(super) fn fits(&self, variables: &VariableMap) -> Result<()> {
        match &self.mode {
            Mode::Overridden { .. } => Ok(()),

            Mode::Instance { params, .. } => {
                for Parameter {
                    ident, initializer, ..
                } in params
                {
                    if variables.contains(ident) {
                        match initializer {
                            Initializer::Capture => (),
                            Initializer::Default(span) | Initializer::Custom(span, ..) => {
                                return Err(error!(
                                    *span,
                                    "Default attribute is not applicable here, \
                                    because corresponding variable is \
                                    explicitly captured in the rule expression.",
                                ));
                            }
                            Initializer::Semantics(span) => {
                                return Err(error!(
                                    *span,
                                    "Semantics attribute is not applicable here, \
                                    because corresponding variable is \
                                    explicitly captured in the rule expression.",
                                ));
                            }
                            Initializer::Node(..) => {
                                system_panic!("\"node\" variable capturing.");
                            }
                            Initializer::Parent(..) => {
                                system_panic!("\"parent\" variable capturing.");
                            }
                        }

                        continue;
                    }

                    if let Initializer::Capture = initializer {
                        return Err(error!(
                            ident.span(),
                            "This parameter is missing in the set of the rule \
                            capturing variables.\nIf this is intending, the \
                            rule needs an explicit \
                            constructor.\nUse #[constructor(...)] \
                            attribute to specify constructor \
                            expression.\nAlternatively, associate this \
                            parameter with #[default] or #[default(...)] \
                            attribute.",
                        ));
                    }
                }

                for variable in variables {
                    let has_corresponding_parameter =
                        params.iter().any(|parameter| &parameter.ident == variable);

                    if has_corresponding_parameter {
                        continue;
                    }

                    return Err(error!(
                        variable.span(),
                        "Capturing \"{variable}\" variable is missing in the \
                        list of variant fields.\nIf this is intending, the \
                        rule needs an explicit constructor.\nUse \
                        #[constructor(...)] attribute to specify constructor \
                        expression.",
                    ));
                }

                Ok(())
            }
        }
    }

    pub(super) fn compile(
        &self,
        input: &NodeInput,
        variables: &VariableMap,
        allow_warnings: bool,
    ) -> TokenStream {
        let span = self.span();
        let this = input.this();

        match &self.mode {
            Mode::Instance { ident, params, .. } => {
                let params = params.iter().map(|param| {
                    let ident = &param.ident;
                    let span = ident.span();

                    match &param.initializer {
                        Initializer::Capture => {
                            let variable = variables.get(ident);

                            quote_spanned!(span=> #ident: #variable,)
                        }

                        Initializer::Node(value_span) => {
                            let core = value_span.face_core();
                            let value = quote_spanned!(*value_span=>
                                #core::syntax::SyntaxSession::node_ref(session));

                            quote_spanned!(span=> #ident: #value,)
                        }

                        Initializer::Parent(value_span) => {
                            let core = value_span.face_core();
                            let value = quote_spanned!(*value_span=>
                                #core::syntax::SyntaxSession::parent_ref(session));

                            quote_spanned!(span=> #ident: #value,)
                        }

                        Initializer::Default(value_span) => {
                            let default = value_span.face_default();
                            let ty = &param.ty;

                            let value = quote_spanned!(*value_span=> <#ty as #default>::default());

                            quote_spanned!(span=> #ident: #value,)
                        }

                        Initializer::Custom(value_span, expr) => {
                            let ty = &param.ty;

                            let (fn_ident, fn_impl) = input.make_fn(
                                format_ident!("default", span = *value_span),
                                true,
                                vec![],
                                Some(ty.to_token_stream()),
                                expr.to_token_stream(),
                                allow_warnings,
                            );

                            quote_spanned!(span=> #ident: {
                                #[inline(always)]
                                #fn_impl
                                #fn_ident(session)
                            },)
                        }

                        Initializer::Semantics(value_span) => {
                            let core = value_span.face_core();
                            let ty = &param.ty;

                            let value = quote_spanned!(*value_span=>
                                <#ty as #core::analysis::Feature>::new_uninitialized(
                                    #core::syntax::SyntaxSession::node_ref(session),
                                )
                            );

                            quote_spanned!(span=> #ident: #value,)
                        }
                    }
                });

                quote_spanned!(span=>#this::#ident {#(
                    #params
                )*})
            }

            Mode::Overridden { expression } => {
                let mut params = Vec::with_capacity(variables.len());
                let mut args = Vec::with_capacity(variables.len());

                for ident in variables {
                    let ty = variables.get(ident).ty();

                    params.push(quote_spanned!(span=> #ident: #ty));
                    args.push(ident.clone());
                }

                let (fn_ident, fn_impl) = input.make_fn(
                    format_ident!("constructor", span = span),
                    true,
                    params,
                    Some(this),
                    expression.to_token_stream(),
                    allow_warnings,
                );

                quote_spanned!(span=> {
                    #[inline(always)]
                    #fn_impl
                    #fn_ident(session #(, #args )*)
                },)
            }
        }
    }
}

enum Mode {
    Instance {
        ident: Ident,
        params: Vec<Parameter>,
    },

    Overridden {
        expression: Expr,
    },
}

struct Parameter {
    ident: Ident,
    ty: Type,
    initializer: Initializer,
}

enum Initializer {
    Capture,
    Node(Span),
    Parent(Span),
    Default(Span),
    Semantics(Span),
    Custom(Span, Expr),
}
