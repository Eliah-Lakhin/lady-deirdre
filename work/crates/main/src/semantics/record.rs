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
    arena::EntryVersion,
    report::debug_unreachable,
    semantics::{AttrContext, AttrRef, AttrResult},
    std::*,
    syntax::NodeRef,
};

pub(super) struct Record {
    pub(super) verified_at: EntryVersion,
    pub(super) cache: Option<Cache>,
    pub(super) node_ref: NodeRef,
    pub(super) function: &'static dyn Function,
}

impl Record {
    #[inline(always)]
    pub(super) fn new<T: Eq + Send + Sync + 'static>(
        node_ref: NodeRef,
        function: &'static (impl Fn(&mut AttrContext) -> AttrResult<T> + Send + Sync + 'static),
    ) -> Self {
        Self {
            verified_at: 0,
            cache: None,
            node_ref,
            function,
        }
    }
}

pub(super) struct Cache {
    pub(super) dirty: bool,
    pub(super) updated_at: EntryVersion,
    pub(super) memo: Box<dyn Memo>,
    pub(super) deps: StdSet<AttrRef>,
}

impl Cache {
    // Safety: `T` properly describes `memo` type.
    #[inline(always)]
    pub(super) unsafe fn downcast_ref<T: 'static>(&self) -> &T {
        if self.memo.memo_type_id() != TypeId::of::<T>() {
            // Safety: Upheld by the caller.
            unsafe { debug_unreachable!("Incorrect memo type.") }
        }

        // Safety: Upheld by the caller.
        unsafe { &*(self.memo.deref() as *const dyn Memo as *const T) }
    }
}

pub(super) trait Memo: Send + Sync + 'static {
    fn memo_type_id(&self) -> TypeId;

    // Safety: `self` and `other` represent the same types.
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

pub(super) trait Function: Send + Sync + 'static {
    fn invoke(&self, context: &mut AttrContext) -> AttrResult<Box<dyn Memo>>;
}

impl<T, F> Function for F
where
    T: Eq + Send + Sync + 'static,
    F: Fn(&mut AttrContext) -> AttrResult<T>,
    F: Send + Sync + 'static,
{
    #[inline(always)]
    fn invoke(&self, context: &mut AttrContext) -> AttrResult<Box<dyn Memo>> {
        Ok(Box::new(self(context)?))
    }
}
