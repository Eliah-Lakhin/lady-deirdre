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

use std::mem::replace;

use crate::{
    lexis::{ByteIndex, Length, Site, TokenCount},
    report::{ld_assert, ld_assert_eq, ld_unreachable},
    syntax::Node,
    units::{
        storage::{
            branch::Branch,
            child::{ChildCursor, ChildIndex},
            item::{Item, ItemRef, ItemRefVariant, Split},
            nesting::{BranchLayer, Height, PageLayer},
            page::{Page, PageList, PageRef},
            refs::TreeRefs,
            spread::Spread,
        },
        Watcher,
    },
};

pub(crate) struct Tree<N: Node> {
    pub(super) length: Length,
    pub(super) height: Height,
    pub(super) root: ItemRefVariant<N>,
    pub(super) pages: PageList<N>,
}

impl<N: Node> Default for Tree<N> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            length: 0,
            height: 0,
            root: ItemRefVariant::dangling(),
            pages: PageList::dangling(),
        }
    }
}

impl<N: Node> Drop for Tree<N> {
    fn drop(&mut self) {
        ld_assert_eq!(self.height, 0, "MutableUnit memory leak.");
    }
}

impl<N: Node> Tree<N> {
    //Safety:
    // 1. `spans`, `strings` and `tokens` produce the same number of items equal to `count`.
    // 2. All `spans` values are positive integers.
    pub(crate) unsafe fn from_chunks(
        refs: &mut TreeRefs<N>,
        count: TokenCount,
        mut spans: impl Iterator<Item = Length>,
        mut indices: impl Iterator<Item = ByteIndex>,
        mut tokens: impl Iterator<Item = N::Token>,
        text: &str,
    ) -> Self {
        if count == 0 {
            return Self::default();
        }

        let mut height = 1;
        let mut length = 0;

        let mut spread = Spread::new::<Page<N>>(count);
        let mut first_page = None;
        let mut last_page = None;
        let mut layer_size = spread.layer_size();
        let mut first_byte = 0;

        ld_assert_eq!(count, spread.total_items(), "Partition failure.");

        loop {
            let index = spread.advance();

            if index == ChildIndex::MAX {
                break;
            }

            let span = match spans.next() {
                Some(span) => span,
                None => unsafe { ld_unreachable!("Spans iterator exceeded.") },
            };

            ld_assert!(span > 0, "Zero input span.");

            let byte_index = match indices.next() {
                Some(byte_index) => byte_index,
                None => unsafe { ld_unreachable!("Indices iterator exceeded.") },
            };

            let token = match tokens.next() {
                Some(token) => token,
                None => unsafe { ld_unreachable!("Tokens iterator exceeded.") },
            };

            length += span;

            if index == 0 {
                let mut new_page_ref = Page::new(spread.items);

                if let Some(mut previous_page) = replace(&mut last_page, Some(new_page_ref)) {
                    {
                        let page = unsafe { previous_page.as_mut() };
                        let slice = unsafe { text.get_unchecked(first_byte..byte_index) };

                        page.string.append(slice);
                    }

                    unsafe { PageRef::interconnect(&mut previous_page, &mut new_page_ref) };
                }

                first_byte = byte_index;

                if first_page.is_none() {
                    first_page = Some(new_page_ref);
                }
            }

            match &mut last_page {
                Some(page_ref) => {
                    let entry_index = refs.chunks.insert_raw(ChildCursor {
                        item: unsafe { page_ref.into_variant() },
                        index,
                    });

                    let page = unsafe { page_ref.as_mut() };

                    ld_assert!(index < page.occupied, "Partition failure.");

                    unsafe { *page.spans.get_unchecked_mut(index) = span };
                    unsafe { page.string.set_byte_index(index, byte_index - first_byte) };
                    unsafe { page.tokens.get_unchecked_mut(index).write(token) };
                    unsafe { *page.chunks.get_unchecked_mut(index) = entry_index };
                    unsafe { page.caches.get_unchecked_mut(index).write(None) };
                }

                None => unsafe { ld_unreachable!("Missing last page.") },
            }
        }

        if let Some(last_page) = &mut last_page {
            let page = unsafe { last_page.as_mut() };
            let slice = unsafe { text.get_unchecked(first_byte..) };

            page.string.append(slice);
        }

        let mut first_item = None;
        let mut last_item = None;

        if layer_size > 1 {
            height += 1;

            let mut next = first_page;

            spread = Spread::new::<Branch<PageLayer, N>>(layer_size);
            layer_size = spread.layer_size();

            loop {
                let index = spread.advance();

                if index == ChildIndex::MAX {
                    break;
                }

                if index == 0 {
                    let new_branch_ref = Branch::<PageLayer, N>::new(spread.items);

                    let new_branch_variant = unsafe { new_branch_ref.into_variant() };

                    if let Some(mut previous_branch) =
                        replace(&mut last_item, Some(new_branch_variant))
                    {
                        unsafe {
                            previous_branch
                                .as_branch_mut::<PageLayer>()
                                .as_mut()
                                .inner
                                .parent
                                .item = new_branch_variant;
                        }
                    }

                    if first_item.is_none() {
                        first_item = Some(new_branch_variant);
                    }
                }

                match &mut last_item {
                    Some(last) => {
                        let mut child_ref = match &next {
                            Some(page) => {
                                let current = *page;

                                next = unsafe { current.as_ref().next };

                                current
                            }

                            None => unsafe { ld_unreachable!("Missing last branch.") },
                        };

                        unsafe { child_ref.as_mut().parent = ChildCursor { item: *last, index } };

                        let branch = unsafe { last.as_branch_mut::<PageLayer>().as_mut() };

                        ld_assert!(index < branch.inner.occupied, "Partition failure.");

                        let child_span = unsafe { child_ref.calculate_length() };

                        unsafe { *branch.inner.spans.get_unchecked_mut(index) = child_span };
                        unsafe {
                            *branch.inner.children.get_unchecked_mut(index) =
                                child_ref.into_variant()
                        };
                    }

                    None => unsafe { ld_unreachable!("Missing last branch.") },
                }
            }
        }

        while layer_size > 1 {
            height += 1;

            let mut next = match first_item {
                Some(first_item) => first_item,

                None => unsafe { ld_unreachable!("Missing layer first item.") },
            };

            first_item = None;
            last_item = None;

            spread = Spread::new::<Branch<BranchLayer, N>>(layer_size);
            layer_size = spread.layer_size();

            loop {
                let index = spread.advance();

                if index == ChildIndex::MAX {
                    break;
                }

                if index == 0 {
                    let new_branch_ref = Branch::<BranchLayer, N>::new(spread.items);

                    let new_branch_variant = unsafe { new_branch_ref.into_variant() };

                    if let Some(mut previous_branch) =
                        replace(&mut last_item, Some(new_branch_variant))
                    {
                        unsafe {
                            previous_branch
                                .as_branch_mut::<BranchLayer>()
                                .as_mut()
                                .inner
                                .parent
                                .item = new_branch_variant;
                        }
                    }

                    if first_item.is_none() {
                        first_item = Some(new_branch_variant);
                    }
                }

                match &mut last_item {
                    Some(last) => {
                        let mut child_variant = next;

                        let child_ref = {
                            let current_ref = unsafe { child_variant.as_branch_mut::<()>() };

                            next = unsafe { current_ref.as_ref().inner.parent.item };

                            current_ref
                        };

                        unsafe {
                            child_ref.as_mut().inner.parent = ChildCursor { item: *last, index }
                        };

                        let branch = unsafe { last.as_branch_mut::<BranchLayer>().as_mut() };

                        ld_assert!(index < branch.inner.occupied, "Partition failure.");

                        let child_span = unsafe { child_ref.calculate_length() };

                        unsafe { *branch.inner.spans.get_unchecked_mut(index) = child_span };
                        unsafe { *branch.inner.children.get_unchecked_mut(index) = child_variant };
                    }

                    None => unsafe { ld_unreachable!("Missing last branch.") },
                }
            }
        }

        let first_page = match first_page {
            Some(first_page) => first_page,

            None => unsafe { ld_unreachable!("Missing first page.") },
        };

        let last_page = match last_page {
            Some(last_page) => last_page,

            None => unsafe { ld_unreachable!("Missing last page.") },
        };

        Self {
            length,
            height,
            root: match first_item {
                Some(root) => root,
                None => unsafe { first_page.into_variant() },
            },
            pages: PageList {
                first: first_page,
                last: last_page,
            },
        }
    }

