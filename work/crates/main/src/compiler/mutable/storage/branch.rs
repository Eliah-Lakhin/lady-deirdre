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
    compiler::mutable::storage::{
        child::{ChildCount, ChildIndex, ChildRefIndex},
        item::{Item, ItemRef, ItemRefVariant, Split},
        nesting::{BranchLayer, Height, Layer, LayerDescriptor, PageLayer},
        references::References,
        utils::{array_copy_to, array_shift},
        BRANCH_B,
        BRANCH_CAP,
    },
    lexis::Length,
    report::{debug_assert, debug_unreachable},
    std::*,
    syntax::Node,
};

#[repr(transparent)]
pub(super) struct Branch<ChildLayer: Layer, N: Node> {
    pub(super) inner: BranchInner<N>,
    pub(super) child_layer: PhantomData<ChildLayer>,
}

pub(super) struct BranchInner<N: Node> {
    pub(super) parent: ChildRefIndex<N>,
    pub(super) occupied: ChildCount,
    pub(super) spans: [Length; BRANCH_CAP],
    pub(super) children: [ItemRefVariant<N>; BRANCH_CAP],
}

impl<ChildLayer: Layer, N: Node> Item for Branch<ChildLayer, N> {
    const B: ChildCount = BRANCH_B;
    const CAP: ChildCount = BRANCH_CAP;

    type Node = N;

    #[inline(always)]
    fn occupied(&self) -> ChildCount {
        self.inner.occupied
    }

    #[inline(always)]
    unsafe fn copy_to(
        &self,
        to: &mut Self,
        source: ChildCount,
        destination: ChildCount,
        count: ChildCount,
    ) {
        debug_assert!(
            source + count <= self.inner.occupied,
            "An attempt to copy non occupied data in Branch.",
        );

        unsafe {
            array_copy_to(
                &self.inner.spans,
                &mut to.inner.spans,
                source,
                destination,
                count,
            )
        };
        unsafe {
            array_copy_to(
                &self.inner.children,
                &mut to.inner.children,
                source,
                destination,
                count,
            )
        };
    }

    #[inline(always)]
    unsafe fn inflate(&mut self, from: ChildIndex, count: ChildCount) {
        debug_assert!(
            from <= self.inner.occupied,
            "An attempt to inflate from out of bounds child in Branch."
        );
        debug_assert!(
            count + self.inner.occupied <= Self::CAP,
            "An attempt to inflate with overflow in Branch."
        );
        debug_assert!(count > 0, "An attempt to inflate of empty range in Page.");

        if from < self.inner.occupied {
            unsafe {
                array_shift(
                    &mut self.inner.spans,
                    from,
                    from + count,
                    self.inner.occupied - from,
                )
            };
            unsafe {
                array_shift(
                    &mut self.inner.children,
                    from,
                    from + count,
                    self.inner.occupied - from,
                )
            };
        }

        self.inner.occupied += count;
    }

    #[inline(always)]
    unsafe fn deflate(&mut self, from: ChildIndex, count: ChildCount) -> bool {
        debug_assert!(
            from < self.inner.occupied,
            "An attempt to deflate from non occupied child in Branch."
        );
        debug_assert!(
            from + count <= self.inner.occupied,
            "An attempt to deflate with overflow in Branch."
        );
        debug_assert!(count > 0, "An attempt to deflate of empty range.");

        if from + count < self.inner.occupied {
            unsafe {
                array_shift(
                    &mut self.inner.spans,
                    from + count,
                    from,
                    self.inner.occupied - from - count,
                )
            };
            unsafe {
                array_shift(
                    &mut self.inner.children,
                    from + count,
                    from,
                    self.inner.occupied - from - count,
                )
            };
        }

        self.inner.occupied -= count;

        self.inner.occupied >= Self::B
    }
}

