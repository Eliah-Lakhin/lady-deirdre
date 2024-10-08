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

use proc_macro2::{Ident, TokenStream};
use syn::{spanned::Spanned, AttrStyle, Error, LitStr, Result, Type, Variant};

use crate::utils::{error, expect_some, Facade};

pub(super) struct Inheritance {
    ident: Ident,
    node: Option<Ident>,
    parent: Option<Ident>,
    children: Vec<Ident>,
    semantics: Option<(Ident, Type)>,
}

impl<'a> TryFrom<&'a Variant> for Inheritance {
    type Error = Error;

    fn try_from(variant: &'a Variant) -> Result<Self> {
        let mut node = None;
        let mut parent = None;
        let mut children = Vec::with_capacity(variant.fields.len());
        let mut semantics = None;

        for field in &variant.fields {
            let ident = expect_some!(&field.ident, "Unnamed field.",);

            let mut is_node = false;
            let mut is_parent = false;
            let mut is_child = false;
            let mut is_semantics = false;

            for attr in &field.attrs {
                match &attr.style {
                    AttrStyle::Inner(_) => continue,
                    AttrStyle::Outer => (),
                }

                let attr_span = attr.span();

                let name = match attr.meta.path().get_ident() {
                    Some(ident) => ident.to_string(),
                    None => continue,
                };

                match name.as_str() {
                    "parent" => {
                        if parent.is_some() {
                            return Err(error!(attr_span, "Duplicate Parent attribute.",));
                        }

                        if is_node {
                            return Err(error!(
                                attr_span,
                                "Parent attribute conflicts with the Node attribute.",
                            ));
                        }

                        if is_child {
                            return Err(error!(
                                attr_span,
                                "Parent attribute conflicts with the Child attribute.",
                            ));
                        }

                        if is_semantics {
                            return Err(error!(
                                attr_span,
                                "Parent attribute conflicts with the Semantics attribute.",
                            ));
                        }

                        is_parent = true;

                        parent = Some(ident.clone());
                    }

                    "node" => {
                        if node.is_some() {
                            return Err(error!(attr_span, "Duplicate Node attribute.",));
                        }

                        if is_parent {
                            return Err(error!(
                                attr_span,
                                "Node attribute conflicts with the Parent attribute.",
                            ));
                        }

                        if is_child {
                            return Err(error!(
                                attr_span,
                                "Node attribute conflicts with the Child attribute.",
                            ));
                        }

                        if is_semantics {
                            return Err(error!(
                                attr_span,
                                "Node attribute conflicts with the Semantics attribute.",
                            ));
                        }

                        is_node = true;

                        node = Some(ident.clone());
                    }

                    "child" => {
                        if is_child {
                            return Err(error!(attr_span, "Duplicate Child attribute.",));
                        }

                        if is_node {
                            return Err(error!(
                                attr_span,
                                "Child attribute conflicts with the Node attribute.",
                            ));
                        }

                        if is_parent {
                            return Err(error!(
                                attr_span,
                                "Child attribute conflicts with the Parent attribute.",
                            ));
                        }

                        if is_semantics {
                            return Err(error!(
                                attr_span,
                                "Child attribute conflicts with the Semantics attribute.",
                            ));
                        }

                        is_child = true;

                        children.push(ident.clone());
                    }

                    "semantics" => {
                        if semantics.is_some() {
                            return Err(error!(
                                attr_span,
                                "Node variant can have at most one Semantics field.",
                            ));
                        }

                        if is_semantics {
                            return Err(error!(attr_span, "Duplicate Semantics attribute.",));
                        }

                        if is_child {
                            return Err(error!(
                                attr_span,
                                "Semantics attribute conflicts with the Child attribute.",
                            ));
                        }

                        if is_node {
                            return Err(error!(
                                attr_span,
                                "Semantics attribute conflicts with the Node attribute.",
                            ));
                        }

                        if is_parent {
                            return Err(error!(
                                attr_span,
                                "Semantics attribute conflicts with the Parent attribute.",
                            ));
                        }

                        is_semantics = true;

                        semantics = Some((ident.clone(), field.ty.clone()));
                    }

                    _ => (),
                }
            }
        }

        Ok(Self {
            ident: variant.ident.clone(),
            node,
            parent,
            children,
            semantics,
        })
    }
}

impl Inheritance {
    pub(super) fn has_parent(&self) -> bool {
        self.parent.is_some()
    }

    pub(super) fn has_node(&self) -> bool {
        self.node.is_some()
    }

    pub(super) fn has_semantics(&self) -> bool {
        self.semantics.is_some()
    }

