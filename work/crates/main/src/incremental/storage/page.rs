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
    arena::RefIndex,
    incremental::storage::{
        branch::BranchRef,
        cache::CacheEntry,
        child::{ChildCount, ChildIndex, ChildRefIndex},
        item::{Item, ItemRef, ItemRefVariant, Split},
        nesting::PageLayer,
        references::References,
        utils::{array_copy_to, array_shift, capacity},
    },
    lexis::Length,
    report::debug_assert,
    std::*,
    syntax::Node,
};

const BRANCHING: ChildCount = 6;

pub(super) struct Page<N: Node> {
    pub(super) parent: ChildRefIndex<N>,
    pub(super) previous: Option<PageRef<N>>,
    pub(super) next: Option<PageRef<N>>,
    pub(super) occupied: ChildCount,
    pub(super) spans: [Length; capacity(BRANCHING)],
    pub(super) strings: [MaybeUninit<String>; capacity(BRANCHING)],
    pub(super) tokens: [MaybeUninit<N::Token>; capacity(BRANCHING)],
    pub(super) chunks: [RefIndex; capacity(BRANCHING)],
    pub(super) clusters: [MaybeUninit<Option<CacheEntry<N>>>; capacity(BRANCHING)],
}

impl<N: Node> Item for Page<N> {
    const BRANCHING: ChildCount = BRANCHING;

    type Node = N;

    #[inline(always)]
    fn occupied(&self) -> ChildCount {
        self.occupied
    }

    #[inline(always)]
    unsafe fn copy_to(
        &mut self,
        to: &mut Self,
        source: ChildCount,
        destination: ChildCount,
        count: ChildCount,
    ) {
        debug_assert!(
            source + count <= self.occupied,
            "Internal error. An attempt to copy non occupied data in Page.",
        );

        unsafe { array_copy_to(&mut self.spans, &mut to.spans, source, destination, count) };

        unsafe {
            array_copy_to(
                &mut self.strings,
                &mut to.strings,
                source,
                destination,
                count,
            )
        };

        unsafe { array_copy_to(&mut self.tokens, &mut to.tokens, source, destination, count) };

        unsafe { array_copy_to(&mut self.chunks, &mut to.chunks, source, destination, count) };

        unsafe {
            array_copy_to(
                &mut self.clusters,
                &mut to.clusters,
                source,
                destination,
                count,
            )
        };
    }

    #[inline(always)]
    unsafe fn inflate(&mut self, from: ChildIndex, count: ChildCount) {
        debug_assert!(
            from <= self.occupied,
            "Internal error. An attempt to inflate from out of bounds child in Page."
        );
        debug_assert!(
            count + self.occupied <= capacity(Self::BRANCHING),
            "Internal error. An attempt to inflate with overflow in Page."
        );
        debug_assert!(
            count > 0,
            "Internal error. An attempt to inflate of empty range in Page."
        );

        if from < self.occupied {
            unsafe { array_shift(&mut self.spans, from, from + count, self.occupied - from) };
            unsafe { array_shift(&mut self.strings, from, from + count, self.occupied - from) };
            unsafe { array_shift(&mut self.tokens, from, from + count, self.occupied - from) };
            unsafe { array_shift(&mut self.chunks, from, from + count, self.occupied - from) };
            unsafe { array_shift(&mut self.clusters, from, from + count, self.occupied - from) };
        }

        self.occupied += count;
    }

    #[inline(always)]
    unsafe fn deflate(&mut self, from: ChildIndex, count: ChildCount) -> bool {
        debug_assert!(
            from < self.occupied,
            "Internal error. An attempt to deflate from non occupied child in Page."
        );
        debug_assert!(
            from + count <= self.occupied,
            "Internal error. An attempt to deflate with overflow in Page."
        );
        debug_assert!(
            count > 0,
            "Internal error. An attempt to deflate of empty range."
        );

        if from + count < self.occupied {
            unsafe {
                array_shift(
                    &mut self.spans,
                    from + count,
                    from,
                    self.occupied - from - count,
                )
            };
            unsafe {
                array_shift(
                    &mut self.strings,
                    from + count,
                    from,
                    self.occupied - from - count,
                )
            };
            unsafe {
                array_shift(
                    &mut self.tokens,
                    from + count,
                    from,
                    self.occupied - from - count,
                )
            };
            unsafe {
                array_shift(
                    &mut self.chunks,
                    from + count,
                    from,
                    self.occupied - from - count,
                )
            };
            unsafe {
                array_shift(
                    &mut self.clusters,
                    from + count,
                    from,
                    self.occupied - from - count,
                )
            };
        }

        self.occupied -= count;

        self.occupied >= Self::BRANCHING
    }
}

