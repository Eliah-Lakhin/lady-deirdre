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
    incremental::storage::ChildRefIndex,
    lexis::{
        utils::{get_lexis_character, NULL},
        ByteIndex,
        Length,
        LexisSession,
        Site,
        Token,
        TokenCount,
    },
    report::{debug_assert, debug_unreachable},
    std::*,
    syntax::Node,
};

pub(super) struct IncrementalLexisSession<'source, N: Node> {
    input: Input<'source>,
    product: Product<N>,
    next_cursor: Cursor<N>,
    begin_cursor: Cursor<N>,
    start_cursor: Cursor<N>,
    end_cursor: Cursor<N>,
    submission_site: Site,
    submission_string: String,
}

impl<'source, N: Node> LexisSession for IncrementalLexisSession<'source, N> {
    #[inline(always)]
    fn advance(&mut self) {
        self.next_cursor.advance(self.input);
    }

    #[inline(always)]
    fn character(&self) -> char {
        self.next_cursor.character
    }

    #[inline(always)]
    fn submit(&mut self) {
        self.end_cursor = self.next_cursor;
    }

    #[inline]
    fn substring(&mut self) -> &str {
        if self.end_cursor.site == self.submission_site {
            return self.submission_string.as_str();
        }

        self.submission_site = self.end_cursor.site;

        self.submission_string.clear();

        if self.start_cursor.site != self.end_cursor.site {
            substring_to(
                self.input,
                &self.start_cursor,
                &self.end_cursor,
                &mut self.submission_string,
            );
        }

        self.submission_string.as_str()
    }
}

impl<'source, N: Node> IncrementalLexisSession<'source, N> {
    //Safety:
    // 1. `tail` is a Page reference(possibly dangling).
    // 2. `tail`'s Tree is immutable during `'source` lifetime.
    // 3. `'source` does not outlive `tail`'s Tree.
    // 4. `input` is not empty.
    // 5. Each item in `input` is not empty.
    #[inline]
    pub(super) unsafe fn run(
        product_capacity: TokenCount,
        input: Input<'source>,
        tail: ChildRefIndex<N>,
    ) -> Product<N> {
        let start_character = match input.first() {
            Some(first) => {
                debug_assert!(
                    !first.is_empty(),
                    "Internal error. Empty input first string.",
                );

                unsafe { get_lexis_character(first.chars()) }
            }

            // Safety: Upheld by 4.
            None => unsafe { debug_unreachable!("Empty Lexer input.") },
        };

        let cursor = Cursor {
            site: 0,
            input_index: 0,
            input_byte: 0,
            character: start_character,
            tail_ref: tail,
            tail_length: 0,
        };

        let mut session = Self {
            input,
            product: Product {
                length: 0,
                spans: Vec::with_capacity(product_capacity),
                strings: Vec::with_capacity(product_capacity),
                tokens: Vec::with_capacity(product_capacity),
                tail_ref: tail,
                tail_length: 0,
            },
            next_cursor: cursor,
            begin_cursor: cursor,
            start_cursor: cursor,
            end_cursor: cursor,
            submission_site: 0,
            submission_string: String::new(),
        };

        loop {
            let token = <N::Token as Token>::new(&mut session);

            if session.start_cursor.site != session.end_cursor.site {
                let submission = session.get_submission();

                session.product.push(
                    token,
                    &session.start_cursor,
                    &session.end_cursor,
                    submission,
                );

                if session.try_finish() {
                    break;
                }

                continue;
            }

            if session.enter_mismatch_loop(token) {
                break;
            }
        }

        session.product
    }

    // Returns true if the parsing process supposed to stop
    #[inline]
    fn enter_mismatch_loop(&mut self, mismatch: N::Token) -> bool {
        loop {
            self.start_cursor.advance(self.input);
            self.next_cursor = self.start_cursor;

            if self.start_cursor.character == NULL {
                self.product.push(
                    mismatch,
                    &self.begin_cursor,
                    &self.start_cursor,
                    self.get_rejection(),
                );

                return true;
            }

            let token = <N::Token as Token>::new(self);

            if self.start_cursor.site < self.end_cursor.site {
                self.product.push(
                    mismatch,
                    &self.begin_cursor,
                    &self.start_cursor,
                    self.get_rejection(),
                );

                let submission = self.get_submission();

                self.product
                    .push(token, &self.start_cursor, &self.end_cursor, submission);

                return self.try_finish();
            }
        }
    }

    #[inline]
    fn get_submission(&mut self) -> String {
        if self.end_cursor.site != self.submission_site {
            self.submission_site = self.end_cursor.site;
            self.submission_string.clear();

            substring_to(
                self.input,
                &self.start_cursor,
                &self.end_cursor,
                &mut self.submission_string,
            );
        }

        return self.submission_string.clone();
    }

    #[inline]
    fn get_rejection(&self) -> String {
        let mut rejection = String::new();

        substring_to(
            self.input,
            &self.begin_cursor,
            &self.start_cursor,
            &mut rejection,
        );

        rejection
    }