    #[inline]
    pub(crate) fn code_length(&self) -> Length {
        self.length
    }

    #[inline(always)]
    pub(crate) fn first(&self) -> ChildCursor<N> {
        if self.height == 0 {
            return ChildCursor::dangling();
        }

        ChildCursor {
            item: unsafe { self.pages.first.into_variant() },
            index: 0,
        }
    }

    #[inline(always)]
    pub(crate) fn last(&self) -> ChildCursor<N> {
        if self.height == 0 {
            return ChildCursor::dangling();
        }

        let last_page = unsafe { self.pages.last.as_ref() };

        ld_assert!(last_page.occupied > 0, "Empty page.");

        ChildCursor {
            item: unsafe { self.pages.last.into_variant() },
            index: last_page.occupied - 1,
        }
    }

    #[inline]
    pub(crate) fn lookup(&self, site: &mut Site) -> ChildCursor<N> {
        if *site >= self.length {
            *site = 0;
            return ChildCursor::dangling();
        }

        ld_assert!(self.height > 0, "An attempt to search in the empty Tree.");

        let mut item = self.root;
        let mut depth = self.height;

        while depth > 1 {
            depth -= 1;

            let branch = unsafe { item.as_branch_ref::<()>().as_ref() };
            let mut index = 0;

            loop {
                ld_assert!(index < branch.inner.occupied, "Branch span inconsistency.");

                let span = unsafe { *branch.inner.spans.get_unchecked(index) };

                if span <= *site {
                    *site -= span;
                    index += 1;
                    continue;
                }

                item = unsafe { *branch.inner.children.get_unchecked(index) };
                break;
            }
        }

        let page = unsafe { item.as_page_ref().as_ref() };
        let mut index = 0;

        loop {
            ld_assert!(index < page.occupied, "Page span inconsistency.");

            let span = unsafe { *page.spans.get_unchecked(index) };

            if span <= *site {
                *site -= span;
                index += 1;
                continue;
            }

            break;
        }

        ChildCursor { item, index }
    }