impl<N: Node> Page<N> {
    #[inline(always)]
    pub(super) fn new(occupied: ChildCount) -> PageRef<N> {
        debug_assert!(
            occupied > 0,
            "Internal error. An attempt to create Page with zero occupied values."
        );

        debug_assert!(
            occupied <= capacity(Self::BRANCHING),
            "Internal error. An attempt to create Page with occupied value exceeding capacity."
        );

        let page = Self {
            parent: ChildRefIndex::dangling(),
            previous: None,
            next: None,
            occupied,
            spans: Default::default(),
            strings: unsafe { MaybeUninit::uninit().assume_init() },
            tokens: unsafe { MaybeUninit::uninit().assume_init() },
            chunks: Default::default(),
            clusters: unsafe { MaybeUninit::uninit().assume_init() },
        };

        let pointer = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(page))) };

        PageRef { pointer }
    }

    // Safety:
    // 1. All references belong to `references` instance.
    pub(super) unsafe fn free(mut self, references: &mut References<N>) -> ChildCount {
        for index in 0..self.occupied {
            let string = unsafe { self.strings.get_unchecked_mut(index) };

            unsafe { string.assume_init_drop() };

            let token = unsafe { self.tokens.get_unchecked_mut(index) };

            unsafe { token.assume_init_drop() };

            let chunk_index = *unsafe { self.chunks.get_unchecked(index) };

            unsafe { references.chunks.remove_unchecked(chunk_index) };

            let cache_entry =
                take(unsafe { self.clusters.get_unchecked_mut(index).assume_init_mut() });

            if let Some(cache_entry) = cache_entry {
                unsafe { references.clusters.remove_unchecked(cache_entry.ref_index) };
            }
        }

        self.occupied
    }
}

#[repr(transparent)]
pub(super) struct PageRef<N: Node> {
    pointer: NonNull<Page<N>>,
}

impl<N: Node> Clone for PageRef<N> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Node> Copy for PageRef<N> {}

impl<N: Node> PartialEq for PageRef<N> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.pointer == other.pointer
    }
}

impl<N: Node> Eq for PageRef<N> {}

impl<N: Node> ItemRef<(), N> for PageRef<N> {
    type SelfLayer = PageLayer;

    type Item = Page<N>;

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
        ItemRefVariant::from_page(self)
    }

    #[inline(always)]
    unsafe fn into_owned(self) -> Box<Self::Item> {
        unsafe { Box::from_raw(self.pointer.as_ptr()) }
    }

    #[inline(always)]
    unsafe fn calculate_length(&self) -> Length {
        let page = unsafe { self.as_ref() };

        let mut length = 0;

        for index in 0..page.occupied {
            length += unsafe { page.spans.get_unchecked(index) };
        }

        length
    }

    #[inline(always)]
    unsafe fn parent(&self) -> &ChildRefIndex<N> {
        unsafe { &self.as_ref().parent }
    }

    #[inline(always)]
    unsafe fn set_parent(&mut self, parent: ChildRefIndex<N>) {
        unsafe { self.as_mut().parent = parent };
    }

    #[inline(always)]
    unsafe fn parent_mut(&mut self) -> &mut BranchRef<Self::SelfLayer, N> {
        let parent_ref_index = unsafe { &mut self.as_mut().parent };

        debug_assert!(
            !parent_ref_index.is_dangling(),
            "Internal error. An attempt to get parent from root.",
        );

        unsafe { parent_ref_index.item.as_branch_mut() }
    }

    unsafe fn update_children(
        &mut self,
        references: &mut References<N>,
        from: ChildIndex,
        count: ChildCount,
    ) -> Length {
        let self_variant = self.into_variant();

        let page = unsafe { self.as_mut() };

        debug_assert!(
            from + count <= page.occupied,
            "Internal error. An attempt to update references in non occupied data in Page.",
        );

        let mut length = 0;

        for index in from..(from + count) {
            length += *unsafe { page.spans.get_unchecked(index) };

            {
                let chunk_index = *unsafe { page.chunks.get_unchecked(index) };
                let chunk_ref = unsafe { references.chunks.get_unchecked_mut(chunk_index) };

                chunk_ref.item = self_variant;
                chunk_ref.index = index;
            }

            let cache_entry = unsafe { page.clusters.get_unchecked(index).assume_init_ref() };

            if let Some(cache_entry) = cache_entry {
                let cluster_ref =
                    unsafe { references.clusters.get_unchecked_mut(cache_entry.ref_index) };

                cluster_ref.item = self_variant;
                cluster_ref.index = index;
            }
        }

        length
    }

    #[inline]
    unsafe fn split(
        &mut self,
        references: &mut References<N>,
        _children_split: Split<N>,
        length: Length,
        from: ChildIndex,
    ) -> Split<N> {
        let mut parent_split = Split::dangling();

        let occupied = unsafe { self.as_ref().occupied };

        debug_assert!(
            from < occupied,
            "Internal error. Split at position out of bounds.",
        );

        match from == 0 {
            true => {
                parent_split.right_span = length;
                parent_split.right_item = unsafe { self.into_variant() };

                parent_split.left_span = 0;
            }

            false => {
                let left = unsafe { self.as_mut() };
                let mut right_ref = Page::new(occupied - from);

                match &mut left.next {
                    None => (),

                    Some(next) => {
                        unsafe { PageRef::interconnect(&mut right_ref, next) };

                        left.next = None;
                    }
                };

                unsafe { left.copy_to(right_ref.as_mut(), from, 0, occupied - from) };
                left.occupied = from;

                parent_split.right_span =
                    unsafe { right_ref.update_children(references, 0, occupied - from) };
                parent_split.right_item = unsafe { right_ref.into_variant() };

                parent_split.left_span = length - parent_split.right_span;
                parent_split.left_item = unsafe { self.into_variant() };
            }
        }

        parent_split
    }
}