impl<ChildLayer: Layer, N: Node> Branch<ChildLayer, N> {
    #[inline(always)]
    pub(super) fn new(occupied: ChildCount) -> BranchRef<ChildLayer, N> {
        debug_assert!(
            occupied > 0,
            "An attempt to create Branch with zero occupied values."
        );

        debug_assert!(
            occupied <= Self::CAP,
            "An attempt to create Branch with occupied value exceeding capacity."
        );

        let branch = Self {
            inner: BranchInner {
                parent: ChildRefIndex::dangling(),
                occupied,
                spans: Default::default(),
                children: Default::default(),
            },
            child_layer: PhantomData::default(),
        };

        let pointer = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(branch))) };

        BranchRef { pointer }
    }

    // Safety:
    // 1. All references belong to `references` instance.
    // 2. `height >= 2`.
    // 3. `height` fits to `ChildLayer`.
    pub(crate) unsafe fn free_subtree(
        mut self,
        height: Height,
        references: &mut References<N>,
    ) -> ChildCount {
        let mut child_count = 0;

        for index in 0..self.inner.occupied {
            let child = unsafe { self.inner.children.get_unchecked_mut(index) };

            match height {
                0 | 1 => unsafe { debug_unreachable!("Incorrect height.") },

                2 => {
                    debug_assert!(
                        matches!(ChildLayer::descriptor(), LayerDescriptor::Page),
                        "Incorrect height.",
                    );

                    let page_ref = *unsafe { child.as_page_ref() };

                    let page = unsafe { page_ref.into_owned() };

                    child_count += unsafe { page.free_subtree(references) };
                }

                3 => {
                    debug_assert!(
                        matches!(ChildLayer::descriptor(), LayerDescriptor::Branch),
                        "Incorrect height.",
                    );

                    let branch_ref = *unsafe { child.as_branch_ref::<PageLayer>() };

                    let branch = unsafe { branch_ref.into_owned() };

                    child_count += unsafe { branch.free_subtree(height - 1, references) }
                }

                _ => {
                    debug_assert!(
                        matches!(ChildLayer::descriptor(), LayerDescriptor::Branch),
                        "Incorrect height.",
                    );

                    let branch_ref = *unsafe { child.as_branch_ref::<BranchLayer>() };

                    let branch = unsafe { branch_ref.into_owned() };

                    child_count += unsafe { branch.free_subtree(height - 1, references) }
                }
            }
        }

        child_count
    }

    // Safety:
    // 1. `height >= 2`.
    // 2. `height` fits to `ChildLayer`.
    pub(crate) unsafe fn free(mut self, height: Height) {
        for index in 0..self.inner.occupied {
            let child = unsafe { self.inner.children.get_unchecked_mut(index) };

            match height {
                0 | 1 => unsafe { debug_unreachable!("Incorrect height.") },

                2 => {
                    debug_assert!(
                        matches!(ChildLayer::descriptor(), LayerDescriptor::Page),
                        "Incorrect height.",
                    );

                    let page_ref = *unsafe { child.as_page_ref() };

                    let page = unsafe { page_ref.into_owned() };

                    unsafe { page.free() };
                }

                3 => {
                    debug_assert!(
                        matches!(ChildLayer::descriptor(), LayerDescriptor::Branch),
                        "Incorrect height.",
                    );

                    let branch_ref = *unsafe { child.as_branch_ref::<PageLayer>() };

                    let branch = unsafe { branch_ref.into_owned() };

                    unsafe { branch.free(height - 1) }
                }

                _ => {
                    debug_assert!(
                        matches!(ChildLayer::descriptor(), LayerDescriptor::Branch),
                        "Incorrect height.",
                    );

                    let branch_ref = *unsafe { child.as_branch_ref::<BranchLayer>() };

                    let branch = unsafe { branch_ref.into_owned() };

                    unsafe { branch.free(height - 1) }
                }
            }
        }
    }

    // Safety:
    // 1. `ChildLayer` correctly describes children layer.
    // 2. `count > 0`
    // 3. `self` data within `from..(from + count)` range is occupied.
    // 4. `self_variant` resolves to self pointer.
    #[inline(always)]
    unsafe fn update_children(
        &mut self,
        self_variant: ItemRefVariant<N>,
        from: ChildIndex,
        count: ChildCount,
    ) -> Length {
        let mut length = 0;

        for index in from..(from + count) {
            length += *unsafe { self.inner.spans.get_unchecked(index) };

            let child = unsafe { self.inner.children.get_unchecked_mut(index) };

            unsafe {
                child.set_parent::<ChildLayer>(ChildRefIndex {
                    item: self_variant,
                    index,
                })
            };
        }

        length
    }
}

#[repr(transparent)]
pub(super) struct BranchRef<ChildLayer: Layer, N: Node> {
    pointer: NonNull<Branch<ChildLayer, N>>,
}

impl<ChildLayer: Layer, N: Node> Clone for BranchRef<ChildLayer, N> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<ChildLayer: Layer, N: Node> Copy for BranchRef<ChildLayer, N> {}

impl<ChildLayer: Layer, N: Node> ItemRef<ChildLayer, N> for BranchRef<ChildLayer, N> {
    type SelfLayer = BranchLayer;

