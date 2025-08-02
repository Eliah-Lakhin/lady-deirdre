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
    lexis::{session::Cursor, ByteIndex, Chunk, LexisSession, Site, Token},
    report::{ld_assert, ld_assert_ne, system_panic},
};

/// todo
pub struct ChunkScanner<'input, T: Token> {
    text: &'input str,
    begin: Cursor<Site>,
    end: Cursor<Site>,
    current: Cursor<Site>,
    pending: Option<Chunk<'input, T>>,
}

unsafe impl<'input, T: Token> LexisSession for ChunkScanner<'input, T> {
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        self.current.advance(self.text)
    }

    #[inline(always)]
    unsafe fn consume(&mut self) {
        self.current.consume(self.text)
    }

    #[inline(always)]
    unsafe fn read(&mut self) -> char {
        self.current.read(self.text)
    }

    #[inline(always)]
    unsafe fn submit(&mut self) {
        #[cfg(debug_assertions)]
        if self.current.byte < self.text.len() {
            let byte = self.text.as_bytes()[self.current.byte];

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

impl<'input, T: Token> Iterator for ChunkScanner<'input, T> {
    type Item = Chunk<'input, T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pending) = take(&mut self.pending) {
            return Some(pending);
        }

        if self.begin.byte == self.text.len() {
            return None;
        }

        let token = T::scan(self);

        if self.begin.byte != self.end.byte {
            let chunk = self.chunk(token, &self.begin, &self.end);

            self.begin = self.end;
            self.current = self.end;

            return Some(chunk);
        }

        let mismatch = self.begin;

        loop {
            if self.begin.advance(&self.text) == 0xFF {
                return Some(self.chunk(T::mismatch(), &mismatch, &self.begin));
            }

            self.begin.consume(&self.text);

            self.end = self.begin;
            self.current = self.begin;

            let token = T::scan(self);

            if self.begin.byte == self.end.byte {
                continue;
            }

            let result = self.chunk(T::mismatch(), &mismatch, &self.begin);
            let pending = self.chunk(token, &self.begin, &self.end);

            self.pending = Some(pending);

            self.begin = self.end;
            self.current = self.end;

            return Some(result);
        }
    }
}

impl<'input, T: Token> FusedIterator for ChunkScanner<'input, T> {}

impl<'input, T: Token> ChunkScanner<'input, T> {
    ///todo
    #[inline(always)]
    pub fn new(input: &'input str) -> Self {
        Self {
            text: input,
            begin: Cursor::default(),
            end: Cursor::default(),
            current: Cursor::default(),
            pending: None,
        }
    }

    ///todo
    #[inline(always)]
    pub fn as_str(&self) -> &'input str {
        self.text
    }

    #[inline(always)]
    fn chunk(&self, token: T, from: &Cursor<Site>, to: &Cursor<Site>) -> Chunk<'input, T> {
        let length = to.site - from.site;

        ld_assert!(length > 0, "Empty length.");
        ld_assert!(from.byte < to.byte, "Invalid range.");
        ld_assert!(to.byte <= self.text.len(), "Invalid range.");

        let site = from.site;
        let string = unsafe { self.text.get_unchecked(from.byte..to.byte) };

        Chunk {
            token,
            site,
            length,
            string,
        }
    }
}

/// todo
pub struct ChunkIndicesScanner<'input, T: Token> {
    text: &'input str,
    begin: Cursor<Site>,
    end: Cursor<Site>,
    current: Cursor<Site>,
    pending: Option<(ByteIndex, Chunk<'input, T>)>,
}

unsafe impl<'input, T: Token> LexisSession for ChunkIndicesScanner<'input, T> {
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        self.current.advance(self.text)
    }

    #[inline(always)]
    unsafe fn consume(&mut self) {
        self.current.consume(self.text)
    }

    #[inline(always)]
    unsafe fn read(&mut self) -> char {
        self.current.read(self.text)
    }

    #[inline(always)]
    unsafe fn submit(&mut self) {
        #[cfg(debug_assertions)]
        if self.current.byte < self.text.len() {
            let byte = self.text.as_bytes()[self.current.byte];

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

impl<'input, T: Token> Iterator for ChunkIndicesScanner<'input, T> {
    type Item = (ByteIndex, Chunk<'input, T>);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pending) = take(&mut self.pending) {
            return Some(pending);
        }

        if self.begin.byte == self.text.len() {
            return None;
        }

        let token = T::scan(self);

        if self.begin.byte != self.end.byte {
            let chunk = self.chunk(token, &self.begin, &self.end);

            self.begin = self.end;
            self.current = self.end;

            return Some(chunk);
        }

        let mismatch = self.begin;

        loop {
            if self.begin.advance(&self.text) == 0xFF {
                return Some(self.chunk(T::mismatch(), &mismatch, &self.begin));
            }

            self.begin.consume(&self.text);

            self.end = self.begin;
            self.current = self.begin;

            let token = T::scan(self);

            if self.begin.byte == self.end.byte {
                continue;
            }

            let result = self.chunk(T::mismatch(), &mismatch, &self.begin);
            let pending = self.chunk(token, &self.begin, &self.end);

            self.pending = Some(pending);

            self.begin = self.end;
            self.current = self.end;

            return Some(result);
        }
    }
}

impl<'input, T: Token> FusedIterator for ChunkIndicesScanner<'input, T> {}

impl<'input, T: Token> ChunkIndicesScanner<'input, T> {
    ///todo
    #[inline(always)]
    pub fn new(input: &'input str) -> Self {
        Self {
            text: input,
            begin: Cursor::default(),
            end: Cursor::default(),
            current: Cursor::default(),
            pending: None,
        }
    }

    ///todo
    #[inline(always)]
    pub fn as_str(&self) -> &'input str {
        self.text
    }

    ///todo
    #[inline(always)]
    pub fn offset(&self) -> ByteIndex {
        match &self.pending {
            Some((byte, _)) => *byte,
            None => self.begin.byte,
        }
    }

    #[inline(always)]
    fn chunk(
        &self,
        token: T,
        from: &Cursor<Site>,
        to: &Cursor<Site>,
    ) -> (ByteIndex, Chunk<'input, T>) {
        let length = to.site - from.site;

        ld_assert!(length > 0, "Empty length.");
        ld_assert!(from.byte < to.byte, "Invalid range.");
        ld_assert!(to.byte <= self.text.len(), "Invalid range.");

        let byte = from.byte;
        let site = from.site;
        let string = unsafe { self.text.get_unchecked(from.byte..to.byte) };

        (
            byte,
            Chunk {
                token,
                site,
                length,
                string,
            },
        )
    }
}

