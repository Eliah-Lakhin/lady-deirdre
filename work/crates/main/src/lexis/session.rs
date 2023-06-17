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
    lexis::{ByteIndex, Site, Token, TokenBuffer},
    report::{debug_assert, debug_assert_ne, system_panic},
    std::*,
};

/// An interface to the source code lexical parsing/re-parsing session.
///
/// This is a low-level API.
///
/// Lexical parsing architecture decoupled into two independent components:
///   - The Source Code Manager that organizes a lexical data storage, and that provides access
///     operations to the lexical structure. This component implements a
///     [SourceCode](crate::lexis::SourceCode) trait.
///   - The Lexical Scanner of particular programming language. This component is unaware about the
///     lexical structure memory management process, and about the source of scanning.
///
/// Both components of this architecture are unaware about each other, and they use a
/// [LexisSession] trait as an input/output "thin" interaction interface.
///
/// The Source Code Manager passes a mutable reference to LexisSession object to the
/// [`Token::new`](crate::lexis::Token::parse) function to initiate lexical scanning procedure in
/// specified context. And, in turn, the `Token::new` function uses this object to read
/// Unicode input character, and to drive the scanning process.
///
/// You can implement this trait as well as the [SourceCode](crate::lexis::SourceCode) trait to
/// create a custom lexis manager of the compilation unit that would be able to work with
/// existing lexical grammar definitions seamlessly.
///
/// As long as the the [Token](crate::lexis::Token) trait implementation follows
/// [`Algorithm Specification`](crate::syntax::Node::parse), the
/// intercommunication between the Lexical Scanner and the Source Code Manager works correctly.
///
/// ```rust
/// use lady_deirdre::lexis::{LexisSession, SimpleToken, Token, ByteIndex};
///
/// // A lexis session object that simply obtains the first token from string.
/// struct First<'a> {
///     // An input string.
///     input: &'a str,
///     // An internal cursor into the `input`.
///     cursor: ByteIndex,
///     // An input parse start cursor.
///     start: ByteIndex,
///     // A submitted parse end cursor.
///     end: ByteIndex
/// };
///
/// unsafe impl<'a> LexisSession for First<'a> {
///     fn advance(&mut self) -> u8 {
///         if self.cursor >= self.input.len() { return 0xFF; }
///
///         let byte = self.input.as_bytes()[self.cursor];
///
///         self.cursor += 1;
///
///         byte
///     }
///
///     unsafe fn consume(&mut self) {
///         // Safety: `read` behavior is similar to the consume,
///         //         except that it does not return decoded character.
///         let _ = unsafe { self.read() };
///     }
///
///     unsafe fn read(&mut self) -> char {
///         let ch = self.input[self.cursor..].chars().next().unwrap();
///
///         self.cursor += ch.len_utf8() - 1;
///
///         ch
///     }
///
///     unsafe fn submit(&mut self) { self.end = self.cursor; }
/// }
///
/// impl<'a> First<'a> {
///     fn run<T: Token>(input: &'a str) -> (T, &'a str) {
///         let mut session = First {
///             input,
///             cursor: 0,
///             start: 0,
///             end: 0,
///         };
///
///         let token = T::parse(&mut session);
///
///         // Token scanner didn't submit anything.
///         // Then the `token` value is "Mismatch" token type.
///         // Entering mismatch recovery loop.
///         if session.end == 0 {
///             while session.start < input.len() {
///                 session.start += input[session.start..].chars().next().unwrap().len_utf8();
///                 session.cursor = session.start;
///
///                 let _ = T::parse(&mut session);
///
///                 if session.end > session.start { break; }
///             }
///
///             return (token, &input[0..session.start]);
///         }
///
///         (token, &input[0..session.end])
///     }
/// }
///
/// assert_eq!(First::run::<SimpleToken>(""), (SimpleToken::Mismatch, ""));
/// assert_eq!(First::run::<SimpleToken>("лексема bar baz"), (SimpleToken::Mismatch, "лексема"));
/// assert_eq!(First::run::<SimpleToken>("foo bar baz"), (SimpleToken::Identifier, "foo"));
/// assert_eq!(First::run::<SimpleToken>("123 bar baz"), (SimpleToken::Number, "123"));
/// ```

