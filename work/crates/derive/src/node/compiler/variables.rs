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

use crate::{
    node::{
        automata::variables::{VariableKind, VariableMap, VariableMeta, VariableRepetition},
        compiler::Compiler,
        regex::terminal::Terminal,
    },
    utils::{debug_panic, Facade},
};

impl VariableMap {
    pub(in crate::node) fn init(&self, compiler: &Compiler<'_>) -> TokenStream {
        let variables = self.into_iter().map(|name| {
            let meta = self.get(name);
            let capture_name = meta.capture_name();

            let core = compiler.facade().core_crate();

            match (&meta.kind(), &meta.repetition()) {
                (
                    VariableKind::TokenRef,
                    VariableRepetition::Single | VariableRepetition::Optional,
                ) => {
                    quote! {
                        let mut #capture_name = #core::lexis::TokenRef::nil();
                    }
                }

                (
                    VariableKind::NodeRef,
                    VariableRepetition::Single | VariableRepetition::Optional,
                ) => {
                    quote! {
                        let mut #capture_name = #core::syntax::NodeRef::nil();
                    }
                }

                (VariableKind::TokenRef, VariableRepetition::Multiple) => {
                    let vec = compiler.facade().vec();

                    quote! {
                        let mut #capture_name = #vec::<#core::lexis::TokenRef>::with_capacity(1);
                    }
                }

                (VariableKind::NodeRef, VariableRepetition::Multiple) => {
                    let vec = compiler.facade().vec();

                    quote! {
                        let mut #capture_name = #vec::<#core::syntax::NodeRef>::with_capacity(1);
                    }
                }
            }
        });

        quote! {
            #( #variables )*
        }
    }
}

impl VariableMeta {
    pub(in crate::node) fn insert(
        &self,
        facade: &Facade,
        terminal: &Terminal,
    ) -> Option<TokenStream> {
        let variable = self.capture_name();

        match self.repetition() {
            VariableRepetition::Single | VariableRepetition::Optional => None,

            VariableRepetition::Multiple => {
                let core = facade.core_crate();
                let vec = facade.vec();

                match terminal {
                    Terminal::Null => debug_panic!("Automata with null transition."),

                    Terminal::Token { .. } => Some(quote! {
                        #vec::push(&mut #variable, #core::lexis::TokenRef::nil());
                    }),

                    Terminal::Node { .. } => Some(quote! {
                        #vec::push(&mut #variable, #core::syntax::NodeRef::nil());
                    }),
                }
            }
        }
    }

    pub(in crate::node) fn write(&self, facade: &Facade, value: TokenStream) -> TokenStream {
        let variable = self.capture_name();

        match self.repetition() {
            VariableRepetition::Single | VariableRepetition::Optional => {
                quote! {
                    #variable = #value;
                }
            }

            VariableRepetition::Multiple => {
                let vec = facade.vec();

                quote! {
                    #vec::push(&mut #variable, #value);
                }
            }
        }
    }

    pub(in crate::node) fn read(&self) -> TokenStream {
        let variable = self.capture_name();

        quote! { #variable }
    }

    #[inline(always)]
    fn capture_name(&self) -> Ident {
        Ident::new(&format!("capture_{}", self.name()), self.name().span())
    }
}
