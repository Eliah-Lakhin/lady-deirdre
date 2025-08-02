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

use std::{iter::FusedIterator, mem::take};

use crate::{
    lexis::{session::Cursor, ByteIndex, LexisSession, Site, Token},
    report::{ld_assert, ld_assert_ne, system_panic},
};

///todo
pub struct TokenScanner<'a, T: Token> {
    input: &'a str,
    begin: Cursor<()>,
    end: Cursor<()>,
    current: Cursor<()>,
    pending: Option<T>,
}

unsafe impl<'a, T: Token> LexisSession for TokenScanner<'a, T> {
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
                system_panic!(
                    "Incorrect use of the LexisSession::submit function.\nA \
                    byte in front of the current cursor is UTF-8 continuation \
                    byte."
                );
            }
        }

        self.end = self.current;
    }
}

impl<'a, T: Token> Iterator for TokenScanner<'a, T> {
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

impl<'a, T: Token> FusedIterator for TokenScanner<'a, T> {}

impl<'a, T: Token> TokenScanner<'a, T> {
    ///todo
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
