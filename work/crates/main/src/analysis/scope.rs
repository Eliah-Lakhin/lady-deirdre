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
    analysis::{AnalysisResult, Attr, AttrContext, Computable, Grammar},
    std::*,
    sync::SyncBuildHasher,
    syntax::NodeRef,
};

pub type ScopeAttr<N> = Attr<Scope<N>>;

pub struct Scope<N: Grammar> {
    pub scope_ref: NodeRef,
    _grammar: PhantomData<N>,
}

impl<N: Grammar> PartialEq for Scope<N> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.scope_ref.eq(&other.scope_ref)
    }
}

impl<N: Grammar> Eq for Scope<N> {}

impl<N: Grammar> Debug for Scope<N> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("Scope")
            .field("scope_ref", &self.scope_ref)
            .finish()
    }
}

impl<N: Grammar> Clone for Scope<N> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Grammar> Copy for Scope<N> {}

impl<N: Grammar> Default for Scope<N> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            scope_ref: NodeRef::nil(),
            _grammar: PhantomData,
        }
    }
}

impl<N: Grammar> Computable for Scope<N> {
    type Node = N;

    fn compute<S: SyncBuildHasher>(context: &mut AttrContext<Self::Node, S>) -> AnalysisResult<Self>
    where
        Self: Sized,
    {
        let node_ref = context.node_ref();
        let document = context.read_doc(node_ref.id)?;

        let Some(node) = node_ref.deref(document.deref()) else {
            return Ok(Self::default());
        };

        let parent_ref = node.parent_ref();

        let Some(parent) = parent_ref.deref(document.deref()) else {
            return Ok(Self::default());
        };

        if parent.is_scope() {
            return Ok(Self {
                scope_ref: parent_ref,
                _grammar: PhantomData,
            });
        }

        Ok(*parent.scope_attr()?.read(context)?.deref())
    }
}