    type Item = Branch<ChildLayer, N>;

    #[inline(always)]
    fn dangling() -> Self {
        Self {
            pointer: NonNull::dangling(),
        }
    }

    #[inline(always)]
    unsafe fn as_ref(&self) -> &Self::Item {
        unsafe { self.pointer.as_ref() }
    }

    #[inline(always)]
    unsafe fn as_mut(&mut self) -> &mut Self::Item {
        unsafe { self.pointer.as_mut() }
    }

    #[inline(always)]
    unsafe fn into_variant(self) -> ItemRefVariant<N> {
        ItemRefVariant::from_branch(self)
    }

    #[inline(always)]
    unsafe fn into_owned(self) -> Box<Self::Item> {
        unsafe { Box::from_raw(self.pointer.as_ptr()) }
    }

    #[inline(always)]
    unsafe fn calculate_length(&self) -> Length {
        let branch = unsafe { self.as_ref() };

        let mut length = 0;

        for index in 0..branch.inner.occupied {
            length += unsafe { branch.inner.spans.get_unchecked(index) };
        }

        length
    }

    #[inline(always)]
    unsafe fn parent(&self) -> &ChildRefIndex<N> {
        unsafe { &self.as_ref().inner.parent }
    }

    #[inline(always)]
    unsafe fn set_parent(&mut self, parent: ChildRefIndex<N>) {
        unsafe { self.as_mut().inner.parent = parent };
    }

    #[inline(always)]
    unsafe fn parent_mut(&mut self) -> &mut BranchRef<BranchLayer, N> {
        let parent_ref_index = unsafe { &mut self.as_mut().inner.parent };

        debug_assert!(
            !parent_ref_index.is_dangling(),
            "An attempt to get parent from root.",
        );

        unsafe { parent_ref_index.item.as_branch_mut() }
    }

    #[inline(always)]
    unsafe fn update_children(
        &mut self,
        _references: &mut References<N>,
        from: ChildIndex,
        count: ChildCount,
    ) -> Length {
        let item = ItemRefVariant::from_branch(*self);

        let branch = unsafe { self.pointer.as_mut() };

        unsafe { branch.update_children(item, from, count) }
    }