    pub(super) fn compile_node_getter(&self) -> Option<TokenStream> {
        let node = self.node.as_ref()?;
        let ident = &self.ident;
        let span = node.span();

        Some(quote_spanned!(span=> Self::#ident { #node, .. } => *#node,))
    }

    pub(super) fn compile_parent_getter(&self) -> Option<TokenStream> {
        let parent = self.parent.as_ref()?;
        let ident = &self.ident;
        let span = parent.span();

        Some(quote_spanned!(span=> Self::#ident { #parent, .. } => *#parent,))
    }

    pub(super) fn compile_parent_setter(&self) -> Option<TokenStream> {
        let parent = self.parent.as_ref()?;
        let ident = &self.ident;
        let span = parent.span();

        Some(
            quote_spanned!(span=> Self::#ident { #parent: target, .. } => {
                *target = parent_ref;
            },),
        )
    }

    pub(super) fn compile_capture_getter(&self) -> Option<TokenStream> {
        let children = &self.children;

        if children.is_empty() {
            return None;
        }

        let mut pattern = Vec::with_capacity(children.len());
        let mut body = Vec::with_capacity(children.len());

        for (index, child) in children.iter().enumerate() {
            let span = child.span();
            let core = span.face_core();
            let option = span.face_option();
            let from = span.face_from();

            let key = LitStr::new(child.to_string().as_str(), span);
            let value = format_ident!("_{}", index, span = span);

            pattern.push(quote_spanned!(span=> #child: #value));

            body.push(quote_spanned!(span=>
                #core::syntax::Key::Index(#index) | #core::syntax::Key::Name(#key) =>
                    #option::Some(<#core::syntax::Capture as #from::<_>>::from(#value)),
            ));
        }

        let ident = &self.ident;
        let span = ident.span();
        let option = span.face_option();

        Some(quote_spanned!(span=> Self::#ident {
            #( #pattern, )*
            ..
        } => match key {
            #(
            #body
            )*
            _ => #option::None,
        }))
    }

    pub(super) fn compile_capture_keys(&self) -> Option<TokenStream> {
        let children = &self.children;

        if children.is_empty() {
            return None;
        }

        let mut keys = Vec::with_capacity(children.len());

        for child in children.iter() {
            let span = child.span();
            let core = span.face_core();

            let key = LitStr::new(child.to_string().as_str(), span);

            keys.push(quote_spanned!(span=> #core::syntax::Key::Name(#key)));
        }

        let ident = &self.ident;
        let span = ident.span();

        Some(quote_spanned!(span=> Self::#ident { .. } => &[#(#keys),*],))
    }

    pub(super) fn compile_initializer(&self) -> Option<TokenStream> {
        let (field_ident, field_ty) = self.semantics.as_ref()?;

        let body = {
            let span = field_ty.span();
            let core = span.face_core();

            quote_spanned!(span=>
                <#field_ty as #core::analysis::Feature>::init(_0, initializer);
            )
        };

        let ident = &self.ident;
        let span = ident.span();

        Some(
            quote_spanned!(span=> Self::#ident { #field_ident: _0, .. } => {
                #body
            }),
        )
    }

    pub(super) fn compile_invalidator(&self) -> Option<TokenStream> {
        let (field_ident, field_ty) = self.semantics.as_ref()?;

        let body = {
            let span = field_ty.span();
            let core = span.face_core();

            quote_spanned!(span=>
                <#field_ty as #core::analysis::Feature>::invalidate(_0, invalidator);
            )
        };

        let ident = &self.ident;
        let span = ident.span();

        Some(
            quote_spanned!(span=> Self::#ident { #field_ident: _0, .. } => {
                #body
            }),
        )
    }

    pub(super) fn compile_attr_ref(&self) -> Option<TokenStream> {
        let (field_ident, field_ty) = self.semantics.as_ref()?;

        let body = {
            let span = field_ty.span();
            let core = span.face_core();

            quote_spanned!(span=>
                <#field_ty as #core::analysis::AbstractFeature>::attr_ref(_0)
            )
        };

        let ident = &self.ident;
        let span = ident.span();

        Some(quote_spanned!(span=> Self::#ident { #field_ident: _0, .. } => #body,))
    }

    pub(super) fn compile_slot_ref(&self) -> Option<TokenStream> {
        let (field_ident, field_ty) = self.semantics.as_ref()?;

        let body = {
            let span = field_ty.span();
            let core = span.face_core();

            quote_spanned!(span=>
                <#field_ty as #core::analysis::AbstractFeature>::slot_ref(_0)
            )
        };

        let ident = &self.ident;
        let span = ident.span();

        Some(quote_spanned!(span=> Self::#ident { #field_ident: _0, .. } => #body,))
    }

    pub(super) fn compile_feature_getter(&self) -> Option<TokenStream> {
        let (field_ident, field_ty) = self.semantics.as_ref()?;

        let body = {
            let span = field_ty.span();
            let core = span.face_core();

            quote_spanned!(span=>
                <#field_ty as #core::analysis::AbstractFeature>::feature(_0, key)
            )
        };

        let ident = &self.ident;
        let span = ident.span();

        Some(quote_spanned!(span=> Self::#ident { #field_ident: _0, .. } => #body,))
    }

    pub(super) fn compile_feature_keys(&self) -> Option<TokenStream> {
        let (field_ident, field_ty) = self.semantics.as_ref()?;

        let body = {
            let span = field_ty.span();
            let core = span.face_core();

            quote_spanned!(span=>
                <#field_ty as #core::analysis::AbstractFeature>::feature_keys(_0)
            )
        };

        let ident = &self.ident;
        let span = ident.span();

        Some(quote_spanned!(span=> Self::#ident { #field_ident: _0, .. } => #body,))
    }

    pub(super) fn compile_scope_attr_getter(&self) -> Option<TokenStream> {
        let (field_ident, field_ty) = self.semantics.as_ref()?;

        let body = {
            let span = field_ty.span();
            let core = span.face_core();

            quote_spanned!(span=>
                #core::analysis::Semantics::scope_attr(_0)
            )
        };

        let ident = &self.ident;
        let span = ident.span();

        Some(quote_spanned!(span=> Self::#ident { #field_ident: _0, .. } => #body,))
    }
}
