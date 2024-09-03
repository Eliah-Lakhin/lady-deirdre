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

use std::fmt::{Display, Formatter};

use lady_deirdre::{
    format::{AnnotationPriority, Highlighter, SnippetFormatter, Style},
    lexis::PositionSpan,
    units::Document,
};

use crate::json_grammar::{lexis::JsonToken, syntax::JsonNode};

pub struct JsonSnippet<'a> {
    pub doc: &'a Document<JsonNode>,
    pub annotation: Vec<(PositionSpan, AnnotationPriority, &'static str)>,
}

impl<'a> Display for JsonSnippet<'a> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let mut snippet = formatter.snippet(self.doc);

        snippet
            .set_caption("Header text")
            .set_summary("Footer text.")
            .set_highlighter(JsonHighlighter);

        for (span, priority, message) in &self.annotation {
            snippet.annotate(span, *priority, *message);
        }

        snippet.finish()
    }
}

pub struct JsonHighlighter;

impl Highlighter<JsonToken> for JsonHighlighter {
    fn token_style(&mut self, dim: bool, token: JsonToken) -> Option<Style> {
        match token {
            JsonToken::True | JsonToken::False | JsonToken::Null => Some(match dim {
                false => Style::new().blue(),
                true => Style::new().bright_blue(),
            }),

            JsonToken::String => Some(match dim {
                false => Style::new().green(),
                true => Style::new().bright_green(),
            }),

            JsonToken::BraceOpen | JsonToken::BraceClose => Some(Style::new().bold()),

            _ => None,
        }
    }
}
