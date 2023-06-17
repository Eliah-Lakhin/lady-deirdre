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

use crate::{lexis::Token, std::*};

/// A common generic lexis.
///
/// You can use this Token type when particular source code grammar is unknown or does not
/// matter(e.g. if the end user opens a custom .txt file in the code editor window), but the text
/// needs to be split into tokens in some reasonable way.
///
/// Also, you can use a companion [SimpleNode](crate::syntax::SimpleNode) syntax implementation
/// that parses parens nesting on top of this lexis.
#[derive(Token, Clone, Copy, Debug, PartialEq, Eq)]
#[define(ALPHABET = ['a'..'z', 'A'..'Z'])]
#[define(NUM = ['0'..'9'])]
#[define(ALPHANUM = ALPHABET | NUM)]
#[define(SYMBOL = [
    '!', '@', '#', '$', '%', '^', '&', '*', '-', '+', '=', '/', '|', ':', ';', '.',
    ',', '<', '>', '?', '~', '`'
])]
#[repr(u8)]
pub enum SimpleToken {
    EOI = 0,

    /// Any other token that does not fit this lexical grammar.
    Mismatch = 1,

    /// A numerical literal. Either integer or a floating point(e.g. `12345` or `1234.56`).
    #[rule(NUM+ & ('.' & NUM+)?)]
    Number,

    /// All keyboard terminal character(e.g. `@` or `%`) except paren terminals.
    #[rule(SYMBOL | '\\')]
    Symbol,

    /// An open parenthesis(`(`) terminal.
    #[rule('(')]
    ParenOpen,

    /// A close parenthesis(`)`) terminal.
    #[rule(')')]
    ParenClose,

    /// An open bracket(`[`) terminal.
    #[rule('[')]
    BracketOpen,

    /// A close bracket(`]`) terminal.
    #[rule(']')]
    BracketClose,

    /// An open brace(`{`) terminal.
    #[rule('{')]
    BraceOpen,

    /// A close brace(`}`) terminal.
    #[rule('}')]
    BraceClose,

    /// An English alphanumeric word that does not start with digit(e.g. `hello_World123`).
    #[rule(ALPHABET & (ALPHANUM | '_')*)]
    Identifier,

    /// A string literal surrounded by `"` characters that allows any characters inside including
    /// the characters escaped by `\` prefix(e.g. `"hello \" \n world"`).
    #[rule('"' & ('\\' & . | ^['\\', '\"'])* & '"')]
    String,

    /// A single character literal surrounded by `'` characters that allows any character inside
    /// including the characters escaped by `\` prefix(e.g. `'A'`, or `'\A'`, or `'\''`)
    #[rule('\'' & ('\\' & . | ^['\\', '\''])  & '\'')]
    Char,

    /// A sequence of whitespace characters as defined in
    /// [`char::is_ascii_whitespace()`](char::is_ascii_whitespace).
    #[rule([' ', '\t', '\n', '\x0c', '\r']+)]
    Whitespace,
}

impl Display for SimpleToken {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(self, formatter)
    }
}