    // Safety:
    // 1. `chunk_ref`(possibly dangling) refers valid data inside this instance.
    #[inline]
    pub(crate) unsafe fn site_of(&self, chunk_ref: &ChildCursor<N>) -> Site {
        if chunk_ref.is_dangling() {
            return self.length;
        }

        ld_assert!(self.height > 0, "Empty tree.");

        let page = unsafe { chunk_ref.item.as_page_ref().as_ref() };

        let mut site = 0;
        let mut index = chunk_ref.index;

        while index > 0 {
            ld_assert!(index < page.occupied, "ChildRefIndex index out of bounds.");

            index -= 1;

            site += unsafe { *page.spans.get_unchecked(index) };
        }

        let mut depth = self.height;
        let mut branch_ref = &page.parent;

        while depth > 1 {
            depth -= 1;

            ld_assert!(!branch_ref.is_dangling(), "Dangling parent ref.");

            let branch = unsafe { branch_ref.item.as_branch_ref::<()>().as_ref() };

            index = branch_ref.index;

            while index > 0 {
                ld_assert!(
                    index < branch.inner.occupied,
                    "ChildRefIndex index out of bounds.",
                );

                index -= 1;

                site += unsafe { *branch.inner.spans.get_unchecked(index) };
            }

            branch_ref = &branch.inner.parent;
        }

        site
    }

