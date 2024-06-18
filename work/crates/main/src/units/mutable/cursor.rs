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
    arena::{Id, Identifiable},
    lexis::{Length, Site, SiteRef, SiteSpan, Token, TokenCount, TokenCursor, TokenRef},
    report::ld_assert,
    syntax::Node,
    units::{storage::ChildCursor, MutableUnit},
};

pub struct MutableCursor<'unit, N: Node> {
    unit: &'unit MutableUnit<N>,
    next_chunk_cursor: ChildCursor<N>,
    peek_chunk_cursor: ChildCursor<N>,
    peek_distance: TokenCount,
    end_chunk_cursor: ChildCursor<N>,
}

impl<'unit, N: Node> Identifiable for MutableCursor<'unit, N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.unit.id()
    }
}

impl<'unit, N: Node> TokenCursor<'unit> for MutableCursor<'unit, N> {
    type Token = N::Token;

    #[inline(always)]
    fn advance(&mut self) -> bool {
        if unsafe { self.next_chunk_cursor.same_chunk_as(&self.end_chunk_cursor) } {
            return false;
        }

        unsafe { self.next_chunk_cursor.next() };

        match self.peek_distance == 0 {
            true => {
                self.peek_chunk_cursor = self.next_chunk_cursor;
            }

            false => {
                self.peek_distance -= 1;
            }
        }

        true
    }

    #[inline(always)]
    fn skip(&mut self, mut distance: TokenCount) {
        if distance == self.peek_distance {
            self.next_chunk_cursor = self.peek_chunk_cursor;
            self.peek_distance = 0;
            return;
        }

        while distance > 0 {
            distance -= 1;

            if !self.advance() {
                break;
            }
        }
    }

    #[inline(always)]
    fn token(&mut self, distance: TokenCount) -> Self::Token {
        if unsafe { self.next_chunk_cursor.same_chunk_as(&self.end_chunk_cursor) } {
            return <Self::Token as Token>::eoi();
        }

        if unsafe { self.jump(distance) } {
            return <Self::Token as Token>::eoi();
        }

        unsafe { self.peek_chunk_cursor.token() }
    }

    #[inline(always)]
    fn site(&mut self, distance: TokenCount) -> Option<Site> {
        if unsafe { self.next_chunk_cursor.same_chunk_as(&self.end_chunk_cursor) } {
            return None;
        }

        if unsafe { self.jump(distance) } {
            return None;
        }

        Some(unsafe { self.unit.tree().site_of(&self.peek_chunk_cursor) })
    }

    #[inline(always)]
    fn length(&mut self, distance: TokenCount) -> Option<Length> {
        if unsafe { self.next_chunk_cursor.same_chunk_as(&self.end_chunk_cursor) } {
            return None;
        }

        if unsafe { self.jump(distance) } {
            return None;
        }

        Some(*unsafe { self.peek_chunk_cursor.span() })
    }

    #[inline(always)]
    fn string(&mut self, distance: TokenCount) -> Option<&'unit str> {
        if unsafe { self.next_chunk_cursor.same_chunk_as(&self.end_chunk_cursor) } {
            return None;
        }

        if unsafe { self.jump(distance) } {
            return None;
        }

        Some(unsafe { self.peek_chunk_cursor.string() })
    }

    #[inline(always)]
    fn token_ref(&mut self, distance: TokenCount) -> TokenRef {
        if unsafe { self.next_chunk_cursor.same_chunk_as(&self.end_chunk_cursor) } {
            return TokenRef::nil();
        }

        if unsafe { self.jump(distance) } {
            return TokenRef::nil();
        }

        let entry_index = unsafe { self.peek_chunk_cursor.chunk_entry_index() };

        let chunk_entry = unsafe { self.unit.refs().chunks.entry_of_unchecked(entry_index) };

        TokenRef {
            id: self.unit.id(),
            entry: chunk_entry,
        }
    }

    #[inline(always)]
    fn site_ref(&mut self, distance: TokenCount) -> SiteRef {
        if unsafe { self.next_chunk_cursor.same_chunk_as(&self.end_chunk_cursor) } {
            return self.end_site_ref();
        }

        if unsafe { self.jump(distance) } {
            return self.end_site_ref();
        }

        let entry_index = unsafe { self.peek_chunk_cursor.chunk_entry_index() };

        let chunk_entry = unsafe { self.unit.refs().chunks.entry_of_unchecked(entry_index) };

        TokenRef {
            id: self.unit.id(),
            entry: chunk_entry,
        }
        .site_ref()
    }

    #[inline(always)]
    fn end_site_ref(&mut self) -> SiteRef {
        if self.end_chunk_cursor.is_dangling() {
            return SiteRef::end_of(self.unit.id());
        }

        let entry_index = unsafe { self.end_chunk_cursor.chunk_entry_index() };

        let chunk_entry = unsafe { self.unit.refs().chunks.entry_of_unchecked(entry_index) };

        TokenRef {
            id: self.unit.id(),
            entry: chunk_entry,
        }
        .site_ref()
    }
}

impl<'unit, N: Node> MutableCursor<'unit, N> {
    pub(super) fn new(unit: &'unit MutableUnit<N>, mut span: SiteSpan) -> Self {
        let mut next_chunk_cursor = unit.tree().lookup(&mut span.start);
        let mut end_chunk_cursor = unit.tree().lookup(&mut span.end);

        if next_chunk_cursor.is_dangling() {
            next_chunk_cursor = unit.tree().last();
        } else if span.start == 0 && unsafe { !next_chunk_cursor.is_first() } {
            unsafe { next_chunk_cursor.back() };
        }

        if !end_chunk_cursor.is_dangling() {
            unsafe { end_chunk_cursor.next() };
        }

        Self {
            unit,
            next_chunk_cursor,
            peek_chunk_cursor: next_chunk_cursor,
            peek_distance: 0,
            end_chunk_cursor,
        }
    }

    // Returns `true` if jump has failed.
    // Safety: `self.next_child_cursor` behind the `self.end_child_cursor`.
    #[inline]
    unsafe fn jump(&mut self, target: TokenCount) -> bool {
        while self.peek_distance < target {
            self.peek_distance += 1;

            unsafe { self.peek_chunk_cursor.next() };

            if unsafe { self.peek_chunk_cursor.same_chunk_as(&self.end_chunk_cursor) } {
                self.peek_distance = 0;
                self.peek_chunk_cursor = self.next_chunk_cursor;
                return true;
            }
        }

        if self.peek_distance > target * 2 {
            self.peek_distance = 0;
            self.peek_chunk_cursor = self.next_chunk_cursor;

            while self.peek_distance < target {
                self.peek_distance += 1;

                unsafe { self.peek_chunk_cursor.next() };

                ld_assert!(!self.peek_chunk_cursor.is_dangling(), "Dangling peek ref.");
            }

            return false;
        }

        while self.peek_distance > target {
            unsafe { self.peek_chunk_cursor.back() }

            ld_assert!(!self.peek_chunk_cursor.is_dangling(), "Dangling peek ref.");

            self.peek_distance -= 1;
        }

        false
    }
}
