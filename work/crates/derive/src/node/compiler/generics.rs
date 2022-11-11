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

use proc_macro2::Span;
use syn::{
    punctuated::Punctuated,
    GenericParam,
    Generics,
    ImplGenerics,
    Lifetime,
    LifetimeDef,
    TypeGenerics,
    WhereClause,
};

pub(in crate::node) struct GenericsSplit {
    node_generics: Generics,
    function_generics: Generics,
    code_lifetime: Lifetime,
    outer_lifetime: Lifetime,
}

impl GenericsSplit {
    #[inline(always)]
    pub(in crate::node) fn node_impl_generics(&self) -> ImplGenerics<'_> {
        let (impl_generics, _, _) = self.node_generics.split_for_impl();

        impl_generics
    }

    #[inline(always)]
    pub(in crate::node) fn node_type_generics(&self) -> TypeGenerics<'_> {
        let (_, type_generics, _) = self.node_generics.split_for_impl();

        type_generics
    }

    #[inline(always)]
    pub(in crate::node) fn node_where_clause(&self) -> Option<&WhereClause> {
        let (_, _, where_clause) = self.node_generics.split_for_impl();

        where_clause
    }
    #[inline(always)]
    pub(in crate::node) fn function_impl_generics(&self) -> ImplGenerics<'_> {
        let (impl_generics, _, _) = self.function_generics.split_for_impl();

        impl_generics
    }

    #[inline(always)]
    #[allow(unused)]
    pub(in crate::node) fn function_type_generics(&self) -> TypeGenerics<'_> {
        let (_, type_generics, _) = self.function_generics.split_for_impl();

        type_generics
    }

    #[inline(always)]
    pub(in crate::node) fn function_where_clause(&self) -> Option<&WhereClause> {
        let (_, _, where_clause) = self.function_generics.split_for_impl();

        where_clause
    }

    #[inline(always)]
    pub(in crate::node) fn code_lifetime(&self) -> &Lifetime {
        &self.code_lifetime
    }

    #[inline(always)]
    pub(in crate::node) fn outer_lifetime(&self) -> &Lifetime {
        &self.outer_lifetime
    }
}

impl GenericsExt for Generics {
    fn to_split(&self) -> GenericsSplit {
        let node_generics = self.clone();

        let code_lifetime = {
            let mut candidate = String::from("'code");

            'outer: loop {
                for lifetime_def in self.lifetimes() {
                    if candidate == lifetime_def.lifetime.ident.to_string() {
                        candidate.push('_');
                        continue 'outer;
                    }
                }

                break;
            }

            Lifetime::new(candidate.as_str(), Span::call_site())
        };

        let outer_lifetime = {
            let mut candidate = String::from("'outer");

            'outer: loop {
                for lifetime_def in self.lifetimes() {
                    if candidate == lifetime_def.lifetime.ident.to_string() {
                        candidate.push('_');
                        continue 'outer;
                    }
                }

                break;
            }

            Lifetime::new(candidate.as_str(), Span::call_site())
        };

        let mut function_generics = self.clone();

        function_generics.params.insert(
            0,
            GenericParam::Lifetime(LifetimeDef {
                attrs: Vec::new(),
                lifetime: code_lifetime.clone(),
                colon_token: None,
                bounds: Punctuated::new(),
            }),
        );

        GenericsSplit {
            node_generics,
            function_generics,
            code_lifetime,
            outer_lifetime,
        }
    }
}

pub(in crate::node) trait GenericsExt {
    fn to_split(&self) -> GenericsSplit;
}
