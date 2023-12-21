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
    lexis::{Length, SiteSpan},
    report::debug_unreachable,
    std::*,
    syntax::Node,
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
            unsafe { debug_unreachable!("Remaining length exceeds unit length.") }
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
            unsafe { debug_unreachable!("Dangling cursor.") }
        }

        let mut inner = unsafe { cursor.page_string() }.chars();

        while span.start > 0 {
            span.start -= 1;

            if inner.next().is_none() {
                unsafe { debug_unreachable!("Page string is too short.") }
            }
        }

        Self {
            cursor,
            inner,
            remaining,
        }
    }
}
