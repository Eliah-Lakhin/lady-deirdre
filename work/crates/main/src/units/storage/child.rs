////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{
    mem::{replace, take},
    ops::Deref,
    str::from_utf8_unchecked,
};

use crate::{
    arena::EntryIndex,
    lexis::{ByteIndex, Length, TokenCount},
    report::{ld_assert, ld_unreachable},
    syntax::Node,
    units::storage::{
        item::{Item, ItemRef, ItemRefVariant},
        page::Page,
        Cache,
    },
};

pub(super) type ChildIndex = usize;
pub(super) type ChildCount = usize;

pub(crate) struct ChildCursor<N: Node> {
    pub(super) item: ItemRefVariant<N>,
    pub(super) index: ChildIndex,
}

impl<N: Node> Clone for ChildCursor<N> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Node> Copy for ChildCursor<N> {}

impl<N: Node> ChildCursor<N> {
    #[inline(always)]
    pub(crate) const fn dangling() -> Self {
        Self {
            item: ItemRefVariant::dangling(),
            index: ChildIndex::MAX,
        }
    }

    #[inline(always)]
    pub(crate) const fn is_dangling(&self) -> bool {
        self.index == ChildIndex::MAX
    }

    // Safety:
    // 1. `self.item` and `other.item` are possibly dangling Page references.
    // 2. `self` and `other` belong to the same `Tree` instance.
    #[inline(always)]
    pub(crate) unsafe fn same_chunk_as(&self, other: &Self) -> bool {
        if self.index != other.index {
            return false;
        }

        if self.index != ChildIndex::MAX
            && unsafe { self.item.as_page_ref() != other.item.as_page_ref() }
        {
            return false;
        }

        true
    }

    // Safety:
    // 1. `self.item` and `other.item` are Page references.
    // 2. `self` and `other` belong to the same `Tree` instance.
    // 3. `self` is not ahead of `other`.
    // 4. `self` is not dangling.
    #[inline]
    pub(crate) unsafe fn continuous_to(&self, tail: &Self) -> Option<TokenCount> {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildRefIndex.",
        );

        let head_page_ref = unsafe { self.item.as_page_ref() };
        let head_page = unsafe { head_page_ref.as_ref() };

        ld_assert!(
            self.index < head_page.occupied,
            "ChildRefIndex index out of bounds.",
        );

