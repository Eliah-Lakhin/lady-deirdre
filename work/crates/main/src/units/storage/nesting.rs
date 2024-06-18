////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use crate::{
    report::ld_unreachable,
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
        unsafe { ld_unreachable!("An attempt to get unit layer branching value.") }
    }

    #[inline(always)]
    fn capacity<ChildLayer: Layer, N: Node>() -> ChildCount {
        unsafe { ld_unreachable!("An attempt to get unit layer capacity value.") }
    }

    #[inline(always)]
    fn descriptor() -> &'static LayerDescriptor {
        unsafe { ld_unreachable!("An attempt to get unit layer description.") }
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