    #[inline]
    unsafe fn split(
        &mut self,
        references: &mut References<N>,
        mut children_split: Split<N>,
        length: Length,
        from: ChildIndex,
    ) -> Split<N> {
        let mut parent_split = Split::dangling();

        let occupied = unsafe { self.as_ref().inner.occupied };

        debug_assert!(from < occupied, "Split at position out of bounds.",);

        match from == 0 {
            false => {
                let mut right_parent_ref = Branch::<ChildLayer, N>::new(occupied - from);

                match children_split.left_span == 0 {
                    false => match from + 1 == occupied {
                        false => {
                            let left_parent_variant = {
                                let left_parent_variant = unsafe { self.into_variant() };

                                unsafe {
                                    children_split.left_item.set_parent::<ChildLayer>(
                                        ChildRefIndex {
                                            item: left_parent_variant,
                                            index: from,
                                        },
                                    );
                                }

                                let left_parent = unsafe { self.as_mut() };

                                unsafe {
                                    left_parent.copy_to(
                                        right_parent_ref.as_mut(),
                                        from + 1,
                                        1,
                                        occupied - from - 1,
                                    )
                                };

                                unsafe {
                                    *left_parent.inner.spans.get_unchecked_mut(from) =
                                        children_split.left_span
                                };
                                unsafe {
                                    *left_parent.inner.children.get_unchecked_mut(from) =
                                        children_split.left_item
                                };

                                left_parent.inner.occupied = from + 1;

                                left_parent_variant
                            };

                            let right_parent_variant = {
                                let right_parent_variant =
                                    unsafe { right_parent_ref.into_variant() };

                                let right_parent = unsafe { right_parent_ref.as_mut() };

                                right_parent.inner.spans[0] = children_split.right_span;
                                right_parent.inner.children[0] = children_split.right_item;

                                right_parent_variant
                            };

                            let right_parent_span = unsafe {
                                right_parent_ref.update_children(references, 0, occupied - from)
                            };

                            parent_split.left_span = length - right_parent_span;
                            parent_split.left_item = left_parent_variant;
                            parent_split.right_span = right_parent_span;
                            parent_split.right_item = right_parent_variant;
                        }

                        true => {
                            let left_parent_variant = {
                                let left_parent_variant = unsafe { self.into_variant() };

                                unsafe {
                                    children_split.left_item.set_parent::<ChildLayer>(
                                        ChildRefIndex {
                                            item: left_parent_variant,
                                            index: from,
                                        },
                                    );
                                }

                                let left_parent = unsafe { self.as_mut() };

                                unsafe {
                                    *left_parent.inner.spans.get_unchecked_mut(from) =
                                        children_split.left_span
                                };
                                unsafe {
                                    *left_parent.inner.children.get_unchecked_mut(from) =
                                        children_split.left_item;
                                }

                                left_parent_variant
                            };

                            let right_parent_variant = {
                                let right_parent_variant =
                                    unsafe { right_parent_ref.into_variant() };

                                unsafe {
                                    children_split.right_item.set_parent::<ChildLayer>(
                                        ChildRefIndex {
                                            item: right_parent_variant,
                                            index: 0,
                                        },
                                    );
                                }

                                let right_parent = unsafe { right_parent_ref.as_mut() };

                                right_parent.inner.spans[0] = children_split.right_span;
                                right_parent.inner.children[0] = children_split.right_item;

                                right_parent_variant
                            };

                            parent_split.left_span = length - children_split.right_span;
                            parent_split.left_item = left_parent_variant;
                            parent_split.right_span = children_split.right_span;
                            parent_split.right_item = right_parent_variant;
                        }
                    },

                    true => {
                        let left_parent = unsafe { self.as_mut() };

                        unsafe {
                            left_parent.copy_to(right_parent_ref.as_mut(), from, 0, occupied - from)
                        };
                        left_parent.inner.occupied = from;

                        parent_split.right_span = unsafe {
                            right_parent_ref.update_children(references, 0, occupied - from)
                        };
                        parent_split.right_item = unsafe { right_parent_ref.into_variant() };

                        parent_split.left_span = length - parent_split.right_span;
                        parent_split.left_item = self.into_variant();
                    }
                }
            }

            true => match children_split.left_span == 0 {
                false => {
                    let left_parent_variant = {
                        let mut left_parent_ref = Branch::<ChildLayer, N>::new(1);
                        let left_parent_variant = unsafe { left_parent_ref.into_variant() };

                        unsafe {
                            children_split
                                .left_item
                                .set_parent::<ChildLayer>(ChildRefIndex {
                                    item: left_parent_variant,
                                    index: 0,
                                });
                        }

                        let left_parent = unsafe { left_parent_ref.as_mut() };

                        left_parent.inner.spans[0] = children_split.left_span;
                        left_parent.inner.children[0] = children_split.left_item;

                        left_parent_variant
                    };

                    let right_parent_variant = {
                        let right_parent_variant = unsafe { self.into_variant() };

                        unsafe {
                            children_split
                                .right_item
                                .set_parent::<ChildLayer>(ChildRefIndex {
                                    item: right_parent_variant,
                                    index: 0,
                                });
                        }

                        let right_parent = unsafe { self.as_mut() };

                        right_parent.inner.spans[0] = children_split.right_span;
                        right_parent.inner.children[0] = children_split.right_item;

                        right_parent_variant
                    };

                    parent_split.left_span = children_split.left_span;
                    parent_split.left_item = left_parent_variant;
                    parent_split.right_span = length - children_split.left_span;
                    parent_split.right_item = right_parent_variant;
                }

                true => {
                    parent_split.left_span = 0;

                    parent_split.right_span = length;
                    parent_split.right_item = unsafe { self.into_variant() };
                }
            },
        }

        parent_split
    }
}

