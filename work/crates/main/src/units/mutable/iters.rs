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

use std::{iter::FusedIterator, str::Chars};

use crate::{
    arena::{Id, RepoEntriesIter},
    lexis::{Length, SiteSpan},
    report::ld_unreachable,
    syntax::{ErrorRef, Node, NodeRef, SyntaxError},
    units::{storage::ChildCursor, MutableUnit},
};

pub struct MutableCharIter<'unit, N: Node> {
    cursor: ChildCursor<N>,
    inner: Chars<'unit>,
    remaining: Length,
}

impl<'unit, N: Node> Iterator for MutableCharIter<'unit, N> {
    type Item = char;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        self.remaining -= 1;

        if let Some(next) = self.inner.next() {
            return Some(next);
        }

        unsafe { self.cursor.next_page() };

        if self.cursor.is_dangling() {
            unsafe { ld_unreachable!("Remaining length exceeds unit length.") }
        }

        let string = unsafe { self.cursor.page_string() };

        self.inner = string.chars();

        self.inner.next()
    }
}

impl<'unit, N: Node> FusedIterator for MutableCharIter<'unit, N> {}

impl<'unit, N: Node> MutableCharIter<'unit, N> {
    // Safety: `span` is valid for this unit.
    #[inline(always)]
    pub(super) unsafe fn new(unit: &'unit MutableUnit<N>, mut span: SiteSpan) -> Self {
        let remaining = span.end - span.start;

        if remaining == 0 {
            return Self {
                cursor: ChildCursor::dangling(),
                inner: "".chars(),
                remaining,
            };
        }

        let cursor = unit.tree().lookup(&mut span.start);

        if cursor.is_dangling() {
            unsafe { ld_unreachable!("Dangling cursor.") }
        }

        let mut inner = unsafe { cursor.page_string() }.chars();

        while span.start > 0 {
            span.start -= 1;

            if inner.next().is_none() {
                unsafe { ld_unreachable!("Page string is too short.") }
            }
        }

        Self {
            cursor,
            inner,
            remaining,
        }
    }
}

pub struct MutableNodeIter<'unit, N: Node> {
    pub(super) id: Id,
    pub(super) inner: RepoEntriesIter<'unit, N>,
}

impl<'unit, N: Node> Iterator for MutableNodeIter<'unit, N> {
    type Item = NodeRef;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.inner.next()?;

        Some(NodeRef { id: self.id, entry })
    }
}

impl<'unit, N: Node> FusedIterator for MutableNodeIter<'unit, N> {}

pub struct MutableErrorIter<'unit> {
    pub(super) id: Id,
    pub(super) inner: RepoEntriesIter<'unit, SyntaxError>,
}

impl<'unit> Iterator for MutableErrorIter<'unit> {
    type Item = ErrorRef;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.inner.next()?;

        Some(ErrorRef { id: self.id, entry })
    }
}

impl<'unit> FusedIterator for MutableErrorIter<'unit> {}
