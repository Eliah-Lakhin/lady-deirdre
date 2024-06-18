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

use std::{
    mem::{replace, take, MaybeUninit},
    ptr::NonNull,
    str::from_utf8_unchecked,
};

use crate::{
    arena::EntryIndex,
    lexis::{ByteIndex, Length},
    mem::{array_copy_to, array_shift},
    report::{ld_assert, ld_unreachable},
    syntax::Node,
    units::{
        storage::{
            branch::BranchRef,
            child::{ChildCount, ChildCursor, ChildIndex},
            item::{Item, ItemRef, ItemRefVariant, Split},
            nesting::PageLayer,
            refs::TreeRefs,
            string::PageString,
            Cache,
            PAGE_B,
            PAGE_CAP,
        },
        Watcher,
    },
};

pub(super) struct Page<N: Node> {
    pub(super) parent: ChildCursor<N>,
    pub(super) previous: Option<PageRef<N>>,
    pub(super) next: Option<PageRef<N>>,
    pub(super) occupied: ChildCount,
    pub(super) spans: [Length; PAGE_CAP],
    pub(super) string: PageString,
    pub(super) tokens: [MaybeUninit<N::Token>; PAGE_CAP],
    pub(super) chunks: [EntryIndex; PAGE_CAP],
    pub(super) caches: [MaybeUninit<Option<Box<Cache>>>; PAGE_CAP],
}

impl<N: Node> Item for Page<N> {
    const B: ChildCount = PAGE_B;
    const CAP: ChildCount = PAGE_CAP;

    type Node = N;

    #[inline(always)]
    fn occupied(&self) -> ChildCount {
        self.occupied
    }

    #[inline(always)]
    unsafe fn copy_to(
        &self,
        to: &mut Self,
        source: ChildCount,
        destination: ChildCount,
        count: ChildCount,
    ) {
        ld_assert!(
            source + count <= self.occupied,
            "An attempt to copy non occupied data in Page.",
        );

        unsafe { array_copy_to(&self.spans, &mut to.spans, source, destination, count) };

        unsafe {
            self.string.copy_to(
                self.occupied,
                &mut to.string,
                to.occupied,
                source,
                destination,
                count,
            )
        }

        unsafe { array_copy_to(&self.tokens, &mut to.tokens, source, destination, count) };

        unsafe { array_copy_to(&self.chunks, &mut to.chunks, source, destination, count) };

        unsafe { array_copy_to(&self.caches, &mut to.caches, source, destination, count) };
    }

    #[inline(always)]
    unsafe fn inflate(&mut self, from: ChildIndex, count: ChildCount) {
        ld_assert!(
            from <= self.occupied,
            "An attempt to inflate from out of bounds child in Page."
        );
        ld_assert!(
            count + self.occupied <= Self::CAP,
            "An attempt to inflate with overflow in Page."
        );
        ld_assert!(count > 0, "An attempt to inflate of empty range in Page.");

        if from < self.occupied {
            unsafe { array_shift(&mut self.spans, from, from + count, self.occupied - from) };
            unsafe { array_shift(&mut self.tokens, from, from + count, self.occupied - from) };
            unsafe { array_shift(&mut self.chunks, from, from + count, self.occupied - from) };
            unsafe { array_shift(&mut self.caches, from, from + count, self.occupied - from) };
        }

        unsafe { self.string.inflate(self.occupied, from, count) };

        self.occupied += count;
    }

    #[inline(always)]
    unsafe fn deflate(&mut self, from: ChildIndex, count: ChildCount) -> bool {
        ld_assert!(
            from < self.occupied,
            "An attempt to deflate from non occupied child in Page."
        );
        ld_assert!(
            from + count <= self.occupied,
            "An attempt to deflate with overflow in Page."
        );
        ld_assert!(count > 0, "An attempt to deflate of empty range.");

        unsafe { self.string.deflate(self.occupied, from, count) };

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
                    &mut self.caches,
                    from + count,
                    from,
                    self.occupied - from - count,
                )
            };
        }

        self.occupied -= count;

        self.occupied >= Self::B
    }
}