impl<ChildLayer: Layer, N: Node> BranchRef<ChildLayer, N> {
    // Safety:
    // 1. `self` is not dangling.
    // 2. `ChildLayer` correctly describes children later of `self`.
    // 3. `GrandchildLayer` correctly describes children later of the `ChildLayer`.
    // 4. All references inside `self` subtree belong to `references` instance.
    #[inline]
    pub(super) unsafe fn fix_leftmost_balance<GrandchildLayer: Layer>(
        &mut self,
        references: &mut References<N>,
    ) -> (bool, ItemRefVariant<N>) {
        let parent_occupied = unsafe { self.as_ref().occupied() };

        match parent_occupied {
            0 => unsafe { debug_unreachable!("Empty item.") },

            1 => (true, unsafe { self.as_ref().inner.children[0] }),

            _ => {
                let mut first_child_variant = unsafe { self.as_ref().inner.children[0] };

                let first_child_occupied = match ChildLayer::descriptor() {
                    LayerDescriptor::Branch => unsafe {
                        first_child_variant
                            .as_branch_ref::<GrandchildLayer>()
                            .as_ref()
                            .occupied()
                    },

                    LayerDescriptor::Page => unsafe {
                        first_child_variant.as_page_ref().as_ref().occupied()
                    },
                };

                if first_child_occupied >= ChildLayer::branching::<GrandchildLayer, N>() {
                    return (true, first_child_variant);
                }

                let mut next_child_variant = unsafe { self.as_ref().inner.children[1] };

                let next_child_occupied = match ChildLayer::descriptor() {
                    LayerDescriptor::Branch => unsafe {
                        next_child_variant
                            .as_branch_ref::<GrandchildLayer>()
                            .as_ref()
                            .occupied()
                    },

                    LayerDescriptor::Page => unsafe {
                        next_child_variant.as_page_ref().as_ref().occupied()
                    },
                };

                if first_child_occupied + next_child_occupied
                    <= ChildLayer::capacity::<GrandchildLayer, N>()
                {
                    let addition = match ChildLayer::descriptor() {
                        LayerDescriptor::Branch => {
                            let first_child_ref =
                                unsafe { first_child_variant.as_branch_mut::<GrandchildLayer>() };

                            let next_child_ref =
                                unsafe { next_child_variant.as_branch_mut::<GrandchildLayer>() };

                            unsafe {
                                ItemRef::merge_to_right(first_child_ref, next_child_ref, references)
                            }
                        }

                        LayerDescriptor::Page => {
                            let first_child_ref = unsafe { first_child_variant.as_page_mut() };

                            let next_child_ref = unsafe { next_child_variant.as_page_mut() };

                            unsafe {
                                ItemRef::merge_to_right(first_child_ref, next_child_ref, references)
                            }
                        }
                    };

                    let parent_variant = unsafe { self.into_variant() };

                    let parent = unsafe { self.as_mut() };

                    parent.inner.spans[1] += addition;

                    let balanced = unsafe { parent.deflate(0, 1) };

                    let _ = unsafe { parent.update_children(parent_variant, 0, parent.occupied()) };

                    return (balanced, next_child_variant);
                }

                let transfer_length = match ChildLayer::descriptor() {
                    LayerDescriptor::Branch => {
                        let first_child_ref =
                            unsafe { first_child_variant.as_branch_mut::<GrandchildLayer>() };

                        let next_child_ref =
                            unsafe { next_child_variant.as_branch_mut::<GrandchildLayer>() };

                        unsafe {
                            ItemRef::balance_to_left(first_child_ref, next_child_ref, references)
                        }
                    }

                    LayerDescriptor::Page => {
                        let first_child_ref = unsafe { first_child_variant.as_page_mut() };

                        let next_child_ref = unsafe { next_child_variant.as_page_mut() };

                        unsafe {
                            ItemRef::balance_to_left(first_child_ref, next_child_ref, references)
                        }
                    }
                };

                let parent = unsafe { self.as_mut() };

                unsafe { parent.inner.spans[0] += transfer_length };
                unsafe { parent.inner.spans[1] -= transfer_length };

                (true, first_child_variant)
            }
        }
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `ChildLayer` correctly describes children later of `self`.
    // 3. `GrandchildLayer` correctly describes children later of the `ChildLayer`.
    // 4. All references inside `self` subtree belong to `references` instance.
    #[inline]
    pub(super) unsafe fn fix_rightmost_balance<GrandchildLayer: Layer>(
        &mut self,
        references: &mut References<N>,
    ) -> (bool, ItemRefVariant<N>) {
        let parent_occupied = unsafe { self.as_ref().occupied() };

        match parent_occupied {
            0 => unsafe { debug_unreachable!("Empty item.") },

            1 => (true, unsafe { self.as_ref().inner.children[0] }),

            _ => {
                let mut last_child_variant = unsafe {
                    *self
                        .as_ref()
                        .inner
                        .children
                        .get_unchecked(parent_occupied - 1)
                };

                let last_child_occupied = match ChildLayer::descriptor() {
                    LayerDescriptor::Branch => unsafe {
                        last_child_variant
                            .as_branch_ref::<GrandchildLayer>()
                            .as_ref()
                            .occupied()
                    },

                    LayerDescriptor::Page => unsafe {
                        last_child_variant.as_page_ref().as_ref().occupied()
                    },
                };

                if last_child_occupied >= ChildLayer::branching::<GrandchildLayer, N>() {
                    return (true, last_child_variant);
                }

                let mut previous_child_variant = unsafe {
                    *self
                        .as_ref()
                        .inner
                        .children
                        .get_unchecked(parent_occupied - 2)
                };

                let previous_child_occupied = match ChildLayer::descriptor() {
                    LayerDescriptor::Branch => unsafe {
                        previous_child_variant
                            .as_branch_ref::<GrandchildLayer>()
                            .as_ref()
                            .occupied()
                    },

                    LayerDescriptor::Page => unsafe {
                        previous_child_variant.as_page_ref().as_ref().occupied()
                    },
                };

                if previous_child_occupied + last_child_occupied
                    <= ChildLayer::capacity::<GrandchildLayer, N>()
                {
                    let addition = match ChildLayer::descriptor() {
                        LayerDescriptor::Branch => {
                            let previous_child_ref = unsafe {
                                previous_child_variant.as_branch_mut::<GrandchildLayer>()
                            };

                            let last_child_ref =
                                unsafe { last_child_variant.as_branch_mut::<GrandchildLayer>() };

                            unsafe {
                                ItemRef::merge_to_left(
                                    previous_child_ref,
                                    last_child_ref,
                                    references,
                                )
                            }
                        }

                        LayerDescriptor::Page => {
                            let previous_child_ref =
                                unsafe { previous_child_variant.as_page_mut() };

                            let last_child_ref = unsafe { last_child_variant.as_page_mut() };

                            unsafe {
                                ItemRef::merge_to_left(
                                    previous_child_ref,
                                    last_child_ref,
                                    references,
                                )
                            }
                        }
                    };

                    let parent = unsafe { self.as_mut() };

                    unsafe {
                        *parent.inner.spans.get_unchecked_mut(parent_occupied - 2) += addition
                    };

                    parent.inner.occupied -= 1;

                    return (
                        parent.inner.occupied >= Branch::<ChildLayer, N>::B,
                        previous_child_variant,
                    );
                }

                let transfer_length = match ChildLayer::descriptor() {
                    LayerDescriptor::Branch => {
                        let previous_child_ref =
                            unsafe { previous_child_variant.as_branch_mut::<GrandchildLayer>() };

                        let last_child_ref =
                            unsafe { last_child_variant.as_branch_mut::<GrandchildLayer>() };

                        unsafe {
                            ItemRef::balance_to_right(
                                previous_child_ref,
                                last_child_ref,
                                references,
                            )
                        }
                    }

                    LayerDescriptor::Page => {
                        let previous_child_ref = unsafe { previous_child_variant.as_page_mut() };

                        let last_child_ref = unsafe { last_child_variant.as_page_mut() };

                        unsafe {
                            ItemRef::balance_to_right(
                                previous_child_ref,
                                last_child_ref,
                                references,
                            )
                        }
                    }
                };

                let parent = unsafe { self.as_mut() };

                unsafe {
                    *parent.inner.spans.get_unchecked_mut(parent_occupied - 1) += transfer_length
                };

                unsafe {
                    *parent.inner.spans.get_unchecked_mut(parent_occupied - 2) -= transfer_length
                };

                (true, last_child_variant)
            }
        }
    }

    // Safety:
    // 1. `self` is not dangling.
    #[inline]
    pub(super) unsafe fn inc_span_left(&mut self, addition: Length) {
        let mut branch = unsafe { self.as_mut() };

        loop {
            branch.inner.spans[0] += addition;

            match branch.inner.parent.is_dangling() {
                true => break,

                false => {
                    branch = unsafe { branch.inner.parent.item.as_branch_mut().as_mut() };
                }
            }
        }
    }

    // Safety:
    // 1. `self` is not dangling.
    #[inline]
    pub(super) unsafe fn inc_span_right(&mut self, addition: Length) {
        let mut branch = unsafe { self.as_mut() };

        loop {
            unsafe {
                *branch
                    .inner
                    .spans
                    .get_unchecked_mut(branch.inner.occupied - 1) += addition
            };

            match branch.inner.parent.is_dangling() {
                true => break,

                false => {
                    branch = unsafe { branch.inner.parent.item.as_branch_mut().as_mut() };
                }
            }
        }
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `item_variant` is not dangling.
    // 3. `ChildLayer` correctly describes `item_variant` type, and the `self` children layer.
    // 4. `self` Branch is not a root branch.
    #[inline]
    pub(super) unsafe fn add_child_left(
        mut self,
        root_length: Length,
        mut head_subtraction: Length,
        mut item_length: Length,
        mut item_variant: ItemRefVariant<N>,
    ) -> Option<ItemRefVariant<N>> {
        loop {
            let this = self;
            let branch = unsafe { self.as_mut() };

            branch.inner.spans[0] -= head_subtraction;

            match branch.inner.occupied < Branch::<ChildLayer, N>::CAP {
                true => {
                    let branch_variant = ItemRefVariant::from_branch(this);

                    let parent_ref_index = ChildRefIndex {
                        item: branch_variant,
                        index: 0,
                    };

                    match ChildLayer::descriptor() {
                        LayerDescriptor::Page => unsafe {
                            item_variant.as_page_mut().set_parent(parent_ref_index)
                        },

                        LayerDescriptor::Branch => unsafe {
                            item_variant
                                .as_branch_mut::<()>()
                                .set_parent(parent_ref_index)
                        },
                    }

                    unsafe { branch.inflate(0, 1) };

                    branch.inner.children[0] = item_variant;
                    branch.inner.spans[0] = item_length;

                    unsafe {
                        let _ =
                            branch.update_children(branch_variant, 1, branch.inner.occupied - 1);
                    }

                    if !branch.inner.parent.is_dangling() {
                        let parent =
                            unsafe { branch.inner.parent.item.as_branch_mut::<BranchLayer>() };

                        unsafe { parent.inc_span_left(item_length - head_subtraction) };
                    }

                    break;
                }

                false => {
                    let mut new_sibling_ref = Branch::new(Branch::<ChildLayer, N>::B);
                    let new_sibling_variant = unsafe { new_sibling_ref.into_variant() };
                    let transfer_length;

                    {
                        let new_sibling = unsafe { new_sibling_ref.as_mut() };

                        unsafe { branch.copy_to(new_sibling, 0, 1, Branch::<ChildLayer, N>::B - 1) }

                        transfer_length = unsafe {
                            new_sibling.update_children(
                                new_sibling_variant,
                                1,
                                Branch::<ChildLayer, N>::B - 1,
                            )
                        };

                        new_sibling.inner.spans[0] = item_length;
                        new_sibling.inner.children[0] = item_variant;

                        unsafe {
                            item_variant.set_parent::<ChildLayer>(ChildRefIndex {
                                item: new_sibling_variant,
                                index: 0,
                            });
                        }
                    }

                    unsafe {
                        let _ = branch.deflate(0, Branch::<ChildLayer, N>::B - 1);
                    }

                    let branch_variant = unsafe { this.into_variant() };

                    let _ = unsafe {
                        branch.update_children(branch_variant, 0, Branch::<ChildLayer, N>::B)
                    };

                    item_length += transfer_length;
                    item_variant = new_sibling_variant;
                    head_subtraction += transfer_length;

                    match branch.inner.parent.is_dangling() {
                        false => match ChildLayer::descriptor() {
                            LayerDescriptor::Branch => {
                                self = *branch.inner.parent.item.as_branch_mut::<ChildLayer>();
                                continue;
                            }

                            LayerDescriptor::Page => {
                                return unsafe {
                                    branch
                                        .inner
                                        .parent
                                        .item
                                        .as_branch_mut::<BranchLayer>()
                                        .add_child_left(
                                            root_length,
                                            head_subtraction,
                                            item_length,
                                            item_variant,
                                        )
                                };
                            }
                        },

                        true => {
                            let mut new_root_ref = Branch::<BranchLayer, N>::new(2);

                            let new_root_variant = unsafe { new_root_ref.into_variant() };

                            unsafe {
                                new_sibling_ref.set_parent(ChildRefIndex {
                                    item: new_root_variant,
                                    index: 0,
                                });
                            }

                            branch.inner.parent = ChildRefIndex {
                                item: new_root_variant,
                                index: 1,
                            };

                            {
                                let new_root = unsafe { new_root_ref.as_mut() };

                                new_root.inner.children[0] = new_sibling_variant;
                                new_root.inner.children[1] = ItemRefVariant::from_branch(this);

                                new_root.inner.spans[0] = item_length;
                                new_root.inner.spans[1] = root_length - head_subtraction;
                            }

                            return Some(new_root_variant);
                        }
                    }
                }
            }
        }

        return None;
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `item_variant` is not dangling.
    // 3. `ChildLayer` correctly describes `item_variant` type, and the `self` children layer.
    // 4. `self` Branch is not a root branch.
    #[inline]
    pub(super) unsafe fn add_child_right(
        mut self,
        root_length: Length,
        mut tail_subtraction: Length,
        mut item_length: Length,
        mut item_variant: ItemRefVariant<N>,
    ) -> Option<ItemRefVariant<N>> {
        loop {
            let this = self;
            let branch = unsafe { self.as_mut() };

            unsafe {
                *branch
                    .inner
                    .spans
                    .get_unchecked_mut(branch.inner.occupied - 1) -= tail_subtraction
            };

            match branch.inner.occupied < Branch::<ChildLayer, N>::CAP {
                true => {
                    let parent_ref_index = ChildRefIndex {
                        item: ItemRefVariant::from_branch(this),
                        index: branch.inner.occupied,
                    };

                    match ChildLayer::descriptor() {
                        LayerDescriptor::Page => unsafe {
                            item_variant.as_page_mut().set_parent(parent_ref_index)
                        },

                        LayerDescriptor::Branch => unsafe {
                            item_variant
                                .as_branch_mut::<()>()
                                .set_parent(parent_ref_index)
                        },
                    }

                    unsafe {
                        *branch
                            .inner
                            .children
                            .get_unchecked_mut(branch.inner.occupied) = item_variant;
                    }

                    unsafe {
                        *branch.inner.spans.get_unchecked_mut(branch.inner.occupied) = item_length;
                    }

                    branch.inner.occupied += 1;

                    if !branch.inner.parent.is_dangling() {
                        let parent =
                            unsafe { branch.inner.parent.item.as_branch_mut::<BranchLayer>() };

                        unsafe { parent.inc_span_right(item_length - tail_subtraction) };
                    }

                    break;
                }

                false => {
                    let mut new_sibling_ref = Branch::new(Branch::<ChildLayer, N>::B);
                    let new_sibling_variant = unsafe { new_sibling_ref.into_variant() };
                    let transfer_length;

                    {
                        let new_sibling = unsafe { new_sibling_ref.as_mut() };

                        unsafe {
                            branch.copy_to(
                                new_sibling,
                                Branch::<ChildLayer, N>::B,
                                0,
                                Branch::<ChildLayer, N>::B - 1,
                            )
                        }

                        transfer_length = unsafe {
                            new_sibling.update_children(
                                new_sibling_variant,
                                0,
                                Branch::<ChildLayer, N>::B - 1,
                            )
                        };

                        new_sibling.inner.spans[Branch::<ChildLayer, N>::B - 1] = item_length;
                        new_sibling.inner.children[Branch::<ChildLayer, N>::B - 1] = item_variant;

                        unsafe {
                            item_variant.set_parent::<ChildLayer>(ChildRefIndex {
                                item: new_sibling_variant,
                                index: Branch::<ChildLayer, N>::B - 1,
                            });
                        }
                    }

                    branch.inner.occupied = Branch::<ChildLayer, N>::B;

                    item_length += transfer_length;
                    item_variant = new_sibling_variant;
                    tail_subtraction += transfer_length;

                    match branch.inner.parent.is_dangling() {
                        false => match ChildLayer::descriptor() {
                            LayerDescriptor::Branch => {
                                self = *branch.inner.parent.item.as_branch_mut::<ChildLayer>();
                                continue;
                            }

                            LayerDescriptor::Page => {
                                return unsafe {
                                    branch
                                        .inner
                                        .parent
                                        .item
                                        .as_branch_mut::<BranchLayer>()
                                        .add_child_right(
                                            root_length,
                                            tail_subtraction,
                                            item_length,
                                            item_variant,
                                        )
                                };
                            }
                        },

                        true => {
                            let mut new_root_ref = Branch::<BranchLayer, N>::new(2);

                            let new_root_variant = unsafe { new_root_ref.into_variant() };

                            branch.inner.parent = ChildRefIndex {
                                item: new_root_variant,
                                index: 0,
                            };

                            unsafe {
                                new_sibling_ref.set_parent(ChildRefIndex {
                                    item: new_root_variant,
                                    index: 1,
                                });
                            }

                            {
                                let new_root = unsafe { new_root_ref.as_mut() };

                                new_root.inner.children[0] = ItemRefVariant::from_branch(this);
                                new_root.inner.children[1] = new_sibling_variant;

                                new_root.inner.spans[0] = root_length - tail_subtraction;
                                new_root.inner.spans[1] = item_length;
                            }

                            return Some(new_root_variant);
                        }
                    }
                }
            }
        }

        return None;
    }
}
