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

//TODO check warnings regularly
#![allow(warnings)]

use std::{
    fmt::{Display, Formatter},
    thread::sleep,
    time::Duration,
};

use lady_deirdre::{
    format::{AnnotationPriority, SnippetConfig, SnippetFormatter},
    lexis::{SiteSpan, SourceCode},
    syntax::SyntaxTree,
    units::Document,
};
use lady_deirdre_examples::json_grammar::syntax::JsonNode;
use lady_deirdre_tests::data::{BenchCommand, BenchData};

fn main() {
    let (small, _) = BenchData::load();

    let mut doc = Document::<JsonNode>::new_mutable("");

    for (op, command) in small.iter().enumerate() {
        match command {
            BenchCommand::Init { text } => {
                doc.write(.., text);
            }

            BenchCommand::Edit {
                site_span, text, ..
            } => {
                Visualizer::print(&small, &doc, op, &site_span);
                sleep(Duration::from_millis(100));
                doc.write(site_span, text);
            }

            BenchCommand::Wait => {
                sleep(Duration::from_millis(1000));
            }
        }
    }
}

struct Visualizer<'a> {
    data: &'a BenchData,
    doc: &'a Document<JsonNode>,
    op: usize,
    span: &'a SiteSpan,
}

impl<'a> Visualizer<'a> {
    fn print(data: &'a BenchData, doc: &'a Document<JsonNode>, op: usize, span: &'a SiteSpan) {
        let visualizer = Self {
            data,
            doc,
            op,
            span,
        };

        print!("{esc}[2J{esc}[1;1H{:#}\n", visualizer, esc = 27 as char);
    }

    fn size(&self) -> String {
        BenchData::format_size(self.doc.length())
    }

    fn lines(&self) -> usize {
        self.doc.lines().lines_count()
    }
}

impl<'a> Display for Visualizer<'a> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let mut snippet = formatter.snippet(self.doc);

        snippet.set_caption(self.data.title());

        let has_errors = self.doc.errors().next().is_some();

        let mut summary = String::new();

        summary.push_str(&format!("Operation: {}/{}\n", self.op, self.data.ops()));
        summary.push_str(&format!("Size: {}/{}\n", self.size(), self.data.size()));
        summary.push_str(&format!(
            "Total Lines: {}/{}\n",
            self.lines(),
            self.data.lines()
        ));
        summary.push('\n');

        match has_errors {
            false => summary.push_str("No syntax errors."),

            true => {
                summary.push_str("Syntax errors:");

                for error in self.doc.errors() {
                    summary.push('\n');

                    let error = format!("  - {}", error.display(self.doc));

                    match error.len() <= 65 {
                        true => summary.push_str(&error),
                        false => {
                            summary.push_str(&error[..65]);
                            summary.push_str("...");
                        }
                    }
                }
            }
        };

        snippet.set_summary(summary);
        snippet.annotate(self.span, AnnotationPriority::Default, "");

        snippet.finish()
    }
}
