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

#[cfg(debug_assertions)]
use crate::report::system_panic;
use crate::{
    lexis::{ByteIndex, Length, LexisSession, Site, Token, TokenCount, CHUNK_SIZE},
    report::{debug_assert, debug_assert_ne, debug_unreachable},
    std::*,
    syntax::Node,
    units::storage::ChildCursor,
};

pub(super) struct MutableLexisSession<'source, N: Node> {
    input: SessionInput<'source>,
    last: usize,
    output: SessionOutput<N>,
    begin: Cursor<N>,
    end: Cursor<N>,
    current: Cursor<N>,
}

unsafe impl<'source, N: Node> LexisSession for MutableLexisSession<'source, N> {
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        self.current.advance(self.input)
    }

    #[inline(always)]
    unsafe fn consume(&mut self) {
        self.current.consume(self.input)
    }

    #[inline(always)]
    unsafe fn read(&mut self) -> char {
        self.current.read(self.input)
    }

    #[inline(always)]
    unsafe fn submit(&mut self) {
        #[cfg(debug_assertions)]
        {
            let string = match self.current.index < self.input.len() {
                true => Some(self.input[self.current.index]),

                false => match self.current.tail.is_dangling() {
                    true => None,
                    false => Some(unsafe { self.current.tail.string() }),
                },
            };

            if let Some(string) = string {
                if self.current.byte < string.len() {
                    let byte = string.as_bytes()[self.current.byte];

                    if byte & 0xC0 == 0x80 {
                        system_panic!(
                            "Incorrect use of the LexisSession::submit \
                            function.\nA byte in front of the current cursor \
                            is UTF-8 continuation byte."
                        );
                    }
                }
            }
        }

        self.end = self.current;
    }
}

impl<'source, N: Node> MutableLexisSession<'source, N> {
    //Safety:
    // 1. `tail` is a Page reference(possibly dangling).
    // 2. `tail`'s Tree is immutable during `'source` lifetime.
    // 3. `'source` does not outlive `tail`'s Tree.
    // 4. `input` is not empty.
    // 5. Each item in `input` is not empty.
    #[inline]
    pub(super) unsafe fn run(
        product_capacity: TokenCount,
        input: SessionInput<'source>,
        tail: ChildCursor<N>,
    ) -> SessionOutput<N> {
        let last = match input.len().checked_sub(1) {
            Some(last) => last,
            None => debug_unreachable!("Empty input buffer."),
        };

        let cursor = Cursor {
            index: 0,
            byte: 0,
            site: 0,
            tail,
            overlap: 0,
        };

        let mut session = Self {
            input,
            last,
            output: SessionOutput {
                length: 0,
                spans: Vec::with_capacity(product_capacity),
                indices: Vec::with_capacity(product_capacity),
                tokens: Vec::with_capacity(product_capacity),
                text: String::with_capacity(product_capacity * CHUNK_SIZE),
                tail,
                overlap: 0,
            },
            begin: cursor,
            end: cursor,
            current: cursor,
        };

        loop {
            let token = <N::Token as Token>::parse(&mut session);

            if session.begin.site != session.end.site {
                session
                    .output
                    .push(session.input, token, &session.begin, &session.end);

                if session.finished() {
                    break;
                }

                session.begin = session.end;
                session.current = session.end;

                continue;
            }

            if session.enter_mismatch_loop() {
                break;
            }
        }

        session.output
    }

    // Returns true if the parsing process supposed to stop
    #[inline]
    fn enter_mismatch_loop(&mut self) -> bool {
        let mismatch = self.begin;

        loop {
            if self.begin.advance(self.input) == 0xFF {
                self.output.push(
                    self.input,
                    <N::Token as Token>::mismatch(),
                    &mismatch,
                    &self.begin,
                );
                return true;
            }

            self.begin.consume(self.input);

            self.end = self.begin;
            self.current = self.begin;

            let token = <N::Token as Token>::parse(self);

            if self.begin.site == self.end.site {
                continue;
            }

            self.output.push(
                self.input,
                <N::Token as Token>::mismatch(),
                &mismatch,
                &self.begin,
            );

            self.output.push(self.input, token, &self.begin, &self.end);

            if self.finished() {
                return true;
            }

            self.begin = self.end;
            self.current = self.end;

            return false;
        }
    }

