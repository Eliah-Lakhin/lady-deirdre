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

#[cfg(debug_assertions)]
use crate::report::system_panic;
use crate::{
    lexis::{ByteIndex, Site, Token, TokenBuffer},
    report::{ld_assert, ld_assert_ne},
};

/// A communication channel of the lexical scanning process.
///
/// Lady Deirdre distinguishes two independent sides of the lexical scanning process:
///
/// - The scanning environment side (e.g., [Document](crate::units::Document))
///   which manages the scanning input (source code text), and the scanning
///   output (token chunks).
///
/// - The scanning algorithm implementation side (via the [Token::scan] function).
///
/// Both sides unaware of each other, and they use LexisSession
/// for intercommunication.
///
///  1. The scanning environment scans tokens one by one by calling the [Token::scan]
///     function, utilizing its own implementation of the LexisSession.
///     This allows the scanning environment to grant the scanning algorithm
///     access to the source code text content that needs to be scanned.
///
///  2. The scanning algorithm reads individual UTF-8 bytes of the source code
///     text using the [LexisSession::advance], [LexisSession::consume], and
///     [LexisSession::read] functions.
///
///  3. The scanning algorithm informs the scanning environment of
///     the token's end position by invoking the [LexisSession::submit] function.
///
///  4. Upon the [Token::scan] algorithm returning control flow to the scanning
///     environment, the scanning environment determines whether to continue
///     scanning the next token or halt the scanning process.
///
/// Since both sides of the scanning process are isolated from each other,
/// it opens up a lot of implementation configurations.
///
/// ## As the user of the LexisSession instances
///
/// You can use the [Token derive macro](lady_deirdre_derive::Token) that
/// provides a canonical generator of the lexical scanner based on the regex
/// rules.
///
/// You can also implement your own hand-written lexical scanner, create
/// a new kind of scanner generator/combinator, or adopt a 3rd party
/// scanning library to Lady Deirdre adopting its interface to the LexisSession
/// interface.
///
/// ## As the author of the LexisSession implementations
///
/// You can create a new kind of language-independent scanning environment
/// where you can determine the source code text and the output tokens storage
/// strategy.
///
/// In particular, Lady Deirdre offers the following scanning environments
/// through custom implementations of the LexisSession trait:
///
///  - The [TokenBuffer] stores the source code text in a contiguous allocation
///    of UTF-8 bytes ([String]) and stores the scanned token metadata
///    in contiguous arrays ([Vec]). This approach is efficient to iterate
///    through the tokens and the source code text bytes and provides a way to
///    resume the scanning process by appending the text and tokens to the end
///    of the buffer. However, the TokenBuffer does not offer rescanning
///    capabilities to rewrite random arbitrary fragments of the content.
///
///  - The mutable [Document](crate::units::Document) and
///    the [MutableUnit](crate::units::MutableUnit) both store the original text
///    and the scanned tokens metadata in the rope-like structure built on top
///    of the self-balancing B+Tree index. This approach provides an efficient
///    way to rewrite and rescan random fragments of the content but is less
///    efficient for the initial scanning of the entire source code text.
///
/// ## Scanning algorithm considerations
///
/// The [Token::scan] function internal scanning algorithm should satisfy
/// the following requirements:
///
///  1. The scanning algorithm should recognize just one token of maximum
///     length in the input stream: from the beginning of the stream to
///     a UTF-8 code point boundary where the token ends.
///
///  2. The algorithm should be **deterministic**: for the same input stream,
///     it must produce the same result.
///
///  3. The rationale behind the scanning algorithm
///     is a **finite state automaton**: it only needs to know
///     the currently read input byte and the current state (from the finite
///     set of inner algorithm states) to transit into the next state or to
///     transit into a final state returning the result.
///
///  4. When the algorithm detects a full token match, it should call
///     the [LexisSession::submit] function. This function should only be
///     called between the UTF-8 code point boundaries or at the end of the
///     input stream.
///
///  5. The algorithm may call the submit function many times if it detects
///     that the token string continues in the input stream. However, if it calls
///     the submit function at least once, the [Token::scan] function must
///     return a non-[mismatch](Token::mismatch) token that corresponds to the
///     latest call of the submit function.
///
///  6. If the algorithm never calls the submit function, the scan function
///     returns a [mismatch](Token::mismatch) token, which indicates that the
///     entire input stream is not recognizable by the lexical scanner.
///
/// ## Input stream
///
///  1. The input of the algorithm (provided by the LexisSession state) is
///     a **possibly empty** stream of bytes that represents valid UTF-8
///     encoding of the source code text fragment that needs to be scanned.
///
///  2. The scanning algorithm reads as many bytes from the input stream
///     as needed by calling the [LexisSession::advance] function, which returns
///     the next byte in the stream and advances internal stream cursor.
///
///  3. If the input stream reaches the end, the advance function returns
///     `0xFF` byte.
///
///  4. Otherwise, the function returns the next byte in the UTF-8 encoding.
///
///  5. If the LexisSession user read a byte at the beginning of the UTF-8
///     code point, the user may read the rest of the code point by calling
///     the [LexisSession::read] function, which returns a [char] of
///     the consumed code point, or the [LexisSession::consume] function, which
///     just consumes the entire code point and does not return anything.
///     Both functions advance the internal cursor of the LexisSession
///     to the beginning of the next code point boundary.
///
/// ## Safety
///
/// The implementor of the LexisSession guarantees that the LexisSession
/// iterates through the bytes of valid UTF-8 encoding of the (possibly empty)
/// text, and the function [LexisSession::advance] returns the `0xFF` byte once
/// the iterator reaches the end of the input.
pub unsafe trait LexisSession {
    /// Returns the next byte in the input stream and advances the byte-cursor.
    ///
    /// If the input stream reaches the end, this function returns the `0xFF`
    /// value.
    fn advance(&mut self) -> u8;