impl<N: Node> Page<N> {
    #[inline(always)]
    pub(super) fn new(occupied: ChildCount) -> PageRef<N> {
        ld_assert!(
            occupied > 0,
            "An attempt to create Page with zero occupied values."
        );

        ld_assert!(
            occupied <= Self::CAP,
            "An attempt to create Page with occupied value exceeding capacity."
        );

        let page = Self {
            parent: ChildCursor::dangling(),
            previous: None,
            next: None,
            occupied,
            spans: Default::default(),
            string: PageString::default(),
            tokens: unsafe { MaybeUninit::uninit().assume_init() },
            chunks: Default::default(),
            caches: unsafe { MaybeUninit::uninit().assume_init() },
        };

        let pointer = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(page))) };

        PageRef { pointer }
    }

    pub(super) fn take_lexis(
        &mut self,
        spans: &mut Vec<Length>,
        tokens: &mut Vec<N::Token>,
        indices: &mut Vec<ByteIndex>,
        text: &mut String,
    ) {
        for index in 0..self.occupied {
            spans.push({
                let span = *unsafe { self.spans.get_unchecked(index) };
                span
            });

            tokens.push({
                let token = unsafe { self.tokens.get_unchecked(index) };
                unsafe { token.assume_init_read() }
            });

            {
                let byte_index = unsafe { self.string.get_byte_index(index) };
                indices.push(text.len() + byte_index);
            }

            let _ = take(unsafe { self.caches.get_unchecked_mut(index).assume_init_mut() });
        }

        let string = unsafe { from_utf8_unchecked(self.string.bytes()) };

        text.push_str(string);

        self.occupied = 0;
    }

    // Safety:
    // 1. All references belong to `refs` instance.
    pub(super) unsafe fn free_subtree(
        mut self,
        refs: &mut TreeRefs<N>,
        watcher: &mut impl Watcher,
    ) -> ChildCount {
        for index in 0..self.occupied {
            let token = unsafe { self.tokens.get_unchecked_mut(index) };

            unsafe { token.assume_init_drop() };

            let chunk_index = *unsafe { self.chunks.get_unchecked(index) };

            let _ = unsafe { refs.chunks.remove_unchecked(chunk_index) };

            let cache = take(unsafe { self.caches.get_unchecked_mut(index).assume_init_mut() });

            if let Some(cache) = cache {
                cache.free(refs, watcher);
            }
        }

        self.occupied
    }

    pub(super) unsafe fn free(mut self) {
        for index in 0..self.occupied {
            let token = unsafe { self.tokens.get_unchecked_mut(index) };

            unsafe { token.assume_init_drop() };

            let _ = take(unsafe { self.caches.get_unchecked_mut(index).assume_init_mut() });
        }
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
    unsafe fn parent(&self) -> &ChildCursor<N> {
        unsafe { &self.as_ref().parent }
    }

    #[inline(always)]
    unsafe fn set_parent(&mut self, parent: ChildCursor<N>) {
        unsafe { self.as_mut().parent = parent };
    }

    #[inline(always)]
    unsafe fn parent_mut(&mut self) -> &mut BranchRef<Self::SelfLayer, N> {
        let parent_entry_index = unsafe { &mut self.as_mut().parent };

        ld_assert!(
            !parent_entry_index.is_dangling(),
            "An attempt to get parent from root.",
        );

        unsafe { parent_entry_index.item.as_branch_mut() }
    }

    unsafe fn update_children(
        &mut self,
        refs: &mut TreeRefs<N>,
        from: ChildIndex,
        count: ChildCount,
    ) -> Length {
        let self_variant = self.into_variant();

        let page = unsafe { self.as_mut() };

        ld_assert!(
            from + count <= page.occupied,
            "An attempt to update refs in non occupied data in Page.",
        );

        let mut length = 0;

        for index in from..(from + count) {
            length += *unsafe { page.spans.get_unchecked(index) };

            {
                let chunk_index = *unsafe { page.chunks.get_unchecked(index) };
                let chunk_ref = unsafe { refs.chunks.get_unchecked_mut(chunk_index) };

                chunk_ref.item = self_variant;
                chunk_ref.index = index;
            }
        }

        length
    }

    #[inline]
    unsafe fn split(
        &mut self,
        refs: &mut TreeRefs<N>,
        _children_split: Split<N>,
        length: Length,
        from: ChildIndex,
    ) -> Split<N> {
        let mut parent_split = Split::dangling();

        let occupied = unsafe { self.as_ref().occupied };

        ld_assert!(from < occupied, "Split at position out of bounds.",);

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
                unsafe { left.string.deflate(left.occupied, from, occupied - from) };
                left.occupied = from;

                parent_split.right_span =
                    unsafe { right_ref.update_children(refs, 0, occupied - from) };
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
    // 2. All references belong to `refs` instance.
    // 3. `from < self.occupied`.
    // 4. `from + count <= self.occupied.
    // 5. `count > 0`
    // 6. `spans`, `indices` and `tokens` have at least `count` items.
    #[inline]
    pub(super) unsafe fn rewrite(
        &mut self,
        refs: &mut TreeRefs<N>,
        watcher: &mut impl Watcher,
        from: ChildIndex,
        count: ChildCount,
        spans: &mut impl Iterator<Item = Length>,
        indices: &mut &[ByteIndex],
        tokens: &mut impl Iterator<Item = N::Token>,
        text: &str,
    ) -> (Length, Length) {
        let page = unsafe { self.as_mut() };

        ld_assert!(
            from < page.occupied,
            "An attempt to rewrite from non occupied child in Page."
        );
        ld_assert!(
            from + count <= page.occupied,
            "An attempt to rewrite with overflow in Page."
        );
        ld_assert!(count > 0, "An attempt to rewrite of empty range.");

        let mut dec = 0;
        let mut inc = 0;

        refs.chunks.commit(true);

        unsafe {
            page.string
                .rewrite(page.occupied, from, text.as_bytes(), indices, count)
        };

        *indices = unsafe { &indices[count..] };

        for index in from..(from + count) {
            ld_assert!(index < Page::<N>::CAP, "Chunk index is out of bounds.",);

            let new_span = match spans.next() {
                Some(span) => span,
                None => unsafe { ld_unreachable!("Spans iterator exceeded.") },
            };

            ld_assert!(new_span > 0, "Zero input span.");

            let new_token = match tokens.next() {
                Some(token) => token,
                None => unsafe { ld_unreachable!("Tokens iterator exceeded.") },
            };

            let span = unsafe { page.spans.get_unchecked_mut(index) };
            let token = unsafe { page.tokens.get_unchecked_mut(index).assume_init_mut() };
            let chunk_index = unsafe { *page.chunks.get_unchecked(index) };
            let cache = take(unsafe { page.caches.get_unchecked_mut(index).assume_init_mut() });

            dec += *span;
            inc += new_span;

            *span = new_span;
            let _ = replace(token, new_token);

            unsafe { refs.chunks.upgrade(chunk_index) };

            if let Some(cache) = cache {
                cache.free(refs, watcher);
            }
        }

        (dec, inc)
    }

    // Safety:
    // 1. `self` is not a dangling reference.
    // 2. All references belong to `refs` instance.
    // 3. `from < self.occupied`.
    // 4. `from + count <= self.occupied.
    // 5. `count > 0`
    #[inline]
    pub(super) unsafe fn remove(
        &mut self,
        refs: &mut TreeRefs<N>,
        watcher: &mut impl Watcher,
        from: ChildIndex,
        count: ChildCount,
    ) -> Length {
        let page = unsafe { self.as_mut() };

        ld_assert!(
            from < page.occupied,
            "An attempt to remove from non occupied child in Page."
        );
        ld_assert!(
            from + count <= page.occupied,
            "An attempt to remove with overflow in Page."
        );
        ld_assert!(count > 0, "An attempt to remove of empty range.");

        let mut length = 0;

        for index in from..(from + count) {
            let span = unsafe { *page.spans.get_unchecked(index) };

            unsafe { page.tokens.get_unchecked_mut(index).assume_init_drop() };

            let chunk_index = unsafe { *page.chunks.get_unchecked(index) };

            let _ = unsafe { refs.chunks.remove_unchecked(chunk_index) };

            let cache = take(unsafe { page.caches.get_unchecked_mut(index).assume_init_mut() });

            if let Some(cache) = cache {
                cache.free(refs, watcher);
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
                    &mut page.caches,
                    from + count,
                    from,
                    page.occupied - from - count,
                )
            };

            for index in from..(page.occupied - count) {
                {
                    let chunk_index = *unsafe { page.chunks.get_unchecked(index) };
                    let chunk_ref = unsafe { refs.chunks.get_unchecked_mut(chunk_index) };

                    chunk_ref.index = index;
                }
            }
        }

        unsafe { page.string.deflate(page.occupied, from, count) };
        page.occupied -= count;

        length
    }

    // Safety:
    // 1. `self` is not a dangling reference.
    // 2. All references belong to `refs` instance.
    // 3. `from <= self.occupied`.
    // 4. `from + count <= self.occupied.
    // 5. `count > 0`
    // 6. `spans`, `indices` and `tokens` have at least `count` items.
    #[inline]
    pub(super) unsafe fn insert(
        &mut self,
        refs: &mut TreeRefs<N>,
        from: ChildIndex,
        count: ChildCount,
        spans: &mut impl Iterator<Item = Length>,
        indices: &mut &[ByteIndex],
        tokens: &mut impl Iterator<Item = N::Token>,
        text: &str,
    ) -> Length {
        let self_ref_variant = unsafe { self.into_variant() };

        let page = unsafe { self.as_mut() };

        ld_assert!(
            from <= page.occupied,
            "An attempt to insert from non occupied child in Page."
        );
        ld_assert!(
            from + count <= Page::<N>::CAP,
            "An attempt to insert with overflow in Page."
        );
        ld_assert!(count > 0, "An attempt to insert of empty range.");

        unsafe { page.inflate(from, count) };

        unsafe {
            page.string
                .rewrite(page.occupied, from, text.as_bytes(), indices, count)
        };

        *indices = unsafe { &indices[count..] };

        let mut length = 0;

        for index in from..(from + count) {
            ld_assert!(index < Page::<N>::CAP, "Chunk index is out of bounds.",);

            let new_span = match spans.next() {
                Some(span) => span,

                None => unsafe { ld_unreachable!("Spans iterator exceeded.") },
            };

            ld_assert!(new_span > 0, "Zero input span.");

            let new_token = match tokens.next() {
                Some(token) => token,
                None => unsafe { ld_unreachable!("Tokens iterator exceeded.") },
            };

            length += new_span;

            unsafe {
                *page.spans.get_unchecked_mut(index) = new_span;
            }

            unsafe {
                page.tokens.get_unchecked_mut(index).write(new_token);
            }

            unsafe {
                *page.chunks.get_unchecked_mut(index) = refs.chunks.insert_raw(ChildCursor {
                    item: self_ref_variant,
                    index,
                })
            }

            unsafe {
                page.caches.get_unchecked_mut(index).write(None);
            }
        }

        for index in (from + count)..page.occupied {
            {
                let chunk_index = *unsafe { page.chunks.get_unchecked(index) };
                let chunk_ref = unsafe { refs.chunks.get_unchecked_mut(chunk_index) };

                chunk_ref.index = index;
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
