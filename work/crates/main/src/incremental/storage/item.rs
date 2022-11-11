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
    incremental::storage::{
        branch::{Branch, BranchRef},
        child::{ChildCount, ChildIndex, ChildRefIndex},
        nesting::{BranchLayer, Layer, LayerDescriptor},
        page::PageRef,
        references::References,
        utils::capacity,
    },
    lexis::Length,
    std::*,
    syntax::Node,
};

pub(super) trait Item: Sized {
    const BRANCHING: ChildCount;

    type Node: Node;

    fn occupied(&self) -> ChildCount;

    // Safety:
    // 1. `self` data within `source..(source + count)` range is occupied.
    // 2. `destination..(destination + count)` range is withing the `to` data capacity.
    unsafe fn copy_to(
        &mut self,
        to: &mut Self,
        source: ChildCount,
        destination: ChildCount,
        count: ChildCount,
    );

    // Safety:
    // 1. `from <= self.occupied`.
    // 2. `self.occupied + count <= capacity`.
    // 3. `count > 0`
    unsafe fn inflate(&mut self, from: ChildIndex, count: ChildCount);

    // Safety:
    // 1. `from < self.occupied`.
    // 2. `from + count <= self.occupied.
    // 3. `count > 0`
    unsafe fn deflate(&mut self, from: ChildIndex, count: ChildCount) -> bool;
}

pub(super) trait ItemRef<ChildLayer: Layer, N: Node>: Copy {
    type SelfLayer: Layer;
    type Item: Item<Node = N>;

    fn dangling() -> Self;

    //Safety:
    // 1. `self` is not dangling.
    unsafe fn as_ref(&self) -> &Self::Item;

    //Safety:
    // 1. `self` is not dangling.
    unsafe fn as_mut(&mut self) -> &mut Self::Item;

    //Safety:
    // 1. `self` is not dangling.
    unsafe fn into_variant(self) -> ItemRefVariant<N>;

    //Safety:
    // 1. `self` is not dangling.
    unsafe fn into_owned(self) -> Box<Self::Item>;

    //Safety:
    // 1. `self` is not dangling.
    unsafe fn calculate_length(&self) -> Length;

    //Safety:
    // 1. `self` is not dangling.
    unsafe fn parent(&self) -> &ChildRefIndex<N>;

    //Safety:
    // 1. `self` is not dangling.
    unsafe fn set_parent(&mut self, parent: ChildRefIndex<N>);

    //Safety:
    // 1. `self` is not dangling.
    // 2. `self` is not a root Item.
    unsafe fn parent_mut(&mut self) -> &mut BranchRef<Self::SelfLayer, N>;

    // Safety:
    // 1. `self` is not dangling.
    // 2. All references belong to `references` instance.
    // 3. `count > 0`
    // 4. `self` data within `from..(from + count)` range is occupied.
    // 5. `ChildLayer` is correctly describes children kind.
    unsafe fn update_children(
        &mut self,
        references: &mut References<N>,
        from: ChildIndex,
        count: ChildCount,
    ) -> Length;

    //Safety:
    // 1. `self` is not dangling.
    // 2. All references belong to `references` instance.
    // 3. `from` is lesser than the number of occupied children.
    // 4. `children_split` correctly describes children layer splitting.
    unsafe fn split(
        &mut self,
        references: &mut References<N>,
        children_split: Split<N>,
        length: Length,
        from: ChildIndex,
    ) -> Split<N>;

    //Safety:
    // 1. `left_ref` is not dangling.
    // 2. `right_ref` is not dangling.
    // 3. `left_ref` and `right_ref` both have children layers of the same kind.
    // 4. `ChildLayer` is correctly describes children kind.
    // 5. All references belong to `references` instance.
    // 6. `left_ref` is not a root item.
    // 7. `right_ref` is a root item.
    #[inline]
    unsafe fn join_to_left(
        left_ref: &mut Self,
        right_ref: &mut Self,
        left_root_length: Length,
        right_length: Length,
        references: &mut References<N>,
    ) -> (bool, Option<ItemRefVariant<N>>) {
        let left_occupied = unsafe { left_ref.as_ref().occupied() };
        let right_occupied = unsafe { right_ref.as_ref().occupied() };

        if left_occupied + right_occupied <= capacity(<Self::Item as Item>::BRANCHING) {
            let span_addition = unsafe { ItemRef::merge_to_left(left_ref, right_ref, references) };

            unsafe { left_ref.parent_mut().inc_span_right(span_addition) };

            return (true, None);
        }

        let transfer_length = match right_occupied < <Self::Item as Item>::BRANCHING {
            false => 0,

            true => unsafe { ItemRef::balance_to_right(left_ref, right_ref, references) },
        };

        let left_parent = unsafe { left_ref.parent_mut() };

        (false, unsafe {
            left_parent.add_child_right(
                left_root_length,
                transfer_length,
                right_length + transfer_length,
                right_ref.into_variant(),
            )
        })
    }