    // Returns true if the parsing process supposed to stop
    #[inline]
    fn finished(&mut self) -> bool {
        if self.end.index < self.last {
            return false;
        }

        if self.end.index == self.last {
            return match self.end.tail.is_dangling() {
                false => false,

                true => {
                    let string = *unsafe { self.input.get_unchecked(self.end.index) };

                    self.end.byte == string.len()
                }
            };
        }

        if !self.end.tail.is_dangling() {
            let string = unsafe { self.end.tail.string() };

            if self.end.byte < string.len() {
                return false;
            }

            if self.end.index > self.last {
                unsafe { self.output.tail.next() };
            }
        }

        true
    }
}

pub(super) type SessionInput<'source> = &'source [&'source str];

pub(super) struct SessionOutput<N: Node> {
    pub(super) length: Length,
    pub(super) spans: Vec<Length>,
    pub(super) indices: Vec<ByteIndex>,
    pub(super) tokens: Vec<N::Token>,
    pub(super) text: String,
    pub(super) tail: ChildCursor<N>,
    pub(super) overlap: Length,
}

impl<N: Node> SessionOutput<N> {
    #[inline(always)]
    pub(super) fn count(&self) -> TokenCount {
        self.spans.len()
    }

    #[inline(always)]
    fn push(&mut self, input: SessionInput, token: N::Token, from: &Cursor<N>, to: &Cursor<N>) {
        let span = to.site - from.site;

        debug_assert!(span > 0, "Empty span.");

        self.length += span;
        self.spans.push(span);
        self.indices.push(self.text.len());
        self.tokens.push(token);
        self.tail = to.tail;
        self.overlap = to.overlap;

        substring_to(input, from, to, &mut self.text);
    }
}

struct Cursor<N: Node> {
    index: usize,
    byte: ByteIndex,
    site: Site,
    tail: ChildCursor<N>,
    overlap: Length,
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
    fn advance(&mut self, input: SessionInput) -> u8 {
        match self.index < input.len() {
            true => {
                let string = *unsafe { input.get_unchecked(self.index) };

                debug_assert!(!string.is_empty(), "Empty input string.");

                if self.byte < string.len() {
                    let point = *unsafe { string.as_bytes().get_unchecked(self.byte) };

                    if point & 0xC0 != 0x80 {
                        self.site += 1;
                    }

                    self.byte += 1;

                    return point;
                }

                self.index += 1;

                if self.index < input.len() {
                    let string = *unsafe { input.get_unchecked(self.index) };

                    debug_assert!(!string.is_empty(), "Empty input string.");

                    let point = *unsafe { string.as_bytes().get_unchecked(0) };

                    self.site += 1;
                    self.byte = 1;

                    return point;
                }

                if self.tail.is_dangling() {
                    return 0xFF;
                }

                let string = unsafe { self.tail.string() };

                let point = *unsafe { string.as_bytes().get_unchecked(0) };

                self.byte = 1;
                self.site += 1;
                self.overlap = 1;

                point
            }

            false => {
                if self.tail.is_dangling() {
                    return 0xFF;
                }

                let string = unsafe { self.tail.string() };

                if self.byte < string.len() {
                    let point = *unsafe { string.as_bytes().get_unchecked(self.byte) };

                    if point & 0xC0 != 0x80 {
                        self.site += 1;
                        self.overlap += 1;
                    }

                    self.byte += 1;

                    return point;
                }

                unsafe { self.tail.next() };

                self.index += 1;

                if self.tail.is_dangling() {
                    self.byte = 0;
                    return 0xFF;
                }

                let string = unsafe { self.tail.string() };

                let point = *unsafe { string.as_bytes().get_unchecked(0) };

                self.byte = 1;
                self.site += 1;
                self.overlap += 1;

                point
            }
        }
    }

    #[inline(always)]
    fn consume(&mut self, input: SessionInput) {
        let (byte, string) = match self.index < input.len() {
            true => {
                let string = *unsafe { input.get_unchecked(self.index) };
                (&mut self.byte, string)
            }
            false => {
                #[cfg(debug_assertions)]
                if self.tail.is_dangling() {
                    system_panic!(
                        "Incorrect use of the LexisSession::consume \
                        function\nEnd of input has been reached.",
                    );
                }

                let string = unsafe { self.tail.string() };
                (&mut self.byte, string)
            }
        };

        debug_assert!(
            *byte > 0,
            "Incorrect use of the LexisSession::consume function.\nCurrent \
            cursor is in the beginning of the input stream.",
        );

        let point = string.as_bytes()[*byte - 1];

        debug_assert_ne!(
            point & 0xC0,
            0x80,
            "Incorrect use of the LexisSession::consume function.\nA byte \
            before the current cursor is not a UTF-8 code point start byte."
        );

        if point & 0x80 == 0 {
            return;
        }

        if point & 0xF0 == 0xF0 {
            *byte += 3;
            return;
        }

        if point & 0xE0 == 0xE0 {
            *byte += 2;
            return;
        }

        if point & 0xC0 == 0xC0 {
            *byte += 1;
            return;
        }
    }

    #[inline(always)]
    fn read(&mut self, input: SessionInput) -> char {
        let (byte, string) = match self.index < input.len() {
            true => {
                let string = *unsafe { input.get_unchecked(self.index) };
                (&mut self.byte, string)
            }
            false => {
                #[cfg(debug_assertions)]
                if self.tail.is_dangling() {
                    system_panic!(
                        "Incorrect use of the LexisSession::read \
                        function\nEnd of input has been reached.",
                    );
                }

                let string = unsafe { self.tail.string() };
                (&mut self.byte, string)
            }
        };

        debug_assert!(
            *byte > 0,
            "Incorrect use of the LexisSession::read function.\nCurrent cursor \
            is in the beginning of the input stream."
        );

        let before = *byte - 1;

        #[cfg(debug_assertions)]
        {
            let point = string.as_bytes()[before];

            if point & 0xC0 == 0x80 {
                system_panic!(
                    "Incorrect use of the LexisSession::read function.\nA byte \
                    before the current cursor is not a UTF-8 code point start \
                    byte."
                )
            }
        }

        let rest = unsafe { string.get_unchecked(before..) };
        let ch = unsafe { rest.chars().next().unwrap_unchecked() };
        let len = ch.len_utf8();

        *byte += len - 1;

        ch
    }
}

#[inline]
fn substring_to<N: Node>(
    input: SessionInput,
    from: &Cursor<N>,
    to: &Cursor<N>,
    target: &mut String,
) {
    if from.index == to.index {
        debug_assert!(from.byte <= to.byte, "From cursor is ahead of To cursor.",);

        let string = match from.index < input.len() {
            true => unsafe { *input.get_unchecked(from.index) },

            false => match from.tail.is_dangling() {
                true => unsafe { from.tail.string() },
                false => "",
            },
        };

        target.push_str(unsafe { string.get_unchecked(from.byte..to.byte) });

        return;
    }

    let mut chunk_cursor = from.tail;

    for index in from.index..=to.index {
        let string = match index < input.len() {
            true => unsafe { *input.get_unchecked(index) },

            false => match chunk_cursor.is_dangling() {
                false => {
                    let string = unsafe { chunk_cursor.string() };

                    unsafe { chunk_cursor.next() };

                    string
                }
                true => "",
            },
        };

        if index == from.index {
            target.push_str(unsafe { string.get_unchecked(from.byte..) });
            continue;
        }

        if index == to.index {
            target.push_str(unsafe { string.get_unchecked(0..to.byte) });
            continue;
        }

        target.push_str(string);
    }
}