    // Safety:
    // 1. `chunk_ref` refers valid data inside this instance.
    #[inline(always)]
    pub(crate) unsafe fn is_writeable(
        &self,
        chunk_ref: &ChildCursor<N>,
        remove: TokenCount,
        insert: TokenCount,
    ) -> bool {
        ld_assert!(
            !chunk_ref.is_dangling(),
            "An attempt to access dangling ChildRefIndex.",
        );

        let page = unsafe { chunk_ref.item.as_page_ref().as_external_ref() };

        ld_assert!(
            chunk_ref.index < page.occupied,
            "ChildRefIndex index out of bounds.",
        );

        if page.occupied - chunk_ref.index < remove {
            return false;
        }

        match self.height {
            0 => unsafe { ld_unreachable!("Incorrect height.") },

            1 => {
                page.occupied + insert >= remove
                    && page.occupied + insert <= Page::<N>::CAP + remove
            }

            _ => {
                page.occupied + insert >= Page::<N>::B + remove
                    && page.occupied + insert <= Page::<N>::CAP + remove
            }
        }
    }

    // Safety:
    // 1. All references belong to `refs` instance.
    // 2. `chunk_ref` is not dangling and refers valid data inside this instance.
    // 3. Referred Page has enough space to remove `remove` and to insert `insert` items.
    // 4. `spans`, `indices` and `tokens` have the same number of items equal to `insert`.
    // 5. All `spans` values are positive integers.
    pub(crate) unsafe fn write(
        &mut self,
        refs: &mut TreeRefs<N>,
        watcher: &mut impl Watcher,
        mut chunk_ref: ChildCursor<N>,
        remove: TokenCount,
        insert: TokenCount,
        mut spans: impl Iterator<Item = Length>,
        mut indices: &[ByteIndex],
        mut tokens: impl Iterator<Item = N::Token>,
        text: &str,
    ) -> (ChildCursor<N>, Length) {
        ld_assert!(self.height > 0, "Empty tree.");

        ld_assert!(
            !chunk_ref.is_dangling(),
            "An attempt to access dangling ChildRefIndex.",
        );

        let page_ref = unsafe { chunk_ref.item.as_page_mut() };
        let occupied = unsafe { page_ref.as_ref().occupied };

        if self.height == 1 && insert == 0 && remove == occupied {
            let mut tree = replace(self, Self::default());

            let removed_count = unsafe { tree.free_as_subtree(refs, watcher) };

            ld_assert_eq!(remove, removed_count, "Token count inconsistency.");

            return (ChildCursor::dangling(), 0);
        }

        let rewrite = remove.min(insert);

        let (mut span_dec, mut span_inc) = match rewrite > 0 {
            true => unsafe {
                page_ref.rewrite(
                    refs,
                    watcher,
                    chunk_ref.index,
                    rewrite,
                    &mut spans,
                    &mut indices,
                    &mut tokens,
                    text,
                )
            },

            false => (0, 0),
        };

        if remove > rewrite {
            unsafe {
                span_dec +=
                    page_ref.remove(refs, watcher, chunk_ref.index + rewrite, remove - rewrite)
            };
        }

        if insert > rewrite {
            unsafe {
                span_inc += page_ref.insert(
                    refs,
                    chunk_ref.index + rewrite,
                    insert - rewrite,
                    &mut spans,
                    &mut indices,
                    &mut tokens,
                    text,
                )
            };
        }

        let mut parent = unsafe { &mut page_ref.as_mut().parent };

        while !parent.is_dangling() {
            let branch = unsafe { parent.item.as_branch_mut::<()>().as_mut() };

            let span = unsafe { branch.inner.spans.get_unchecked_mut(parent.index) };

            ld_assert!(*span + span_inc > span_dec, "Span inconsistency.");

            *span += span_inc;
            *span -= span_dec;

            parent = &mut branch.inner.parent;
        }

        ld_assert!(self.length + span_inc > span_dec, "Length inconsistency.");

        self.length += span_inc;
        self.length -= span_dec;

        if insert == 0 && chunk_ref.index + remove == occupied {
            chunk_ref = match chunk_ref.item.as_page_ref().as_ref().next {
                Some(next) => ChildCursor {
                    item: next.into_variant(),
                    index: 0,
                },

                None => ChildCursor::dangling(),
            };
        }

        (chunk_ref, span_inc)
    }