    //Safety:
    // 1. `left_ref` is not dangling.
    // 2. `right_ref` is not dangling.
    // 3. `left_ref` and `right_ref` both have children layers of the same kind.
    // 4. `ChildLayer` is correctly describes children kind.
    // 5. All references belong to `references` instance.
    // 6. `left_ref` is a root item.
    // 7. `right_ref` is not a root item.
    #[inline]
    unsafe fn join_to_right(
        left_ref: &mut Self,
        right_ref: &mut Self,
        left_length: Length,
        right_root_length: Length,
        references: &mut References<N>,
    ) -> (bool, Option<ItemRefVariant<N>>) {
        let left_occupied = unsafe { left_ref.as_ref().occupied() };
        let right_occupied = unsafe { right_ref.as_ref().occupied() };

        if left_occupied + right_occupied <= capacity(<Self::Item as Item>::BRANCHING) {
            let span_addition = unsafe { ItemRef::merge_to_right(left_ref, right_ref, references) };

            unsafe { right_ref.parent_mut().inc_span_left(span_addition) };

            return (true, None);
        }

        let transfer_length = match left_occupied < <Self::Item as Item>::BRANCHING {
            false => 0,

            true => unsafe { ItemRef::balance_to_left(left_ref, right_ref, references) },
        };

        let right_parent = unsafe { right_ref.parent_mut() };

        (false, unsafe {
            right_parent.add_child_left(
                right_root_length,
                transfer_length,
                left_length + transfer_length,
                left_ref.into_variant(),
            )
        })
    }

    //Safety:
    // 1. `left_ref` is not dangling.
    // 2. `right_ref` is not dangling.
    // 3. `left_ref` and `right_ref` both have children layers of the same kind.
    // 4. `ChildLayer` is correctly describes children kind.
    // 5. All references belong to `references` instance.
    #[inline]
    unsafe fn join_roots(
        left_ref: &mut Self,
        right_ref: &mut Self,
        mut left_length: Length,
        mut right_length: Length,
        references: &mut References<N>,
    ) -> Option<ItemRefVariant<N>> {
        let left_occupied = unsafe { left_ref.as_ref().occupied() };
        let right_occupied = unsafe { right_ref.as_ref().occupied() };

        if left_occupied + right_occupied <= capacity(<Self::Item as Item>::BRANCHING) {
            let _ = unsafe { ItemRef::merge_to_left(left_ref, right_ref, references) };

            return None;
        }

        if left_occupied < <Self::Item as Item>::BRANCHING {
            let difference = unsafe { ItemRef::balance_to_left(left_ref, right_ref, references) };

            left_length += difference;
            right_length -= difference;
        } else if right_occupied < <Self::Item as Item>::BRANCHING {
            let difference = unsafe { ItemRef::balance_to_right(left_ref, right_ref, references) };

            left_length -= difference;
            right_length += difference;
        }

        let mut new_root_ref = Branch::<BranchLayer, _>::new(2);

        let parent_ref_variant = unsafe { new_root_ref.into_variant() };

        unsafe {
            left_ref.set_parent(ChildRefIndex {
                item: parent_ref_variant,
                index: 0,
            })
        };

        unsafe {
            right_ref.set_parent(ChildRefIndex {
                item: parent_ref_variant,
                index: 1,
            })
        };

        {
            let new_root = unsafe { new_root_ref.as_mut() };

            new_root.inner.children[0] = unsafe { left_ref.into_variant() };
            new_root.inner.children[1] = unsafe { right_ref.into_variant() };
            new_root.inner.spans[0] = left_length;
            new_root.inner.spans[1] = right_length;
        }

        Some(parent_ref_variant)
    }

    //Safety:
    // 1. `left_ref` is not dangling.
    // 2. `right_ref` is not dangling.
    // 3. `left_ref` and `right_ref` both have children layers of the same kind.
    // 4. `ChildLayer` is correctly describes children kind.
    // 5. All references belong to `references` instance.
    // 6. The total `left_ref` and `right_ref` occupied count is lesser or equal to capacity.
    #[inline]
    unsafe fn merge_to_left(
        left_ref: &mut Self,
        right_ref: &mut Self,
        references: &mut References<N>,
    ) -> Length {
        let left_occupied = unsafe { left_ref.as_ref().occupied() };
        let right_occupied = unsafe { right_ref.as_ref().occupied() };

        debug_assert!(
            left_occupied + right_occupied <= capacity(<Self::Item as Item>::BRANCHING),
            "Internal error. Merge failure.",
        );

        unsafe { left_ref.as_mut().inflate(left_occupied, right_occupied) };

        unsafe {
            right_ref
                .as_mut()
                .copy_to(left_ref.as_mut(), 0, left_occupied, right_occupied)
        };

        forget(*unsafe { right_ref.into_owned() });

        let difference =
            unsafe { left_ref.update_children(references, left_occupied, right_occupied) };

        difference
    }

