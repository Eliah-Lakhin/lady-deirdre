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
    arena::{Id, Identifiable},
    incremental::{storage::ChildRefIndex, Document},
    lexis::{Length, Site, SiteRef, SiteSpan, TokenCount, TokenCursor, TokenRef},
    report::debug_assert,
    std::*,
    syntax::Node,
};

pub struct DocumentCursor<'document, N: Node> {
    document: &'document Document<N>,
    next_chunk_ref: ChildRefIndex<N>,
    end_chunk_ref: ChildRefIndex<N>,
    peek_chunk_ref: ChildRefIndex<N>,
    peek_distance: TokenCount,
}

impl<'document, N: Node> Identifiable for DocumentCursor<'document, N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.document.id()
    }
}

impl<'document, N: Node> TokenCursor<'document> for DocumentCursor<'document, N> {
    type Token = N::Token;

    #[inline(always)]
    fn advance(&mut self) -> bool {
        if unsafe { self.next_chunk_ref.same_chunk_as(&self.end_chunk_ref) } {
            return false;
        }

        unsafe { self.next_chunk_ref.next() };

        match self.peek_distance == 0 {
            true => {
                self.peek_chunk_ref = self.next_chunk_ref;
            }

            false => {
                self.peek_distance -= 1;
            }
        }

        true
    }

    #[inline(always)]
    fn token(&mut self, distance: TokenCount) -> Option<&'document Self::Token> {
        if unsafe { self.next_chunk_ref.same_chunk_as(&self.end_chunk_ref) } {
            return None;
        }

        if unsafe { self.jump(distance) } {
            return None;
        }

        Some(unsafe { self.peek_chunk_ref.token() })
    }

    #[inline(always)]
    fn site(&mut self, distance: TokenCount) -> Option<Site> {
        if unsafe { self.next_chunk_ref.same_chunk_as(&self.end_chunk_ref) } {
            return None;
        }

        if unsafe { self.jump(distance) } {
            return None;
        }

        Some(unsafe { self.document.tree().site_of(&self.peek_chunk_ref) })
    }

    #[inline(always)]
    fn length(&mut self, distance: TokenCount) -> Option<Length> {
        if unsafe { self.next_chunk_ref.same_chunk_as(&self.end_chunk_ref) } {
            return None;
        }

        if unsafe { self.jump(distance) } {
            return None;
        }

        Some(*unsafe { self.peek_chunk_ref.span() })
    }

    #[inline(always)]
    fn string(&mut self, distance: TokenCount) -> Option<&'document str> {
        if unsafe { self.next_chunk_ref.same_chunk_as(&self.end_chunk_ref) } {
            return None;
        }

        if unsafe { self.jump(distance) } {
            return None;
        }

        Some(unsafe { self.peek_chunk_ref.string() })
    }

    #[inline(always)]
    fn token_ref(&mut self, distance: TokenCount) -> TokenRef {
        if unsafe { self.next_chunk_ref.same_chunk_as(&self.end_chunk_ref) } {
            return TokenRef::nil();
        }

        if unsafe { self.jump(distance) } {
            return TokenRef::nil();
        }

        let ref_index = unsafe { self.peek_chunk_ref.chunk_ref_index() };

        let chunk_ref = unsafe { self.document.references.chunks().make_ref(ref_index) };

        TokenRef {
            id: self.document.id(),
            chunk_ref,
        }
    }

    #[inline(always)]
    fn site_ref(&mut self, distance: TokenCount) -> SiteRef {
        if unsafe { self.next_chunk_ref.same_chunk_as(&self.end_chunk_ref) } {
            return self.end_site_ref();
        }

        if unsafe { self.jump(distance) } {
            return self.end_site_ref();
        }

        let ref_index = unsafe { self.peek_chunk_ref.chunk_ref_index() };

        let chunk_ref = unsafe { self.document.references.chunks().make_ref(ref_index) };

        TokenRef {
            id: self.document.id(),
            chunk_ref,
        }
        .site_ref()
    }

    #[inline(always)]
    fn end_site_ref(&mut self) -> SiteRef {
        if self.end_chunk_ref.is_dangling() {
            return SiteRef::new_code_end(self.document.id());
        }

        let ref_index = unsafe { self.end_chunk_ref.chunk_ref_index() };

        let chunk_ref = unsafe { self.document.references.chunks().make_ref(ref_index) };

        TokenRef {
            id: self.document.id(),
            chunk_ref,
        }
        .site_ref()
    }
}

impl<'document, N: Node> DocumentCursor<'document, N> {
    pub(super) fn new(document: &'document Document<N>, mut span: SiteSpan) -> Self {
        let mut next_chunk_ref = document.tree().lookup(&mut span.start);
        let mut end_chunk_ref = document.tree().lookup(&mut span.end);

        if next_chunk_ref.is_dangling() {
            next_chunk_ref = document.tree().last();
        } else if span.start == 0 && unsafe { !next_chunk_ref.is_first() } {
            unsafe { next_chunk_ref.back() };
        }

        if !end_chunk_ref.is_dangling() {
            unsafe { end_chunk_ref.next() };
        }

        Self {
            document,
            next_chunk_ref,
            end_chunk_ref,
            peek_chunk_ref: next_chunk_ref,
            peek_distance: 0,
        }
    }

    // Returns `true` if jump has failed.
    // Safety: `self.next_chunk_ref` behind the `self.end_chunk_ref`.
    #[inline]
    unsafe fn jump(&mut self, target: TokenCount) -> bool {
        while self.peek_distance < target {
            self.peek_distance += 1;

            unsafe { self.peek_chunk_ref.next() };

            if unsafe { self.peek_chunk_ref.same_chunk_as(&self.end_chunk_ref) } {
                self.peek_distance = 0;
                self.peek_chunk_ref = self.next_chunk_ref;
                return true;
            }
        }

        if self.peek_distance > target * 2 {
            self.peek_distance = 0;
            self.peek_chunk_ref = self.next_chunk_ref;

            while self.peek_distance < target {
                self.peek_distance += 1;

                unsafe { self.peek_chunk_ref.next() };

                debug_assert!(!self.peek_chunk_ref.is_dangling(), "Dangling peek ref.");
            }

            return false;
        }

        while self.peek_distance > target {
            unsafe { self.peek_chunk_ref.back() }

            debug_assert!(!self.peek_chunk_ref.is_dangling(), "Dangling peek ref.");

            self.peek_distance -= 1;
        }

        false
    }
}