    //Safety:
    // 1. All references belong to `refs` instance.
    // 2. `chunk_ref` refers valid data inside this instance.
    pub(crate) unsafe fn split(
        &mut self,
        refs: &mut TreeRefs<N>,
        mut chunk_ref: ChildCursor<N>,
    ) -> Self {
        if chunk_ref.is_dangling() {
            return Self::default();
        }

        ld_assert!(self.height > 0, "An attempt to split empty Tree.");

        if self.height == 1 {
            return match chunk_ref.index == 0 {
                true => replace(self, Self::default()),

                false => {
                    let split = unsafe {
                        chunk_ref.item.as_page_mut().split(
                            refs,
                            Split::dangling(),
                            self.length,
                            chunk_ref.index,
                        )
                    };

                    self.root = split.left_item;
                    self.length = split.left_span;

                    let right_page = *unsafe { split.right_item.as_page_ref() };

                    Self {
                        length: split.right_span,
                        height: 1,
                        root: split.right_item,
                        pages: PageList {
                            first: right_page,
                            last: right_page,
                        },
                    }
                }
            };
        }

        let mut container = unsafe { *chunk_ref.item.as_page_ref().parent() };

        let mut split = {
            let length = unsafe { container.branch_span() };

            unsafe {
                chunk_ref
                    .item
                    .as_page_mut()
                    .split(refs, Split::dangling(), length, chunk_ref.index)
            }
        };

        match self.height > 2 {
            true => {
                let parent = unsafe { *container.item.as_branch_ref::<PageLayer>().parent() };

                split = {
                    let length = unsafe { parent.branch_span() };
                    let container_ref = unsafe { container.item.as_branch_mut::<PageLayer>() };

                    unsafe { container_ref.split(refs, split, length, container.index) }
                };

                container = parent;

                let mut depth = 3;

                while depth < self.height {
                    let parent = unsafe { *container.item.as_branch_ref::<BranchLayer>().parent() };

                    split = {
                        let length = unsafe { parent.branch_span() };
                        let container_ref =
                            unsafe { container.item.as_branch_mut::<BranchLayer>() };

                        unsafe { container_ref.split(refs, split, length, container.index) }
                    };

                    container = parent;

                    depth += 1;
                }

                let container_ref = unsafe { container.item.as_branch_mut::<BranchLayer>() };

                split = unsafe { container_ref.split(refs, split, self.length, container.index) }
            }

            false => {
                let container_ref = unsafe { container.item.as_branch_mut::<PageLayer>() };

                split = unsafe { container_ref.split(refs, split, self.length, container.index) }
            }
        };

        if split.left_span == 0 {
            return replace(self, Self::default());
        }

        let mut right = Self {
            length: split.right_span,
            height: self.height,
            root: split.right_item,
            pages: PageList {
                first: PageRef::dangling(),
                last: self.pages.last,
            },
        };

        while !unsafe { right.fix_leftmost_balance(refs) } {}

        if unsafe { right.pages.first.as_ref().next.is_none() } {
            right.pages.last = right.pages.first;
        }

        right.shrink_top();

        self.length = split.left_span;
        self.root = split.left_item;

        while !unsafe { self.fix_rightmost_balance(refs) } {}

        if unsafe { self.pages.last.as_ref().previous.is_none() } {
            self.pages.first = self.pages.last;
        }

        self.shrink_top();

        right
    }

    //Safety:
    // 1. All references belong to `refs` instance.
    #[inline]
    pub(crate) unsafe fn join(&mut self, refs: &mut TreeRefs<N>, other: Self) {
        if other.height == 0 {
            return;
        }

        if self.height == 0 {
            *self = other;
            return;
        }

        if self.height == other.height {
            unsafe { self.join_roots(other, refs) };
            return;
        }

        if self.height > other.height {
            unsafe { self.join_to_left(other, refs) };
            return;
        }

        unsafe { self.join_to_right(other, refs) };
    }