impl<N: Node> PageRef<N> {
    // Safety: `left` and `right` are not dangling reference.
    #[inline(always)]
    pub(super) unsafe fn interconnect(left: &mut Self, right: &mut Self) {
        unsafe {
            left.as_mut().next = Some(*right);
        }

        unsafe {
            right.as_mut().previous = Some(*left);
        }
    }

    // Safety: `self` is not a dangling reference.
    #[inline(always)]
    pub(super) unsafe fn disconnect_left(&mut self) {
        unsafe {
            self.as_mut().previous = None;
        }
    }

    // Safety: `self` is not a dangling reference.
    #[inline(always)]
    pub(super) unsafe fn disconnect_right(&mut self) {
        unsafe {
            self.as_mut().next = None;
        }
    }

    // Safety:
    // 1. `self` is not a dangling reference.
    // 2. `'a` does not outlive Page instance.
    #[inline(always)]
    pub(super) unsafe fn as_external_ref<'a>(&self) -> &'a Page<N> {
        unsafe { self.pointer.as_ref() }
    }

    // Safety:
    // 1. `self` is not a dangling reference.
    // 2. `'a` does not outlive Page instance.
    #[inline(always)]
    pub(super) unsafe fn as_external_mut<'a>(&self) -> &'a mut Page<N> {
        let mut pointer = self.pointer;

        unsafe { pointer.as_mut() }
    }