    /// Reads the in-progress code point to the end.
    ///
    /// **Safety**
    ///
    /// The caller side is confident that the previous byte denoted
    /// the beginning of a code point.
    ///
    /// The ASCII code points are valid code points to consume as well. If the
    /// previous byte denotes the beginning of a single-byte code point (ASCII
    /// code point), this function does nothing.
    unsafe fn consume(&mut self);

    /// Reads the in-progress code point to the end and returns a [char]
    /// decoding of this code point.
    ///
    /// **Safety**
    ///
    /// The caller side is confident that the previous byte denoted
    /// the beginning of a code point.
    ///
    /// The ASCII code points are valid code points to read as well. If the
    /// previous byte denotes the beginning of a single-byte code point (ASCII
    /// code point), this function returns an ASCII character.
    unsafe fn read(&mut self) -> char;

    /// Pins that the current byte cursor position is a possible end of the
    /// scanning token.
    ///
    /// The caller may call this function many times. In this case, only
    /// the latest submitted position will be considered as a token end.
    ///
    /// **Safety**
    ///
    /// 1. The next byte in the input stream would be the beginning of the next
    ///    code point or the end of the input.
    ///
    /// 2. The caller side has successfully read at least one code point from
    ///    the input stream.
    unsafe fn submit(&mut self);
}

pub(super) struct BufferLexisSession<'code, T: Token> {
    pub(super) buffer: &'code mut TokenBuffer<T>,
    pub(super) begin: Cursor<Site>,
    pub(super) end: Cursor<Site>,
    pub(super) current: Cursor<Site>,
}