// Safety: LexisSession walks through valid and complete utf-8 sequence of bytes.
pub unsafe trait LexisSession {
    fn advance(&mut self) -> u8;

    // Safety:
    //   1. There is a start byte of utf-8 code-point just behind the current cursor.
    //   2. End of input is not reached yet.
    unsafe fn consume(&mut self);

    // Safety:
    //   1. There is a start byte of utf-8 code-point just behind the current cursor.
    //   2. End of input is not reached yet.
    unsafe fn read(&mut self) -> char;

    // Safety: There is a utf-8 code point start byte in front of cursor, or
    //         the cursor reached the end of input.
    unsafe fn submit(&mut self);
}

pub(super) struct SequentialLexisSession<'code, T: Token> {
    pub(super) buffer: &'code mut TokenBuffer<T>,
    pub(super) begin: Cursor,
    pub(super) end: Cursor,
    pub(super) current: Cursor,
}

unsafe impl<'code, T: Token> LexisSession for SequentialLexisSession<'code, T> {
    #[inline(always)]
    fn advance(&mut self) -> u8 {
        self.current.advance(self.buffer)
    }

    #[inline(always)]
    unsafe fn consume(&mut self) {
        self.current.consume(self.buffer)
    }

    #[inline(always)]
    unsafe fn read(&mut self) -> char {
        self.current.read(self.buffer)
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

impl<'code, T: Token> SequentialLexisSession<'code, T> {
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
            let token = T::parse(&mut session);

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
            if self.begin.advance(self.buffer) == 0xFF {
                self.buffer.push(T::mismatch(), &mismatch, &self.begin);
                return true;
            }

            self.begin.consume(self.buffer);

            self.end = self.begin;
            self.current = self.begin;

            let token = T::parse(self);

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

#[derive(Clone, Copy)]
pub(super) struct Cursor {
    pub(super) byte: ByteIndex,
    pub(super) site: Site,
}

impl Cursor {
    #[inline(always)]
    fn advance<T: Token>(&mut self, buffer: &TokenBuffer<T>) -> u8 {
        if self.byte == buffer.text.len() {
            return 0xFF;
        }

        let point = *unsafe { buffer.text.as_bytes().get_unchecked(self.byte) };

        if point & 0xC0 != 0x80 {
            self.site += 1;
        }

        self.byte += 1;

        point
    }

    #[inline(always)]
    fn consume<T: Token>(&mut self, buffer: &TokenBuffer<T>) {
        debug_assert!(
            self.byte > 0,
            "Incorrect use of the LexisSession::consume function.\nCurrent \
            cursor is in the beginning of the input stream.",
        );

        let point = buffer.text.as_bytes()[self.byte - 1];

        debug_assert_ne!(
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
    fn read<T: Token>(&mut self, buffer: &TokenBuffer<T>) -> char {
        debug_assert!(
            self.byte > 0,
            "Incorrect use of the LexisSession::read function.\nCurrent cursor \
            is in the beginning of the input stream."
        );

        let byte = self.byte - 1;

        #[cfg(debug_assertions)]
        {
            let point = buffer.text.as_bytes()[byte];

            if point & 0xC0 == 0x80 {
                system_panic!(
                    "Incorrect use of the LexisSession::read function.\nA byte \
                    before the current cursor is not a UTF-8 code point start \
                    byte."
                )
            }
        }

        let rest = unsafe { buffer.text.get_unchecked(byte..) };
        let ch = unsafe { rest.chars().next().unwrap_unchecked() };
        let len = ch.len_utf8();

        self.byte += len - 1;

        ch
    }
}