    //Safety:
    // 1. All references belong to `refs` instance.
    pub(crate) unsafe fn free_as_subtree(
        &mut self,
        refs: &mut TreeRefs<N>,
        watcher: &mut impl Watcher,
    ) -> TokenCount {
        if self.height == 0 {
            return 0;
        }

        let root = &mut self.root;

        let token_count = match self.height {
            1 => unsafe { root.as_page_ref().into_owned().free_subtree(refs, watcher) },

            2 => unsafe {
                root.as_branch_ref::<PageLayer>().into_owned().free_subtree(
                    self.height,
                    refs,
                    watcher,
                )
            },

            _ => unsafe {
                root.as_branch_ref::<BranchLayer>()
                    .into_owned()
                    .free_subtree(self.height, refs, watcher)
            },
        };

        self.height = 0;

        token_count
    }

    pub(crate) unsafe fn free(&mut self) {
        if self.height == 0 {
            return;
        }

        let root = &mut self.root;

        match self.height {
            1 => unsafe { root.as_page_ref().into_owned().free() },

            2 => unsafe {
                root.as_branch_ref::<PageLayer>()
                    .into_owned()
                    .free(self.height)
            },

            _ => unsafe {
                root.as_branch_ref::<BranchLayer>()
                    .into_owned()
                    .free(self.height)
            },
        };

        self.height = 0;
    }

    //Safety:
    // 1. `self.height >= 2`.
    // 2. All references belong to `refs` instance.
    unsafe fn fix_leftmost_balance(&mut self, refs: &mut TreeRefs<N>) -> bool {
        ld_assert!(self.height >= 2, "Incorrect height.");

        let mut depth = 1;
        let mut leftmost_variant = self.root;
        let mut balanced = true;

        while depth < self.height - 2 {
            depth += 1;

            let leftmost_ref = unsafe { leftmost_variant.as_branch_mut::<BranchLayer>() };
            let is_balanced;

            (is_balanced, leftmost_variant) =
                unsafe { leftmost_ref.fix_leftmost_balance::<BranchLayer>(refs) };

            balanced = balanced && is_balanced;
        }

        if depth < self.height - 1 {
            depth += 1;

            let leftmost_ref = unsafe { leftmost_variant.as_branch_mut::<BranchLayer>() };
            let is_balanced;

            (is_balanced, leftmost_variant) =
                unsafe { leftmost_ref.fix_leftmost_balance::<PageLayer>(refs) };

            balanced = balanced && is_balanced;
        }

        ld_assert_eq!(depth, self.height - 1, "Depth mismatch.");

        self.pages.first = {
            let leftmost_ref = unsafe { leftmost_variant.as_branch_mut::<PageLayer>() };
            let is_balanced;

            (is_balanced, leftmost_variant) =
                unsafe { leftmost_ref.fix_leftmost_balance::<()>(refs) };

            balanced = balanced && is_balanced;

            let mut first_page = unsafe { *leftmost_variant.as_page_ref() };

            unsafe { first_page.disconnect_left() };

            first_page
        };

        balanced
    }

    //Safety:
    // 1. `self.height >= 2`.
    // 2. All references belong to `refs` instance.
    #[inline]
    unsafe fn fix_rightmost_balance(&mut self, refs: &mut TreeRefs<N>) -> bool {
        ld_assert!(self.height >= 2, "Incorrect height.");

        let mut depth = 1;
        let mut rightmost_variant = self.root;
        let mut balanced = true;

        while depth < self.height - 2 {
            depth += 1;

            let rightmost_ref = unsafe { rightmost_variant.as_branch_mut::<BranchLayer>() };
            let is_balanced;

            (is_balanced, rightmost_variant) =
                unsafe { rightmost_ref.fix_rightmost_balance::<BranchLayer>(refs) };

            balanced = balanced && is_balanced;
        }

        if depth < self.height - 1 {
            depth += 1;

            let rightmost_ref = unsafe { rightmost_variant.as_branch_mut::<BranchLayer>() };
            let is_balanced;

            (is_balanced, rightmost_variant) =
                unsafe { rightmost_ref.fix_rightmost_balance::<PageLayer>(refs) };

            balanced = balanced && is_balanced;
        }

        ld_assert_eq!(depth, self.height - 1, "Depth mismatch.");

        self.pages.last = {
            let rightmost_ref = unsafe { rightmost_variant.as_branch_mut::<PageLayer>() };
            let is_balanced;

            (is_balanced, rightmost_variant) =
                unsafe { rightmost_ref.fix_rightmost_balance::<()>(refs) };

            balanced = balanced && is_balanced;

            let mut last_page = unsafe { *rightmost_variant.as_page_ref() };

            unsafe { last_page.disconnect_right() };

            last_page
        };

        balanced
    }