unsafe impl<'code, T: Token> LexisSession for BufferLexisSession<'code, T> {
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        self.current.advance(&self.buffer.text)
    }

    #[inline(always)]
    unsafe fn consume(&mut self) {
        self.current.consume(&self.buffer.text)
    }

    #[inline(always)]
    unsafe fn read(&mut self) -> char {
        self.current.read(&self.buffer.text)
    }

    #[inline(always)]
    unsafe fn submit(&mut self) {
        #[cfg(debug_assertions)]
        if self.current.byte < self.buffer.text.len() {
            let byte = self.buffer.text.as_bytes()[self.current.byte];

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

impl<'code, T: Token> BufferLexisSession<'code, T> {
    #[inline]
    pub(super) fn run(buffer: &'code mut TokenBuffer<T>, byte: ByteIndex, site: Site)
    where
        T: Token,
    {
        let cursor = Cursor { byte, site };

        let mut session = Self {
            buffer,
            begin: cursor,
            end: cursor,
            current: cursor,
        };

        loop {
            let token = T::scan(&mut session);

            if session.begin.byte != session.end.byte {
                session.buffer.push(token, &session.begin, &session.end);

                if session.end.byte == session.buffer.text.len() {
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
    }

    // Returns true if the parsing process supposed to stop
    #[inline]
    fn enter_mismatch_loop(&mut self) -> bool
    where
        T: Token,
    {
        let mismatch = self.begin;

        loop {
            if self.begin.advance(&self.buffer.text) == 0xFF {
                self.buffer.push(T::mismatch(), &mismatch, &self.begin);
                return true;
            }

            self.begin.consume(&self.buffer.text);

            self.end = self.begin;
            self.current = self.begin;

            let token = T::scan(self);

            if self.begin.byte == self.end.byte {
                continue;
            }

            self.buffer.push(T::mismatch(), &mismatch, &self.begin);
            self.buffer.push(token, &self.begin, &self.end);

            if self.end.byte == self.buffer.text.len() {
                return true;
            }

            self.begin = self.end;
            self.current = self.end;

            return false;
        }
    }
}

#[derive(Default, Clone, Copy)]
pub(super) struct Cursor<S> {
    pub(super) byte: ByteIndex,
    pub(super) site: S,
}

impl<S: CursorSite> Cursor<S> {
    #[inline(always)]
    pub(super) fn advance(&mut self, text: &str) -> u8 {
        if self.byte == text.len() {
            return 0xFF;
        }

        let point = *unsafe { text.as_bytes().get_unchecked(self.byte) };

        self.site.inc(point);

        self.byte += 1;

        point
    }

    #[inline(always)]
    pub(super) fn consume(&mut self, text: &str) {
        ld_assert!(
            self.byte > 0,
            "Incorrect use of the LexisSession::consume function.\nCurrent \
            cursor is in the beginning of the input stream.",
        );

        let point = unsafe { *text.as_bytes().get_unchecked(self.byte - 1) };

        ld_assert_ne!(
            point & 0xC0,
            0x80,
            "Incorrect use of the LexisSession::consume function.\nA byte \
            before the current cursor is not a UTF-8 code point start byte.",
        );

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
    pub(super) fn read(&mut self, text: &str) -> char {
        ld_assert!(
            self.byte > 0,
            "Incorrect use of the LexisSession::read function.\nCurrent cursor \
            is in the beginning of the input stream."
        );

        let byte = self.byte - 1;

        #[cfg(debug_assertions)]
        {
            let point = text.as_bytes()[byte];

            if point & 0xC0 == 0x80 {
                system_panic!(
                    "Incorrect use of the LexisSession::read function.\nA byte \
                    before the current cursor is not a UTF-8 code point start \
                    byte."
                )
            }
        }

        let rest = unsafe { text.get_unchecked(byte..) };
        let ch = unsafe { rest.chars().next().unwrap_unchecked() };
        let len = ch.len_utf8();

        self.byte += len - 1;

        ch
    }
}

pub(super) trait CursorSite {
    fn inc(&mut self, point: u8);
}

impl CursorSite for () {
    #[inline(always)]
    fn inc(&mut self, _point: u8) {}
}

impl CursorSite for Site {
    #[inline(always)]
    fn inc(&mut self, point: u8) {
        if point & 0xC0 != 0x80 {
            *self += 1;
        }
    }
}
