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

use crate::lexis::{Length, Site, SiteSpan, SourceCode, ToSite, ToSpan, Token};

/// An object of the [token](Token) metadata.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Chunk<'source, T: Token> {
    /// A copy of the token.
    pub token: T,

    /// A start [site](Site) of the token.
    pub site: Site,

    /// The [length](Length) of the source code text fragment covered
    /// by this token.
    pub length: Length,

    /// A reference to the string of the text fragment covered by this token.
    pub string: &'source str,
}

unsafe impl<'source, T: Token> ToSpan for Chunk<'source, T> {
    #[inline(always)]
    fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan> {
        let start = unsafe { self.start().to_site(code).unwrap_unchecked() };
        let end = unsafe { self.end().to_site(code).unwrap_unchecked() };

        Some(start..end)
    }

    #[inline(always)]
    fn is_valid_span(&self, _code: &impl SourceCode) -> bool {
        true
    }
}

impl<'source, T: Token> Chunk<'source, T> {
    /// Returns the start [site](Site) of the token.
    #[inline(always)]
    pub fn start(&self) -> Site {
        self.site
    }

    /// Returns the end [site](Site) of the token.
    #[inline(always)]
    pub fn end(&self) -> Site {
        self.site + self.length
    }

    /// Returns the [site span](SiteSpan) of the token.
    ///
    /// The returning value equals the `chunk.start()..chunk.end()`.
    #[inline(always)]
    pub fn span(&self) -> SiteSpan {
        self.start()..self.end()
    }
}