    #[inline]
    fn shrink_top(&mut self) {
        while self.height > 1 {
            let root_occupied = unsafe { self.root.as_branch_ref::<()>().as_ref().occupied() };

            if root_occupied > 1 {
                break;
            }

            let child = unsafe { self.root.as_branch_ref::<()>().as_ref().inner.children[0] };

            let _ = unsafe { *self.root.as_branch_ref::<()>().into_owned() };

            self.root = child;

            self.height -= 1;
        }

        match self.height {
            0 => unsafe { ld_unreachable!("Incorrect height.") },

            1 => unsafe { self.root.as_page_mut().as_mut().parent.make_dangle() },

            _ => unsafe {
                self.root
                    .as_branch_mut::<()>()
                    .as_mut()
                    .inner
                    .parent
                    .make_dangle()
            },
        }
    }

    //Safety:
    // 1. `self` height is greater than `other` height.
    // 2. `self.height` is positive value.
    // 3. All references belong to `refs` instance.
    unsafe fn join_to_left(&mut self, mut other: Self, refs: &mut TreeRefs<N>) {
        let mut depth = self.height;
        let mut left = self.root;

        while depth > other.height {
            depth -= 1;

            let parent = unsafe { left.as_branch_ref::<()>().as_ref() };

            left = *unsafe {
                parent
                    .inner
                    .children
                    .get_unchecked(parent.inner.occupied - 1)
            };
        }

        let right = &mut other.root;

        let new_root = match depth {
            0 => unsafe { ld_unreachable!("Incorrect height.") },

            1 => {
                let left_ref = unsafe { left.as_page_mut() };
                let right_ref = unsafe { right.as_page_mut() };

                let (merged, new_root) = unsafe {
                    ItemRef::join_to_left(left_ref, right_ref, self.length, other.length, refs)
                };

                if !merged {
                    unsafe {
                        PageRef::interconnect(&mut self.pages.last, &mut other.pages.first);
                    }

                    self.pages.last = other.pages.last;
                }

                new_root
            }

            2 => {
                unsafe {
                    PageRef::interconnect(&mut self.pages.last, &mut other.pages.first);
                }

                self.pages.last = other.pages.last;

                let left_ref = unsafe { left.as_branch_mut::<PageLayer>() };
                let right_ref = unsafe { right.as_branch_mut::<PageLayer>() };

                unsafe {
                    ItemRef::join_to_left(left_ref, right_ref, self.length, other.length, refs)
                }
                .1
            }

            _ => {
                unsafe {
                    PageRef::interconnect(&mut self.pages.last, &mut other.pages.first);
                }

                self.pages.last = other.pages.last;

                let left_ref = unsafe { left.as_branch_mut::<BranchLayer>() };
                let right_ref = unsafe { right.as_branch_mut::<BranchLayer>() };

                unsafe {
                    ItemRef::join_to_left(left_ref, right_ref, self.length, other.length, refs)
                }
                .1
            }
        };

        self.length += other.length;

        if let Some(new_root) = new_root {
            self.height += 1;
            self.root = new_root;
        }

        other.height = 0;
    }