    // Safety:
    // 1. `self` is not a dangling reference.
    // 2. All references belong to `references` instance.
    // 3. `from < self.occupied`.
    // 4. `from + count <= self.occupied.
    // 5. `count > 0`
    // 6. `spans`, `strings` and `tokens` can produce at least `count` items.
    #[inline]
    pub(super) unsafe fn rewrite(
        &mut self,
        references: &mut References<N>,
        from: ChildIndex,
        count: ChildCount,
        spans: &mut impl Iterator<Item = Length>,
        strings: &mut impl Iterator<Item = String>,
        tokens: &mut impl Iterator<Item = N::Token>,
    ) -> (Length, Length) {
        let page = unsafe { self.as_mut() };

        debug_assert!(
            from < page.occupied,
            "Internal error. An attempt to rewrite from non occupied child in Page."
        );
        debug_assert!(
            from + count <= page.occupied,
            "Internal error. An attempt to rewrite with overflow in Page."
        );
        debug_assert!(
            count > 0,
            "Internal error. An attempt to rewrite of empty range."
        );

        let mut dec = 0;
        let mut inc = 0;

        references.chunks.commit();

        for index in from..(from + count) {
            debug_assert!(
                index < capacity(Page::<N>::BRANCHING),
                "Internal error. Chunk index is out of bounds.",
            );

            let new_span = match spans.next() {
                Some(span) => span,
                None => {
                    #[cfg(debug_assertions)]
                    {
                        unreachable!("Internal error. Spans iterator exceeded.");
                    }

                    #[allow(unreachable_code)]
                    unsafe {
                        unreachable_unchecked()
                    }
                }
            };

            debug_assert!(new_span > 0, "Internal error. Zero input span.");

            let new_string = match strings.next() {
                Some(string) => string,
                None => {
                    #[cfg(debug_assertions)]
                    {
                        unreachable!("Internal error. Strings iterator exceeded.");
                    }

                    #[allow(unreachable_code)]
                    unsafe {
                        unreachable_unchecked()
                    }
                }
            };

            let new_token = match tokens.next() {
                Some(token) => token,
                None => {
                    #[cfg(debug_assertions)]
                    {
                        unreachable!("Internal error. Tokens iterator exceeded.");
                    }

                    #[allow(unreachable_code)]
                    unsafe {
                        unreachable_unchecked()
                    }
                }
            };

            let span = unsafe { page.spans.get_unchecked_mut(index) };
            let string = unsafe { page.strings.get_unchecked_mut(index).assume_init_mut() };
            let token = unsafe { page.tokens.get_unchecked_mut(index).assume_init_mut() };
            let chunk_index = unsafe { *page.chunks.get_unchecked(index) };
            let cache_entry =
                take(unsafe { page.clusters.get_unchecked_mut(index).assume_init_mut() });

            dec += *span;
            inc += new_span;

            *span = new_span;
            let _ = replace(string, new_string);
            let _ = replace(token, new_token);

            unsafe { references.chunks.upgrade(chunk_index) };

            if let Some(cache_entry) = cache_entry {
                unsafe { references.clusters.remove_unchecked(cache_entry.ref_index) }
            }
        }

        (dec, inc)
    }

    // Safety:
    // 1. `self` is not a dangling reference.
    // 2. All references belong to `references` instance.
    // 3. `from < self.occupied`.
    // 4. `from + count <= self.occupied.
    // 5. `count > 0`
    #[inline]
    pub(super) unsafe fn remove(
        &mut self,
        references: &mut References<N>,
        from: ChildIndex,
        count: ChildCount,
    ) -> Length {
        let page = unsafe { self.as_mut() };

        debug_assert!(
            from < page.occupied,
            "Internal error. An attempt to remove from non occupied child in Page."
        );
        debug_assert!(
            from + count <= page.occupied,
            "Internal error. An attempt to remove with overflow in Page."
        );
        debug_assert!(
            count > 0,
            "Internal error. An attempt to remove of empty range."
        );

        let mut length = 0;

        for index in from..(from + count) {
            let span = unsafe { *page.spans.get_unchecked(index) };

            unsafe { page.strings.get_unchecked_mut(index).assume_init_drop() };
            unsafe { page.tokens.get_unchecked_mut(index).assume_init_drop() };

            let chunk_index = unsafe { *page.chunks.get_unchecked(index) };

            unsafe { references.chunks.remove_unchecked(chunk_index) };

            let cache_entry =
                take(unsafe { page.clusters.get_unchecked_mut(index).assume_init_mut() });

            if let Some(cache_entry) = cache_entry {
                unsafe { references.clusters.remove_unchecked(cache_entry.ref_index) }
            }

            length += span;
        }

        if from + count < page.occupied {
            unsafe {
                array_shift(
                    &mut page.spans,
                    from + count,
                    from,
                    page.occupied - from - count,
                )
            };
            unsafe {
                array_shift(
                    &mut page.strings,
                    from + count,
                    from,
                    page.occupied - from - count,
                )
            };
            unsafe {
                array_shift(
                    &mut page.tokens,
                    from + count,
                    from,
                    page.occupied - from - count,
                )
            };
            unsafe {
                array_shift(
                    &mut page.chunks,
                    from + count,
                    from,
                    page.occupied - from - count,
                )
            };
            unsafe {
                array_shift(
                    &mut page.clusters,
                    from + count,
                    from,
                    page.occupied - from - count,
                )
            };

            for index in from..(page.occupied - count) {
                {
                    let chunk_index = *unsafe { page.chunks.get_unchecked(index) };
                    let chunk_ref = unsafe { references.chunks.get_unchecked_mut(chunk_index) };

                    chunk_ref.index = index;
                }

                let cache_entry = unsafe { page.clusters.get_unchecked(index).assume_init_ref() };

                if let Some(cache_entry) = cache_entry {
                    let cluster_ref =
                        unsafe { references.clusters.get_unchecked_mut(cache_entry.ref_index) };

                    cluster_ref.index = index;
                }
            }
        }

        page.occupied -= count;

        length
    }