    //Safety:
    // 1. `left_ref` is not dangling.
    // 2. `right_ref` is not dangling.
    // 3. `left_ref` and `right_ref` both have children layers of the same kind.
    // 4. `ChildLayer` is correctly describes children kind.
    // 5. All references belong to `references` instance.
    // 6. The total `left_ref` and `right_ref` occupied count is lesser or equal to capacity.
    #[inline]
    unsafe fn merge_to_right(
        left_ref: &mut Self,
        right_ref: &mut Self,
        references: &mut References<N>,
    ) -> Length {
        let left_occupied = unsafe { left_ref.as_ref().occupied() };
        let right_occupied = unsafe { right_ref.as_ref().occupied() };

        debug_assert!(
            left_occupied + right_occupied <= capacity(<Self::Item as Item>::BRANCHING),
            "Internal error. Merge failure.",
        );

        unsafe { right_ref.as_mut().inflate(0, left_occupied) };

        unsafe {
            left_ref
                .as_mut()
                .copy_to(right_ref.as_mut(), 0, 0, left_occupied)
        };

        forget(*unsafe { left_ref.into_owned() });

        let difference = unsafe { right_ref.update_children(references, 0, left_occupied) };

        let _ = unsafe { right_ref.update_children(references, left_occupied, right_occupied) };

        difference
    }

    //Safety:
    // 1. `left_ref` is not dangling.
    // 2. `right_ref` is not dangling.
    // 3. `left_ref` and `right_ref` both have children layers of the same kind.
    // 4. `ChildLayer` is correctly describes children kind.
    // 5. All references belong to `references` instance.
    // 6. The total `left_ref` and `right_ref` occupied count is greater than capacity.
    // 7. `left_ref` occupied count is lesser than branching factor.
    // 8. `right_ref` occupied count is greater or equal to branching factor.
    #[inline]
    unsafe fn balance_to_left(
        left_ref: &mut Self,
        right_ref: &mut Self,
        references: &mut References<N>,
    ) -> Length {
        let left_occupied = unsafe { left_ref.as_ref().occupied() };
        let right_occupied = unsafe { right_ref.as_ref().occupied() };

        debug_assert!(
            left_occupied + right_occupied > capacity(<Self::Item as Item>::BRANCHING),
            "Internal error. Balance failure.",
        );

        debug_assert!(
            left_occupied < <Self::Item as Item>::BRANCHING,
            "Internal error. Balance failure.",
        );

        debug_assert!(
            right_occupied >= <Self::Item as Item>::BRANCHING,
            "Internal error. Balance failure.",
        );

        let transfer_count = <Self::Item as Item>::BRANCHING - left_occupied;

        debug_assert!(
            right_occupied - <Self::Item as Item>::BRANCHING >= transfer_count,
            "Internal error. Balance failure.",
        );

        unsafe { left_ref.as_mut().inflate(left_occupied, transfer_count) };

        unsafe {
            right_ref
                .as_mut()
                .copy_to(left_ref.as_mut(), 0, left_occupied, transfer_count)
        };

        let is_right_balanced = unsafe { right_ref.as_mut().deflate(0, transfer_count) };

        debug_assert!(
            is_right_balanced,
            "Internal error. Balance-to-left failure.",
        );

        let difference =
            unsafe { left_ref.update_children(references, left_occupied, transfer_count) };

        let _ =
            unsafe { right_ref.update_children(references, 0, right_occupied - transfer_count) };

        difference
    }

