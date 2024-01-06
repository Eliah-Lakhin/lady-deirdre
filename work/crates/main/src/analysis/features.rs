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

extern crate lady_deirdre_derive;

pub use lady_deirdre_derive::Feature;

use crate::{
    analysis::{
        database::AbstractDatabase,
        record::Record,
        AnalysisResult,
        AttrRef,
        Computable,
        Grammar,
        ScopeAttr,
    },
    arena::{Entry, Id, Identifiable, Repository},
    std::*,
    sync::SyncBuildHasher,
    syntax::{Key, NodeRef},
};

pub trait Feature: AbstractFeature {
    type Node: Grammar;

    fn new_uninitialized(node_ref: NodeRef) -> Self
    where
        Self: Sized;

    fn initialize<S: SyncBuildHasher>(
        &mut self,
        initializer: &mut FeatureInitializer<Self::Node, S>,
    );

    fn invalidate<S: SyncBuildHasher>(&self, invalidator: &mut FeatureInvalidator<Self::Node, S>);

    fn scope_attr(&self) -> AnalysisResult<&ScopeAttr<Self::Node>>;
}

pub trait AbstractFeature {
    fn attr_ref(&self) -> &AttrRef;

    fn feature(&self, key: Key) -> AnalysisResult<&dyn AbstractFeature>;

    fn feature_keys(&self) -> &'static [&'static Key];
}

pub struct FeatureInitializer<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    pub(super) id: Id,
    pub(super) database: Weak<dyn AbstractDatabase>,
    pub(super) records: &'a mut Repository<Record<N, S>>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Identifiable for FeatureInitializer<'a, N, S> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> FeatureInitializer<'a, N, S> {
    #[inline(always)]
    pub(super) fn register_attribute<C: Computable<Node = N> + Eq>(
        &mut self,
        node_ref: NodeRef,
    ) -> (Weak<dyn AbstractDatabase>, Entry) {
        (
            self.database.clone(),
            self.records.insert(Record::new::<C>(node_ref)),
        )
    }
}

pub struct FeatureInvalidator<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    pub(super) id: Id,
    pub(super) records: &'a mut Repository<Record<N, S>>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Identifiable for FeatureInvalidator<'a, N, S> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> FeatureInvalidator<'a, N, S> {
    #[inline(always)]
    pub(super) fn invalidate_attribute(&mut self, entry: &Entry) {
        let Some(record) = self.records.get(entry) else {
            return;
        };

        record.invalidate();
    }
}
