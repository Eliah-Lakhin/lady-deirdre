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

use lady_deirdre_examples::json_grammar::lexis::JsonToken;
use logos::Logos;

#[derive(Logos, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogosJsonToken {
    #[token("true")]
    True,

    #[token("false")]
    False,

    #[token("null")]
    Null,

    #[token("{")]
    BraceOpen,

    #[token("}")]
    BraceClose,

    #[token("[")]
    BracketOpen,

    #[token("]")]
    BracketClose,

    #[token(",")]
    Comma,

    #[token(":")]
    Colon,

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#)]
    String,

    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?")]
    Number,

    #[regex(r"[ \t\r\n\f]+")]
    Whitespace,
}

impl LogosJsonToken {
    pub fn into_ld(self) -> JsonToken {
        match self {
            LogosJsonToken::True => JsonToken::True,
            LogosJsonToken::False => JsonToken::False,
            LogosJsonToken::Null => JsonToken::Null,
            LogosJsonToken::BraceOpen => JsonToken::BraceOpen,
            LogosJsonToken::BraceClose => JsonToken::BraceClose,
            LogosJsonToken::BracketOpen => JsonToken::BracketOpen,
            LogosJsonToken::BracketClose => JsonToken::BracketClose,
            LogosJsonToken::Comma => JsonToken::Comma,
            LogosJsonToken::Colon => JsonToken::Colon,
            LogosJsonToken::String => JsonToken::String,
            LogosJsonToken::Number => JsonToken::Number,
            LogosJsonToken::Whitespace => JsonToken::Whitespace,
        }
    }
}