///todo
pub struct TokenScanner<'input, T: Token> {
    text: &'input str,
    begin: Cursor<()>,
    end: Cursor<()>,
    current: Cursor<()>,
    pending: Option<T>,
}

unsafe impl<'input, T: Token> LexisSession for TokenScanner<'input, T> {
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        self.current.advance(self.text)
    }

    #[inline(always)]
    unsafe fn consume(&mut self) {
        self.current.consume(self.text)
    }

    #[inline(always)]
    unsafe fn read(&mut self) -> char {
        self.current.read(self.text)
    }

    #[inline(always)]
    unsafe fn submit(&mut self) {
        #[cfg(debug_assertions)]
        if self.current.byte < self.text.len() {
            let byte = self.text.as_bytes()[self.current.byte];

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

impl<'input, T: Token> Iterator for TokenScanner<'input, T> {
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pending) = take(&mut self.pending) {
            return Some(pending);
        }

        if self.begin.byte == self.text.len() {
            return None;
        }

        let token = T::scan(self);

        if self.begin.byte != self.end.byte {
            self.begin = self.end;
            self.current = self.end;

            return Some(token);
        }

        loop {
            if self.begin.advance(self.text) == 0xFF {
                return Some(T::mismatch());
            }

            self.begin.consume(self.text);

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

impl<'input, T: Token> FusedIterator for TokenScanner<'input, T> {}

impl<'input, T: Token> TokenScanner<'input, T> {
    ///todo
    #[inline(always)]
    pub fn new(input: &'input str) -> Self {
        Self {
            text: input,
            begin: Cursor::default(),
            end: Cursor::default(),
            current: Cursor::default(),
            pending: None,
        }
    }

    ///todo
    #[inline(always)]
    pub fn as_str(&self) -> &'input str {
        self.text
    }
}

///todo
pub struct TokenIndicesScanner<'input, T: Token> {
    text: &'input str,
    begin: Cursor<()>,
    end: Cursor<()>,
    current: Cursor<()>,
    pending: Option<(ByteIndex, T)>,
}

unsafe impl<'input, T: Token> LexisSession for TokenIndicesScanner<'input, T> {
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        self.current.advance(self.text)
    }

    #[inline(always)]
    unsafe fn consume(&mut self) {
        self.current.consume(self.text)
    }

    #[inline(always)]
    unsafe fn read(&mut self) -> char {
        self.current.read(self.text)
    }

    #[inline(always)]
    unsafe fn submit(&mut self) {
        #[cfg(debug_assertions)]
        if self.current.byte < self.text.len() {
            let byte = self.text.as_bytes()[self.current.byte];

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

impl<'input, T: Token> Iterator for TokenIndicesScanner<'input, T> {
    type Item = (ByteIndex, T);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pending) = take(&mut self.pending) {
            return Some(pending);
        }

        if self.begin.byte == self.text.len() {
            return None;
        }

        let token = T::scan(self);

        if self.begin.byte != self.end.byte {
            let byte = self.begin.byte;

            self.begin = self.end;
            self.current = self.end;

            return Some((byte, token));
        }

        let mismatch = self.begin.byte;

        loop {
            if self.begin.advance(self.text) == 0xFF {
                return Some((mismatch, T::mismatch()));
            }

            self.begin.consume(self.text);

            self.end = self.begin;
            self.current = self.begin;

            let token = T::scan(self);

            if self.begin.byte == self.end.byte {
                continue;
            }

            self.pending = Some((self.begin.byte, token));

            self.begin = self.end;
            self.current = self.end;

            return Some((mismatch, T::mismatch()));
        }
    }
}

impl<'input, T: Token> FusedIterator for TokenIndicesScanner<'input, T> {}

impl<'input, T: Token> TokenIndicesScanner<'input, T> {
    ///todo
    #[inline(always)]
    pub fn new(input: &'input str) -> Self {
        Self {
            text: input,
            begin: Cursor::default(),
            end: Cursor::default(),
            current: Cursor::default(),
            pending: None,
        }
    }

    ///todo
    #[inline(always)]
    pub fn as_str(&self) -> &'input str {
        self.text
    }

    ///todo
    #[inline(always)]
    pub fn offset(&self) -> ByteIndex {
        match &self.pending {
            Some((byte, _)) => *byte,
            None => self.begin.byte,
        }
    }
}
