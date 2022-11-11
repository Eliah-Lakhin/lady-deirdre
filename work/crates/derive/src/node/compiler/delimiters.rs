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

use proc_macro2::Ident;

use crate::{
    node::builder::{kind::VariantKind, variant::NodeVariant, Builder},
    utils::{Map, PredictableCollection},
};

#[derive(Clone, Default)]
pub(in crate::node) struct PanicDelimiters<'a> {
    single: Option<&'a Ident>,
    local: Option<(&'a Ident, &'a Ident)>,
    global: Map<&'a Ident, SynchronizationAction>,
}

impl<'a> PanicDelimiters<'a> {
    #[inline(always)]
    pub(in crate::node) fn single(&self) -> Option<&'a Ident> {
        self.single
    }

    #[inline(always)]
    pub(in crate::node) fn local(&self) -> Option<(&'a Ident, &'a Ident)> {
        self.local
    }

    #[inline(always)]
    pub(in crate::node) fn global(&self) -> &Map<&'a Ident, SynchronizationAction> {
        &self.global
    }

    pub(in crate::node) fn new(variant: &'a NodeVariant, builder: &'a Builder) -> Self {
        let mut single;
        let mut local;

        match variant.kind() {
            VariantKind::Unspecified(..) | VariantKind::Root(..) => {
                single = None;
                local = None;
            }
            VariantKind::Comment(..) | VariantKind::Sentence(..) => {
                let variant_local = variant.synchronization();

                single = variant_local.close();

                local = match (variant_local.open(), variant_local.close()) {
                    (Some(open), Some(close)) => Some((open, close)),
                    _ => None,
                };
            }
        };

        let mut global;

        match variant.kind() {
            VariantKind::Unspecified(..) | VariantKind::Comment(..) => {
                global = Map::empty();
                local = None;
            }

            VariantKind::Root(..) | VariantKind::Sentence(..) => {
                let synchronization_map = builder.synchronization();

                let mut local_found = false;
                let mut states = 1..;
                global = Map::with_capacity(synchronization_map.len() * 2);

                for (from, to) in synchronization_map {
                    if Some((from, to)) == local {
                        local_found = true;
                    }

                    if single == Some(from) || single == Some(to) {
                        single = None;
                    }

                    let state = states
                        .next()
                        .expect("Internal error. State generate exceeded.");

                    let outer = match &single {
                        Some(delimiter) if *delimiter == from => false,

                        _ => true,
                    };

                    let _ = global.insert(from, SynchronizationAction::Push { state, outer });

                    let _ = global.insert(to, SynchronizationAction::Pop { state, outer: true });
                }

                if !local_found {
                    local = None;
                }
            }
        };

        Self {
            single,
            local,
            global,
        }
    }
}

#[derive(Clone)]
pub(in crate::node) enum SynchronizationAction {
    Push { state: usize, outer: bool },
    Pop { state: usize, outer: bool },
}
