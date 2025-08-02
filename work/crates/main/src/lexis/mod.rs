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

mod buffer;
mod chunks;
mod code;
mod cursor;
mod lines;
mod position;
mod rule;
mod scanners;
mod session;
mod site;
mod span;
mod token;
mod ucd;

pub(crate) const CHUNK_SIZE: Length = 5;

pub(crate) use crate::lexis::site::SiteRefInner;
pub use crate::lexis::{
    buffer::TokenBuffer,
    chunks::Chunk,
    code::{ChunkIter, SourceCode},
    cursor::TokenCursor,
    lines::LineIndex,
    position::{Column, Line, Position},
    rule::{TokenRule, TokenSet, EMPTY_TOKEN_SET, EOI, FULL_TOKEN_SET, MISMATCH},
    scanners::TokenScanner,
    session::LexisSession,
    site::{ByteIndex, Length, Site, SiteRef, ToSite, NIL_SITE_REF},
    span::{PositionSpan, SiteRefSpan, SiteSpan, ToSpan},
    token::{Token, TokenCount, TokenRef, NIL_TOKEN_REF},
    ucd::{Char, CharProperties},
};
