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
    lexis::{
        utils::{get_lexis_character, NULL},
        ByteIndex,
        Site,
        Token,
        TokenBuffer,
    },
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
/// impl<'a> LexisSession for First<'a> {
///     fn advance(&mut self) {
///         if self.cursor >= self.input.len() { return; }
///         self.cursor += self.input[self.cursor..].chars().next().unwrap().len_utf8();
///     }
///
///     fn character(&self) -> char {
///         if self.cursor >= self.input.len() { return '\0'; }
///
///         let character = self.input[self.cursor..].chars().next().unwrap();
///
///         if character == '\0' { return char::REPLACEMENT_CHARACTER; }
///
///         character
///     }
///
///     fn submit(&mut self) { self.end = self.cursor; }
///
///     fn substring(&mut self) -> &str { &self.input[self.start..self.end] }
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
pub trait LexisSession {
    /// Tells the iterator to move to the next input character.
    ///
    /// This function does nothing if there are no more characters in the input sequence.
    fn advance(&mut self);

    /// Returns current character of the input sequence.
    ///
    /// This function does not [advance](LexisSession::advance) Session's internal cursor.
    ///
    /// If the current character is a Null character(`'\0'`), the function returns
    /// [replacement character](::std::char::REPLACEMENT_CHARACTER) instead.
    ///
    /// If there are no more characters in the input sequences(the Session has reach the end of
    /// input) this function returns Null character.
    fn character(&self) -> char;

    /// Tells the iterator that the sequence of characters scanned prior to the current
    /// characters(excluding the current character) build up complete token.
    ///
    /// The Algorithm can call this function multiple times. In this case the Session will ignore
    /// all previous "submissions" in favor to the last one.
    ///
    /// If the Algorithm never invokes this function, or the Algorithm never invokes
    /// [advance](LexisSession::advance) function during the scanning session, the input sequence
    /// considered to be lexically incorrect.
    ///
    /// This function does not advance Session's internal cursor.
    fn submit(&mut self);

    /// Returns a substring of the input text from the beginning of the scanning session till the
    /// latest [submitted](LexisSession::submit) character(excluding that submitted character).
    ///
    /// This function does not [advance](LexisSession::advance) Session's internal cursor.
    fn substring(&mut self) -> &str;
}

pub(super) struct SequentialLexisSession<'code, T: Token> {
    pub(super) buffer: &'code mut TokenBuffer<T>,
    pub(super) next_cursor: Cursor,
    pub(super) begin_cursor: Cursor,
    pub(super) start_cursor: Cursor,
    pub(super) end_cursor: Cursor,
}

impl<'code, T: Token> LexisSession for SequentialLexisSession<'code, T> {
    #[inline(always)]
    fn advance(&mut self) {
        self.next_cursor.advance(self.buffer);
    }

    #[inline(always)]
    fn character(&self) -> char {
        self.next_cursor.character
    }

    #[inline(always)]
    fn submit(&mut self) {
        self.end_cursor = self.next_cursor;
    }

    #[inline(always)]
    fn substring(&mut self) -> &str {
        unsafe {
            self.buffer
                .tail
                .get_unchecked(self.start_cursor.byte_index..self.end_cursor.byte_index)
        }
    }
}

impl<'code, T: Token> SequentialLexisSession<'code, T> {
    #[inline]
    pub(super) fn run(buffer: &'code mut TokenBuffer<T>, site: Site)
    where
        T: Token,
    {
        let cursor = Cursor {
            site,
            byte_index: 0,
            character: unsafe { get_lexis_character(buffer.tail.get_unchecked(0..).chars()) },
        };

        let mut session = Self {
            buffer,
            next_cursor: cursor,
            begin_cursor: cursor,
            start_cursor: cursor,
            end_cursor: cursor,
        };

        loop {
            let token = T::parse(&mut session);

            if session.start_cursor.site != session.end_cursor.site {
                session
                    .buffer
                    .push(token, &session.start_cursor, &session.end_cursor);

                if session.end_cursor.character == NULL {
                    break;
                }

                session.reset();

                continue;
            }

            if session.enter_mismatch_loop(token) {
                break;
            }
        }
    }

    // Returns true if the parsing process supposed to stop
    #[inline]
    fn enter_mismatch_loop(&mut self, mismatch: T) -> bool
    where
        T: Token,
    {
        loop {
            self.start_cursor.advance(&self.buffer);
            self.next_cursor = self.start_cursor;

            if self.start_cursor.character == NULL {
                self.buffer
                    .push(mismatch, &self.begin_cursor, &self.start_cursor);

                return true;
            }

            let token = T::parse(self);

            if self.start_cursor.site < self.end_cursor.site {
                self.buffer
                    .push(mismatch, &self.begin_cursor, &self.start_cursor);

                self.buffer
                    .push(token, &self.start_cursor, &self.end_cursor);

                if self.end_cursor.character == NULL {
                    return true;
                }

                self.reset();

                return false;
            }
        }
    }

    #[inline(always)]
    fn reset(&mut self) {
        self.begin_cursor = self.end_cursor;
        self.start_cursor = self.end_cursor;
        self.next_cursor = self.end_cursor;
    }
}

#[derive(Clone, Copy)]
pub(super) struct Cursor {
    pub(super) site: Site,
    pub(super) byte_index: ByteIndex,
    pub(super) character: char,
}

impl Cursor {
    #[inline]
    fn advance<T: Token>(&mut self, buffer: &TokenBuffer<T>) {
        if self.character == NULL {
            return;
        }

        self.site += 1;
        self.byte_index += self.character.len_utf8();

        if self.byte_index == buffer.tail.len() {
            self.character = NULL;
            return;
        }

        self.character =
            unsafe { get_lexis_character(buffer.tail.get_unchecked(self.byte_index..).chars()) };
    }
}
