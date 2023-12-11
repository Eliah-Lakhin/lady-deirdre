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
    analysis::{AnalysisResult, AnalysisTask, Computable},
    report::debug_unreachable,
    std::*,
    sync::SyncBuildHasher,
    syntax::{Node, NodeRef},
};

pub(super) trait Memo: Send + Sync + 'static {
    fn memo_type_id(&self) -> TypeId;

    // Safety: `self` and `other` represent the same type.
    unsafe fn memo_eq(&self, other: &dyn Memo) -> bool;
}

impl<T: Eq + Send + Sync + 'static> Memo for T {
    #[inline(always)]
    fn memo_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    #[inline(always)]
    unsafe fn memo_eq(&self, other: &dyn Memo) -> bool {
        if self.memo_type_id() != other.memo_type_id() {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        let other = unsafe { &*(other as *const dyn Memo as *const T) };

        self.eq(other)
    }
}

pub(super) trait Function<N: Node, S: SyncBuildHasher>: Send + Sync + 'static {
    fn invoke(&self, task: &mut AnalysisTask<N, S>) -> AnalysisResult<Box<dyn Memo>>;
}

impl<N, T, S> Function<N, S> for fn(&mut AnalysisTask<N, S>) -> AnalysisResult<T>
where
    N: Node,
    T: Eq + Send + Sync + Sized + 'static,
    S: SyncBuildHasher,
{
    fn invoke(&self, task: &mut AnalysisTask<N, S>) -> AnalysisResult<Box<dyn Memo>> {
        Ok(Box::new(self(task)?))
    }
}

#[inline(always)]
pub(super) fn get_function<C, S>() -> &'static dyn Function<C::Node, S>
where
    C: Computable + Eq,
    S: SyncBuildHasher,
{
    &(C::compute as fn(&mut AnalysisTask<C::Node, S>) -> AnalysisResult<C>)
}
