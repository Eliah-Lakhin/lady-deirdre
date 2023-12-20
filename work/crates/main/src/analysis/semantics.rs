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

use crate::{
    analysis::{
        AbstractFeature,
        AnalysisError,
        AnalysisResult,
        AttrRef,
        Feature,
        FeatureInitializer,
        FeatureInvalidator,
        ScopeAttr,
    },
    std::*,
    sync::SyncBuildHasher,
    syntax::{Key, NodeRef},
};

pub struct Semantics<F: Feature> {
    inner: Box<SemanticsInner<F>>,
}

impl<F: Feature> AbstractFeature for Semantics<F> {
    #[inline(always)]
    fn attr_ref(&self) -> &AttrRef {
        let Ok(inner) = self.get() else {
            static NIL_REF: AttrRef = AttrRef::nil();

            return &NIL_REF;
        };

        inner.attr_ref()
    }

    #[inline(always)]
    fn feature(&self, key: Key) -> AnalysisResult<&dyn AbstractFeature> {
        self.get()?.feature(key)
    }

    #[inline(always)]
    fn feature_keys(&self) -> &'static [&'static Key] {
        let Ok(inner) = self.get() else {
            return &[];
        };

        inner.feature_keys()
    }
}

impl<F: Feature> Feature for Semantics<F> {
    type Node = F::Node;

    fn new_uninitialized(node_ref: NodeRef) -> Self {
        Self {
            inner: Box::new(SemanticsInner::Uninit(node_ref)),
        }
    }

    fn initialize<S: SyncBuildHasher>(
        &mut self,
        initializer: &mut FeatureInitializer<Self::Node, S>,
    ) {
        let SemanticsInner::Uninit(node_ref) = self.inner.deref() else {
            return;
        };

        let node_ref = *node_ref;

        let mut feature = F::new_uninitialized(node_ref);

        feature.initialize(initializer);

        *self.inner = SemanticsInner::Init(feature);
    }

    fn invalidate<S: SyncBuildHasher>(&self, invalidator: &mut FeatureInvalidator<Self::Node, S>) {
        let SemanticsInner::Init(feature) = self.inner.deref() else {
            return;
        };

        feature.invalidate(invalidator);
    }

    fn scope_attr(&self) -> AnalysisResult<&ScopeAttr<Self::Node>> {
        self.get()?.scope_attr()
    }
}

impl<F: Feature> Semantics<F> {
    #[inline(always)]
    pub fn get(&self) -> AnalysisResult<&F> {
        let SemanticsInner::Init(feature) = self.inner.deref() else {
            return Err(AnalysisError::MissingDocument);
        };

        Ok(feature)
    }
}

enum SemanticsInner<F: Feature> {
    Uninit(NodeRef),
    Init(F),
}
