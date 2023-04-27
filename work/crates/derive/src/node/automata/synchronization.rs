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
use syn::spanned::Spanned;

use crate::{
    node::{automata::NodeAutomata, regex::terminal::Terminal},
    utils::debug_panic,
};

pub(in crate::node) struct Synchronization {
    variant_name: Ident,
    attribute_span: Span,
    open: Option<Ident>,
    close: Option<Ident>,
}

impl Spanned for Synchronization {
    #[inline(always)]
    fn span(&self) -> Span {
        self.attribute_span
    }
}

impl Synchronization {
    #[inline(always)]
    pub(in crate::node) fn variant_name(&self) -> &Ident {
        &self.variant_name
    }

    #[inline(always)]
    pub(in crate::node) fn open(&self) -> Option<&Ident> {
        self.open.as_ref()
    }

    #[inline(always)]
    pub(in crate::node) fn close(&self) -> Option<&Ident> {
        self.close.as_ref()
    }
}

impl AutomataSynchronization for NodeAutomata {
    fn synchronization(&self, variant_name: Ident, attribute_span: Span) -> Synchronization {
        enum Single<'a> {
            Vacant,
            Found(&'a Ident),
            Ambiguity,
        }

        let mut open = Single::Vacant;
        let mut close = Single::Vacant;

        for (from, through, to) in self.transitions() {
            let start = from == self.start();
            let end = self.finish().contains(to);

            if !start && !end {
                continue;
            }

            match through {
                Terminal::Null => debug_panic!("Automata with null transition."),

                Terminal::Token { name, .. } => {
                    if start {
                        match &open {
                            Single::Vacant => open = Single::Found(name),
                            Single::Found(..) => open = Single::Ambiguity,
                            Single::Ambiguity => (),
                        }
                    }

                    if end {
                        match &close {
                            Single::Vacant => close = Single::Found(name),
                            Single::Found(token) if *token != name => close = Single::Ambiguity,
                            _ => (),
                        }
                    }
                }

                Terminal::Node { .. } => {
                    if start {
                        open = Single::Ambiguity;
                    }

                    if end {
                        close = Single::Ambiguity;
                    }
                }
            }
        }

        let open = match open {
            Single::Found(token) => Some(token.clone()),
            _ => None,
        };

        let close = match close {
            Single::Found(token) => Some(token.clone()),
            _ => None,
        };

        Synchronization {
            variant_name,
            attribute_span,
            open,
            close,
        }
    }
}

pub(in crate::node) trait AutomataSynchronization {
    fn synchronization(&self, variant_name: Ident, span: Span) -> Synchronization;
}
