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
        Grammar,
        ScopeAttr,
        NIL_ATTR_REF,
    },
    std::*,
    sync::SyncBuildHasher,
    syntax::{Key, NodeRef},
};

#[repr(transparent)]
pub struct Signal<L: Lifecycle> {
    payload: L::Payload,
}

impl<L: Lifecycle> Debug for Signal<L>
where
    L::Payload: Debug,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("Signal")
            .field("payload", &self.payload)
            .finish()
    }
}

impl<L: Lifecycle> Drop for Signal<L> {
    fn drop(&mut self) {
        L::on_deregister(&self.payload);
    }
}

impl<L: Lifecycle> Signal<L> {
    #[inline(always)]
    pub fn trigger(payload: L::Payload) -> Self {
        let signal = Self { payload };

        L::on_register(&signal.payload);

        signal
    }
}

impl<L: Lifecycle<Payload = NodeRef>> Feature for Signal<L> {
    type Node = L::Node;

    #[inline(always)]
    fn new_uninitialized(node_ref: NodeRef) -> Self
    where
        Self: Sized,
    {
        Self { payload: node_ref }
    }

    #[inline(always)]
    fn initialize<S: SyncBuildHasher>(
        &mut self,
        _initializer: &mut FeatureInitializer<Self::Node, S>,
    ) {
        L::on_register(&self.payload);
    }

    #[inline(always)]
    fn invalidate<S: SyncBuildHasher>(&self, _invalidator: &mut FeatureInvalidator<Self::Node, S>) {
    }

    #[inline(always)]
    fn scope_attr(&self) -> AnalysisResult<&ScopeAttr<Self::Node>> {
        Err(AnalysisError::MissingScope)
    }
}

impl<L: Lifecycle<Payload = NodeRef>> AbstractFeature for Signal<L> {
    #[inline(always)]
    fn attr_ref(&self) -> &AttrRef {
        &NIL_ATTR_REF
    }

    #[inline(always)]
    fn feature(&self, _key: Key) -> AnalysisResult<&dyn AbstractFeature> {
        Err(AnalysisError::MissingFeature)
    }

    #[inline(always)]
    fn feature_keys(&self) -> &'static [&'static Key] {
        &[]
    }
}

pub trait Lifecycle: Send + Sync + 'static {
    type Node: Grammar;
    type Payload: Send + Sync + 'static;

    fn on_register(payload: &Self::Payload);

    fn on_deregister(payload: &Self::Payload);
}