    // Safety:
    // 1. `self` is not a dangling reference.
    // 2. All references belong to `references` instance.
    // 3. `from <= self.occupied`.
    // 4. `from + count <= self.occupied.
    // 5. `count > 0`
    // 6. `spans`, `strings` and `tokens` can produce at least `count` items.
    #[inline]
    pub(super) unsafe fn insert(
        &mut self,
        references: &mut References<N>,
        from: ChildIndex,
        count: ChildCount,
        spans: &mut impl Iterator<Item = Length>,
        strings: &mut impl Iterator<Item = String>,
        tokens: &mut impl Iterator<Item = N::Token>,
    ) -> Length {
        let self_ref_variant = unsafe { self.into_variant() };

        let page = unsafe { self.as_mut() };

        debug_assert!(
            from <= page.occupied,
            "Internal error. An attempt to insert from non occupied child in Page."
        );
        debug_assert!(
            from + count <= capacity(Page::<N>::BRANCHING),
            "Internal error. An attempt to insert with overflow in Page."
        );
        debug_assert!(
            count > 0,
            "Internal error. An attempt to insert of empty range."
        );

        unsafe {
            page.inflate(from, count);
        }

        let mut length = 0;

        for index in from..(from + count) {
            debug_assert!(
                index < capacity(Page::<N>::BRANCHING),
                "Internal error. Chunk index is out of bounds.",
            );

            let new_span = match spans.next() {
                Some(span) => span,

                None => {
                    #[cfg(debug_assertions)]
                    {
                        unreachable!("Internal error. Spans iterator exceeded.");
                    }

                    #[allow(unreachable_code)]
                    unsafe {
                        unreachable_unchecked()
                    }
                }
            };

            debug_assert!(new_span > 0, "Internal error. Zero input span.");

            let new_string = match strings.next() {
                Some(string) => string,
                None => {
                    #[cfg(debug_assertions)]
                    {
                        unreachable!("Internal error. Strings iterator exceeded.");
                    }

                    #[allow(unreachable_code)]
                    unsafe {
                        unreachable_unchecked()
                    }
                }
            };

            let new_token = match tokens.next() {
                Some(token) => token,
                None => {
                    #[cfg(debug_assertions)]
                    {
                        unreachable!("Internal error. Tokens iterator exceeded.");
                    }

                    #[allow(unreachable_code)]
                    unsafe {
                        unreachable_unchecked()
                    }
                }
            };

            length += new_span;

            unsafe {
                *page.spans.get_unchecked_mut(index) = new_span;
            }

            unsafe {
                page.strings.get_unchecked_mut(index).write(new_string);
            }

            unsafe {
                page.tokens.get_unchecked_mut(index).write(new_token);
            }

            unsafe {
                *page.chunks.get_unchecked_mut(index) =
                    references.chunks.insert_index(ChildRefIndex {
                        item: self_ref_variant,
                        index,
                    })
            }

            unsafe {
                page.clusters.get_unchecked_mut(index).write(None);
            }
        }

        for index in (from + count)..page.occupied {
            {
                let chunk_index = *unsafe { page.chunks.get_unchecked(index) };
                let chunk_ref = unsafe { references.chunks.get_unchecked_mut(chunk_index) };

                chunk_ref.index = index;
            }

            let cache_entry = unsafe { page.clusters.get_unchecked(index).assume_init_ref() };

            if let Some(cache_entry) = cache_entry {
                let cluster_ref =
                    unsafe { references.clusters.get_unchecked_mut(cache_entry.ref_index) };

                cluster_ref.index = index;
            }
        }

        length
    }
}

pub(super) struct PageList<N: Node> {
    pub(super) first: PageRef<N>,
    pub(super) last: PageRef<N>,
}

impl<N: Node> Clone for PageList<N> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Node> Copy for PageList<N> {}

impl<N: Node> PageList<N> {
    #[inline(always)]
    pub(super) fn dangling() -> Self {
        Self {
            first: PageRef::dangling(),
            last: PageRef::dangling(),
        }
    }
}