    //Safety:
    // 1. `self` height is greater than `other` height.
    // 2. `self.height` is positive value.
    // 3. All references belong to `refs` instance.
    unsafe fn join_to_right(&mut self, mut other: Self, refs: &mut TreeRefs<N>) {
        let mut depth = other.height;
        let mut right = other.root;

        while depth > self.height {
            depth -= 1;

            let parent = unsafe { right.as_branch_ref::<()>().as_ref() };

            right = parent.inner.children[0];
        }

        let left = &mut self.root;

        let new_root = match depth {
            0 => unsafe { ld_unreachable!("Incorrect height.") },

            1 => {
                let left_ref = unsafe { left.as_page_mut() };
                let right_ref = unsafe { right.as_page_mut() };

                let (merged, new_root) = unsafe {
                    ItemRef::join_to_right(left_ref, right_ref, self.length, other.length, refs)
                };

                if !merged {
                    unsafe {
                        PageRef::interconnect(&mut self.pages.last, &mut other.pages.first);
                    }

                    other.pages.first = self.pages.first;
                }

                new_root
            }

            2 => {
                unsafe {
                    PageRef::interconnect(&mut self.pages.last, &mut other.pages.first);
                }

                other.pages.first = self.pages.first;

                let left_ref = unsafe { left.as_branch_mut::<PageLayer>() };
                let right_ref = unsafe { right.as_branch_mut::<PageLayer>() };

                unsafe {
                    ItemRef::join_to_right(left_ref, right_ref, self.length, other.length, refs)
                }
                .1
            }

            _ => {
                unsafe {
                    PageRef::interconnect(&mut self.pages.last, &mut other.pages.first);
                }

                other.pages.first = self.pages.first;

                let left_ref = unsafe { left.as_branch_mut::<BranchLayer>() };
                let right_ref = unsafe { right.as_branch_mut::<BranchLayer>() };

                unsafe {
                    ItemRef::join_to_right(left_ref, right_ref, self.length, other.length, refs)
                }
                .1
            }
        };

        other.length += self.length;

        if let Some(new_root) = new_root {
            other.height += 1;
            other.root = new_root;
        }

        self.height = 0;

        let _ = replace(self, other);
    }

    //Safety:
    // 1. `self` height equals to `right` height.
    // 2. Height is positive value.
    // 3. All references belong to `refs` instance.
    unsafe fn join_roots(&mut self, mut other: Self, refs: &mut TreeRefs<N>) {
        let left = &mut self.root;
        let right = &mut other.root;

        other.height = 0;

        let new_root = match self.height {
            0 => unsafe { ld_unreachable!("Incorrect height.") },

            1 => {
                let left_ref = unsafe { left.as_page_mut() };
                let right_ref = unsafe { right.as_page_mut() };

                let new_root = unsafe {
                    ItemRef::join_roots(left_ref, right_ref, self.length, other.length, refs)
                };

                if new_root.is_some() {
                    unsafe {
                        PageRef::interconnect(&mut self.pages.last, &mut other.pages.first);
                    }

                    self.pages.last = other.pages.last;
                }

                new_root
            }

            2 => {
                unsafe {
                    PageRef::interconnect(&mut self.pages.last, &mut other.pages.first);
                }

                self.pages.last = other.pages.last;

                let left_ref = unsafe { left.as_branch_mut::<PageLayer>() };
                let right_ref = unsafe { right.as_branch_mut::<PageLayer>() };

                unsafe { ItemRef::join_roots(left_ref, right_ref, self.length, other.length, refs) }
            }

            _ => {
                unsafe {
                    PageRef::interconnect(&mut self.pages.last, &mut other.pages.first);
                }

                self.pages.last = other.pages.last;

                let left_ref = unsafe { left.as_branch_mut::<BranchLayer>() };
                let right_ref = unsafe { right.as_branch_mut::<BranchLayer>() };

                unsafe { ItemRef::join_roots(left_ref, right_ref, self.length, other.length, refs) }
            }
        };

        self.length += other.length;

        if let Some(new_root) = new_root {
            self.height += 1;
            self.root = new_root;
        }
    }
}
