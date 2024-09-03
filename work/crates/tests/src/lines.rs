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

use lady_deirdre::lexis::{Length, LexisSession, Token, TokenRule};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LineToken {
    EOI = 0,
    Mismatch = 1,
    Line = 2,
}

impl Token for LineToken {
    const LOOKBACK: Length = 1;

    #[inline(always)]
    fn scan(session: &mut impl LexisSession) -> Self {
        let mut byte = session.advance();

        if byte == b'\n' {
            unsafe { session.submit() };
            return Self::Line;
        }

        if byte == 0xFF {
            return Self::Mismatch;
        }

        loop {
            byte = session.advance();

            if byte == b'\n' || byte == 0xFF {
                unsafe { session.submit() };
                break;
            }
        }

        Self::Line
    }

    #[inline(always)]
    fn eoi() -> Self {
        Self::EOI
    }

    #[inline(always)]
    fn mismatch() -> Self {
        Self::Mismatch
    }

    #[inline(always)]
    fn rule(self) -> TokenRule {
        self as u8
    }

    #[inline(always)]
    fn rule_name(_rule: TokenRule) -> Option<&'static str> {
        None
    }

    #[inline(always)]
    fn rule_description(_rule: TokenRule, _verbose: bool) -> Option<&'static str> {
        None
    }
}