        match tail.is_dangling() {
            false => {
                let tail_page_ref = unsafe { tail.item.as_page_ref() };

                match head_page_ref == tail_page_ref {
                    true => {
                        ld_assert!(
                            tail.index < head_page.occupied,
                            "ChildRefIndex index out of bounds.",
                        );

                        match self.index <= tail.index {
                            true => Some(tail.index - self.index),

                            false => unsafe { ld_unreachable!("Head is ahead of tail.") },
                        }
                    }

                    false => {
                        if tail.index > 0 {
                            return None;
                        }

                        match &head_page.next {
                            Some(next) if next == tail_page_ref => {
                                Some(head_page.occupied - self.index)
                            }

                            _ => None,
                        }
                    }
                }
            }

            true => match head_page.next.is_some() {
                false => Some(head_page.occupied - self.index),

                true => None,
            },
        }
    }

    #[inline(always)]
    pub(super) fn make_dangle(&mut self) {
        self.index = ChildIndex::MAX;
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    // 3. `'a` does not outlive corresponding Page instance.
    // 4. There are no other mutable references to this span.
    #[inline(always)]
    pub(crate) unsafe fn span<'a>(&self) -> &'a Length {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildRefIndex.",
        );

        let page = unsafe { self.item.as_page_ref().as_external_ref() };

        ld_assert!(
            self.index < page.occupied,
            "ChildRefIndex index out of bounds.",
        );

        let span = unsafe { page.spans.get_unchecked(self.index) };

        ld_assert!(*span > 0, "Zero span in Page.");

        span
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    // 3. `'a` does not outlive corresponding Page instance.
    // 4. There are no other mutable references to this String.
    #[inline(always)]
    pub(crate) unsafe fn string<'a>(&self) -> &'a str {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_external_ref() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        let slice = unsafe { page.string.byte_slice(page.occupied, self.index) };

        let string = unsafe { from_utf8_unchecked(slice) };

        ld_assert!(!string.is_empty(), "Empty string in Page.");

        string
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    // 3. `'a` does not outlive corresponding Page instance.
    // 4. There are no mutable references to this String.
    #[inline(always)]
    pub(crate) unsafe fn page_string<'a>(&self) -> &'a str {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_external_ref() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        let slice = match self.index == 0 {
            true => unsafe { page.string.bytes() },
            false => unsafe { page.string.byte_slice_from(page.occupied, self.index) },
        };

        let string = unsafe { from_utf8_unchecked(slice) };

        ld_assert!(!string.is_empty(), "Empty string in Page.");

        string
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    // 3. There are no other mutable references to this Token.
    #[inline(always)]
    pub(crate) unsafe fn token(&self) -> <N as Node>::Token {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_external_ref() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        unsafe { page.tokens.get_unchecked(self.index).assume_init_read() }
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    // 3. `'a` does not outlive corresponding Page instance.
    #[inline(always)]
    pub(crate) unsafe fn cache<'a>(&self) -> Option<&'a Cache> {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_external_ref() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        match unsafe { page.caches.get_unchecked(self.index).assume_init_ref() } {
            Some(cache) => Some(cache.deref()),

            None => None,
        }
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    // 3. Referred item contains a cache.
    // 4. There are no other references to this cache.
    #[inline(always)]
    pub(crate) unsafe fn release_cache(&self) -> Cache {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_external_mut() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        match unsafe { take(page.caches.get_unchecked_mut(self.index).assume_init_mut()) } {
            Some(cache) => *cache,

            None => unsafe { ld_unreachable!("An attempt to release unset cache.") },
        }
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    // 3. Referred item does not have a cache.
    #[inline(always)]
    pub(crate) unsafe fn install_cache(&self, cache: Cache) {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_external_mut() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        let previous = replace(
            page.caches.get_unchecked_mut(self.index).assume_init_mut(),
            Some(Box::new(cache)),
        );

        if previous.is_some() {
            unsafe { ld_unreachable!("An attempt to replace unreleased cache.") }
        }
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    #[inline(always)]
    pub(crate) unsafe fn chunk_entry_index(&self) -> EntryIndex {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_external_ref() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        unsafe { *page.chunks.get_unchecked(self.index) }
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    #[inline(always)]
    pub(crate) unsafe fn is_first(&self) -> bool {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_ref() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        self.index == 0 && page.previous.is_none()
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    #[inline(always)]
    #[allow(unused)]
    pub(crate) unsafe fn is_last(&self) -> bool {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_ref() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        self.index + 1 == page.occupied && page.next.is_none()
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    #[inline(always)]
    pub(crate) unsafe fn next(&mut self) {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_ref() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        if self.index + 1 < page.occupied {
            self.index += 1;
            return;
        }

        let Some(next_ref) = &page.next else {
            self.index = ChildIndex::MAX;
            return;
        };

        ld_assert!(
            unsafe { next_ref.as_ref().occupied } >= Page::<N>::B,
            "Incorrect Page balance."
        );

        self.item = unsafe { next_ref.into_variant() };
        self.index = 0;
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    #[inline(always)]
    pub(crate) unsafe fn next_page(&mut self) {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_ref() };

        let Some(next_ref) = &page.next else {
            self.index = ChildIndex::MAX;
            return;
        };

        ld_assert!(
            unsafe { next_ref.as_ref().occupied } >= Page::<N>::B,
            "Incorrect Page balance."
        );

        self.item = unsafe { next_ref.into_variant() };
        self.index = 0;
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    #[inline(always)]
    pub(crate) unsafe fn back(&mut self) {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_ref() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        if self.index > 0 {
            self.index -= 1;
            return;
        }

        match &page.previous {
            None => {
                self.index = ChildIndex::MAX;
            }

            Some(previous_ref) => {
                let previous_occupied = unsafe { previous_ref.as_ref().occupied };

                ld_assert!(previous_occupied >= Page::<N>::B, "Incorrect Page balance.");

                self.item = unsafe { previous_ref.into_variant() };
                self.index = previous_occupied - 1;
            }
        }
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Page reference.
    // 3. Referred Page will not be used used anymore, and the entire Tree
    //    will be dropped without other types of access.
    // 4. There are no other references to this Page data.
    #[inline(always)]
    pub(crate) unsafe fn take_lexis(
        &mut self,
        spans: &mut Vec<Length>,
        tokens: &mut Vec<N::Token>,
        indices: &mut Vec<ByteIndex>,
        text: &mut String,
    ) {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to access dangling ChildCursor.",
        );

        let page = unsafe { self.item.as_page_ref().as_external_mut() };

        ld_assert!(
            self.index < page.occupied,
            "ChildCursor index out of bounds.",
        );

        page.take_lexis(spans, tokens, indices, text);
    }

    // Safety:
    // 1. `self` is not dangling.
    // 2. `self.item` is a Branch reference.
    #[inline(always)]
    pub(super) unsafe fn branch_span(&self) -> Length {
        ld_assert!(
            !self.is_dangling(),
            "An attempt to get span from dangling ChildCursor.",
        );

        let branch = unsafe { self.item.as_branch_ref::<()>().as_ref() };

        ld_assert!(
            self.index < branch.inner.occupied,
            "ChildCursor index is out of bounds.",
        );

        unsafe { *branch.inner.spans.get_unchecked(self.index) }
    }
}