    // Returns true if the parsing process supposed to stop
    #[inline(always)]
    fn try_finish(&mut self) -> bool {
        if self.end_cursor.character == NULL {
            return true;
        }

        if self.end_cursor.input_byte == 0 && self.end_cursor.input_index >= self.input.len() {
            return true;
        }

        self.reset();

        return false;
    }

    #[inline(always)]
    fn reset(&mut self) {
        self.begin_cursor = self.end_cursor;
        self.start_cursor = self.end_cursor;
        self.next_cursor = self.end_cursor;
        self.submission_string.clear();
    }
}

pub(super) type Input<'source> = &'source [&'source str];

#[inline]
fn substring_to<N: Node>(input: Input, from: &Cursor<N>, to: &Cursor<N>, target: &mut String) {
    if from.input_index == to.input_index {
        debug_assert!(
            from.input_byte <= to.input_byte,
            "Internal error. From cursor is ahead of To cursor.",
        );

        let string = match from.input_index < input.len() {
            true => unsafe { *input.get_unchecked(from.input_index) },

            false => match from.tail_ref.is_dangling() {
                true => unsafe { from.tail_ref.string() },
                false => "",
            },
        };

        target.push_str(unsafe { string.get_unchecked(from.input_byte..to.input_byte) });

        return;
    }

    let mut chunk_ref = from.tail_ref;

    for index in from.input_index..=to.input_index {
        let string = match index < input.len() {
            true => unsafe { *input.get_unchecked(index) },

            false => match chunk_ref.is_dangling() {
                false => {
                    let string = unsafe { from.tail_ref.string() };

                    unsafe { chunk_ref.next() };

                    string
                }
                true => "",
            },
        };

        if index == from.input_index {
            target.push_str(unsafe { string.get_unchecked(from.input_byte..) });
            continue;
        }

        if index == to.input_index {
            target.push_str(unsafe { string.get_unchecked(0..to.input_byte) });
            continue;
        }

        target.push_str(string);
    }
}

pub(super) struct Product<N: Node> {
    pub(super) length: Length,
    pub(super) spans: Vec<Length>,
    pub(super) strings: Vec<String>,
    pub(super) tokens: Vec<N::Token>,
    pub(super) tail_ref: ChildRefIndex<N>,
    pub(super) tail_length: Length,
}

impl<N: Node> Product<N> {
    #[inline(always)]
    pub(super) fn count(&self) -> TokenCount {
        self.spans.len()
    }

    #[inline(always)]
    fn push(&mut self, token: N::Token, from: &Cursor<N>, to: &Cursor<N>, string: String) {
        let span = to.site - from.site;

        self.length += span;

        self.spans.push(span);
        self.strings.push(string);
        self.tokens.push(token);
        self.tail_ref = to.tail_ref;
        self.tail_length = to.tail_length;
    }
}

struct Cursor<N: Node> {
    site: Site,
    input_index: usize,
    input_byte: ByteIndex,
    character: char,
    tail_ref: ChildRefIndex<N>,
    tail_length: Length,
}

impl<N: Node> Clone for Cursor<N> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Node> Copy for Cursor<N> {}

impl<N: Node> Cursor<N> {
    #[inline]
    fn advance(&mut self, input: Input) {
        if self.character == NULL {
            return;
        }

        self.site += 1;
        self.input_byte += self.character.len_utf8();

        match self.input_index < input.len() {
            true => {
                let string = unsafe { *input.get_unchecked(self.input_index) };

                if self.input_byte < string.len() {
                    self.character = unsafe {
                        get_lexis_character(string.get_unchecked(self.input_byte..).chars())
                    };

                    return;
                }

                self.input_index += 1;
                self.input_byte = 0;

                if self.input_index < input.len() {
                    let string = unsafe { input.get_unchecked(self.input_index) };

                    debug_assert!(!string.is_empty(), "Empty input string.");

                    self.character = unsafe { get_lexis_character(string.chars()) };

                    return;
                }

                if self.tail_ref.is_dangling() {
                    self.character = NULL;
                    return;
                }

                let string = unsafe { self.tail_ref.string() };

                debug_assert!(!string.is_empty(), "Empty tail string.");

                self.character = unsafe { get_lexis_character(string.chars()) };
            }

            false => {
                self.tail_length += 1;

                let string = unsafe { self.tail_ref.string() };

                if self.input_byte < string.len() {
                    self.character = unsafe {
                        get_lexis_character(string.get_unchecked(self.input_byte..).chars())
                    };

                    return;
                }

                self.input_index += 1;
                self.input_byte = 0;

                unsafe { self.tail_ref.next() }

                if self.tail_ref.is_dangling() {
                    self.character = NULL;
                    return;
                }

                let string = unsafe { self.tail_ref.string() };

                debug_assert!(!string.is_empty(), "Empty tail string.");

                self.character = unsafe { get_lexis_character(string.chars()) };
            }
        }
    }
}
