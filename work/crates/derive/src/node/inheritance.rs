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
use syn::{spanned::Spanned, AttrStyle, Error, Fields, LitStr, Result, Variant};

use crate::utils::{error, expect_some, Facade};

pub(super) struct Inheritance {
    ident: Ident,
    node: Option<Ident>,
    parent: Option<Ident>,
    children: Vec<Ident>,
}

impl<'a> TryFrom<&'a Variant> for Inheritance {
    type Error = Error;

    fn try_from(variant: &'a Variant) -> Result<Self> {
        let mut node = None;
        let mut parent = None;
        let mut children = Vec::with_capacity(variant.fields.len());

        for field in &variant.fields {
            let ident = expect_some!(&field.ident, "Unnamed field.",);

            let mut is_node = false;
            let mut is_parent = false;
            let mut is_child = false;

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
                                "Parent attribute conflicts with Node attribute.",
                            ));
                        }

                        if is_child {
                            return Err(error!(
                                attr_span,
                                "Parent attribute conflicts with Child attribute.",
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
                                "Node attribute conflicts with Parent attribute.",
                            ));
                        }

                        if is_child {
                            return Err(error!(
                                attr_span,
                                "Node attribute conflicts with Child attribute.",
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
                                "Child attribute conflicts with Node attribute.",
                            ));
                        }

                        if is_parent {
                            return Err(error!(
                                attr_span,
                                "Child attribute conflicts with Parent attribute.",
                            ));
                        }

                        is_child = true;

                        children.push(ident.clone());
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
        })
    }
}

impl Inheritance {
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

    pub(super) fn compile_children_getter(&self) -> Option<TokenStream> {
        let children = &self.children;

        if children.is_empty() {
            return None;
        }

        let ident = &self.ident;
        let span = ident.span();

        let mut pattern = Vec::with_capacity(children.len());
        let mut append = Vec::with_capacity(children.len());

        for (index, child) in children.iter().enumerate() {
            let span = child.span();
            let core = span.face_core();
            let key = LitStr::new(child.to_string().as_str(), span);
            let value = format_ident!("_{}", index, span = span);

            pattern.push(quote_spanned!(span=> #child: #value));

            append.push(quote_spanned!(span=>
                #core::syntax::Children::set(&mut children, #key, #value);
            ));
        }

        Some(quote_spanned!(span=> Self::#ident {
            #( #pattern, )*
            ..
        } => {
            #(
            #append
            )*
        }))
    }
}
