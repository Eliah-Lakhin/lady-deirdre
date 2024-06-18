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

use std::mem::take;

use lady_deirdre::lexis::{ByteIndex, LexisSession, Site, Token};

pub struct LDStatelessScanner<'a, T: Token> {
    input: &'a str,
    begin: Cursor,
    end: Cursor,
    current: Cursor,
    pending: Option<T>,
}

unsafe impl<'a, T: Token> LexisSession for LDStatelessScanner<'a, T> {
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
        if self.current.byte < self.input.len() {
            let byte = self.input.as_bytes()[self.current.byte];

            if byte & 0xC0 == 0x80 {
                unreachable!()
            }
        }

        self.end = self.current;
    }
}

impl<'a, T: Token> LDStatelessScanner<'a, T> {
    #[inline(always)]
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            begin: Cursor::default(),
            end: Cursor::default(),
            current: Cursor::default(),
            pending: None,
        }
    }
}

impl<'a, T: Token> Iterator for LDStatelessScanner<'a, T> {
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pending) = take(&mut self.pending) {
            return Some(pending);
        }

        if self.begin.byte == self.input.len() {
            return None;
        }

        let token = T::scan(self);

        if self.begin.byte != self.end.byte {
            self.begin = self.end;
            self.current = self.end;

            return Some(token);
        }

        loop {
            if self.begin.advance(self.input) == 0xFF {
                return Some(T::mismatch());
            }

            self.begin.consume(self.input);

            self.end = self.begin;
            self.current = self.begin;

            let token = T::scan(self);

            if self.begin.byte == self.end.byte {
                continue;
            }

            self.pending = Some(token);

            self.begin = self.end;
            self.current = self.end;

            return Some(T::mismatch());
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Cursor {
    byte: ByteIndex,
}

impl Cursor {
    #[inline(always)]
    fn advance(&mut self, text: &str) -> u8 {
        if self.byte == text.len() {
            return 0xFF;
        }

        let point = *unsafe { text.as_bytes().get_unchecked(self.byte) };

        self.byte += 1;

        point
    }

    #[inline(always)]
    fn consume(&mut self, text: &str) {
        debug_assert!(self.byte > 0);

        let point = text.as_bytes()[self.byte - 1];

        debug_assert_ne!(point & 0xC0, 0x80);

        if point & 0x80 == 0 {
            return;
        }

        if point & 0xF0 == 0xF0 {
            self.byte += 3;
            return;
        }

        if point & 0xE0 == 0xE0 {
            self.byte += 2;
            return;
        }

        if point & 0xC0 == 0xC0 {
            self.byte += 1;
            return;
        }
    }

    #[inline(always)]
    fn read(&mut self, text: &str) -> char {
        debug_assert!(self.byte > 0);

        let byte = self.byte - 1;

        #[cfg(debug_assertions)]
        {
            let point = text.as_bytes()[byte];

            if point & 0xC0 == 0x80 {
                unreachable!()
            }
        }

        let rest = unsafe { text.get_unchecked(byte..) };
        let ch = unsafe { rest.chars().next().unwrap_unchecked() };
        let len = ch.len_utf8();

        self.byte += len - 1;

        ch
    }
}