    //Safety:
    // 1. `left_ref` is not dangling.
    // 2. `right_ref` is not dangling.
    // 3. `left_ref` and `right_ref` both have children layers of the same kind.
    // 4. `ChildLayer` is correctly describes children kind.
    // 5. All references belong to `references` instance.
    // 6. The total `left_ref` and `right_ref` occupied count is greater than capacity.
    // 7. `left_ref` occupied count is greater or equal to branching factor.
    // 8. `right_ref` occupied count is lesser than branching factor.
    #[inline]
    unsafe fn balance_to_right(
        left_ref: &mut Self,
        right_ref: &mut Self,
        references: &mut References<N>,
    ) -> Length {
        let left_occupied = unsafe { left_ref.as_ref().occupied() };
        let right_occupied = unsafe { right_ref.as_ref().occupied() };

        debug_assert!(
            left_occupied + right_occupied > capacity(<Self::Item as Item>::BRANCHING),
            "Internal error. Balance failure.",
        );

        debug_assert!(
            left_occupied >= <Self::Item as Item>::BRANCHING,
            "Internal error. Balance failure.",
        );

        debug_assert!(
            right_occupied < <Self::Item as Item>::BRANCHING,
            "Internal error. Balance failure.",
        );

        let transfer_count = <Self::Item as Item>::BRANCHING - right_occupied;

        debug_assert!(
            left_occupied >= <Self::Item as Item>::BRANCHING,
            "Internal error. Balance failure.",
        );

        debug_assert!(
            left_occupied - <Self::Item as Item>::BRANCHING >= transfer_count,
            "Internal error. Balance failure.",
        );

        unsafe { right_ref.as_mut().inflate(0, transfer_count) };

        unsafe {
            left_ref.as_mut().copy_to(
                right_ref.as_mut(),
                left_occupied - transfer_count,
                0,
                transfer_count,
            )
        };

        let is_left_balanced = unsafe {
            left_ref
                .as_mut()
                .deflate(left_occupied - transfer_count, transfer_count)
        };

        debug_assert!(
            is_left_balanced,
            "Internal error. Balance-to-right failure.",
        );

        let difference = unsafe { right_ref.update_children(references, 0, transfer_count) };

        let _ = unsafe {
            right_ref.update_children(
                references,
                transfer_count,
                <Self::Item as Item>::BRANCHING - transfer_count,
            )
        };

        difference
    }
}

pub(super) struct Split<N: Node> {
    pub(super) left_span: Length,
    pub(super) left_item: ItemRefVariant<N>,
    pub(super) right_span: Length,
    pub(super) right_item: ItemRefVariant<N>,
}

impl<N: Node> Split<N> {
    #[inline(always)]
    pub(super) const fn dangling() -> Self {
        Self {
            left_span: 0,
            left_item: ItemRefVariant::dangling(),
            right_span: 0,
            right_item: ItemRefVariant::dangling(),
        }
    }
}

pub(super) union ItemRefVariant<N: Node> {
    branch: BranchRef<(), N>,
    page: PageRef<N>,
    dangling: (),
}

impl<N: Node> Default for ItemRefVariant<N> {
    #[inline(always)]
    fn default() -> Self {
        Self::dangling()
    }
}

impl<N: Node> Clone for ItemRefVariant<N> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Node> Copy for ItemRefVariant<N> {}

impl<N: Node> ItemRefVariant<N> {
    #[inline(always)]
    pub(super) const fn dangling() -> Self {
        Self { dangling: () }
    }

    #[inline(always)]
    pub(super) fn from_branch<ChildLayer: Layer>(branch: BranchRef<ChildLayer, N>) -> Self {
        Self {
            branch: unsafe { transmute(branch) },
        }
    }

    #[inline(always)]
    pub(super) fn from_page(page: PageRef<N>) -> Self {
        Self { page }
    }

    // Safety:
    // 1. Variant is a Branch variant.
    // 2. `ChildLayer` correctly describes child layer of the Branch instance.
    #[inline(always)]
    pub(super) unsafe fn as_branch_ref<ChildLayer: Layer>(&self) -> &BranchRef<ChildLayer, N> {
        unsafe { transmute(&self.branch) }
    }

    // Safety:
    // 1. Variant is a Branch variant.
    // 2. `ChildLayer` correctly describes child layer of the Branch instance.
    #[inline(always)]
    pub(super) unsafe fn as_branch_mut<ChildLayer: Layer>(
        &mut self,
    ) -> &mut BranchRef<ChildLayer, N> {
        unsafe { transmute(&mut self.branch) }
    }

    //Safety: Variant is a Page variant.
    #[inline(always)]
    pub(super) unsafe fn as_page_ref(&self) -> &PageRef<N> {
        unsafe { &self.page }
    }

    //Safety: Variant is a Page variant.
    #[inline(always)]
    pub(super) unsafe fn as_page_mut(&mut self) -> &mut PageRef<N> {
        unsafe { &mut self.page }
    }

    //Safety: `SelfLayer` correctly describes variant kind.
    #[inline(always)]
    pub(super) unsafe fn set_parent<SelfLayer: Layer>(&mut self, parent: ChildRefIndex<N>) {
        match SelfLayer::descriptor() {
            LayerDescriptor::Branch => unsafe { self.as_branch_mut::<()>().set_parent(parent) },
            LayerDescriptor::Page => unsafe { self.as_page_mut().set_parent(parent) },
        }
    }
}
