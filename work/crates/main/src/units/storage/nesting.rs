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
    report::debug_unreachable,
    syntax::Node,
    units::storage::{branch::Branch, child::ChildCount, item::Item, page::Page},
};

pub(super) type Height = usize;

// Safety: `Layer` is implemented for zero-sized and 'static types only.
pub(super) unsafe trait Layer {
    fn branching<ChildLayer: Layer, N: Node>() -> ChildCount;

    fn capacity<ChildLayer: Layer, N: Node>() -> ChildCount;

    fn descriptor() -> &'static LayerDescriptor;
}

unsafe impl Layer for () {
    #[inline(always)]
    fn branching<ChildLayer: Layer, N: Node>() -> ChildCount {
        unsafe { debug_unreachable!("An attempt to get unit layer branching value.") }
    }

    #[inline(always)]
    fn capacity<ChildLayer: Layer, N: Node>() -> ChildCount {
        unsafe { debug_unreachable!("An attempt to get unit layer capacity value.") }
    }

    #[inline(always)]
    fn descriptor() -> &'static LayerDescriptor {
        unsafe { debug_unreachable!("An attempt to get unit layer description.") }
    }
}

pub(super) struct BranchLayer;

unsafe impl Layer for BranchLayer {
    #[inline(always)]
    fn branching<ChildLayer: Layer, N: Node>() -> ChildCount {
        Branch::<ChildLayer, N>::B
    }

    #[inline(always)]
    fn capacity<ChildLayer: Layer, N: Node>() -> ChildCount {
        Branch::<ChildLayer, N>::CAP
    }

    #[inline(always)]
    fn descriptor() -> &'static LayerDescriptor {
        static BRANCH: LayerDescriptor = LayerDescriptor::Branch;

        &BRANCH
    }
}

pub(super) struct PageLayer;

unsafe impl Layer for PageLayer {
    #[inline(always)]
    fn branching<ChildLayer: Layer, N: Node>() -> ChildCount {
        Page::<N>::B
    }

    #[inline(always)]
    fn capacity<ChildLayer: Layer, N: Node>() -> ChildCount {
        Page::<N>::CAP
    }

    #[inline(always)]
    fn descriptor() -> &'static LayerDescriptor {
        static PAGE: LayerDescriptor = LayerDescriptor::Page;

        &PAGE
    }
}

pub(super) enum LayerDescriptor {
    Branch,
    Page,
}
