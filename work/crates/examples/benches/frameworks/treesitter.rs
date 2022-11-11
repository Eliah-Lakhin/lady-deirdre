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

use std::time::{Duration, Instant};

use criterion::black_box;
use lady_deirdre::lexis::SiteSpan;
use tree_sitter::Point;

use crate::FrameworkCase;

fn find_position(text: &str, mut byte: usize) -> Point {
    let mut row = 0;
    let mut column = 0;

    for char in text.chars() {
        if byte == 0 {
            break;
        }

        match char {
            '\n' => {
                column = 0;
                row += 1;
            }

            _ => {
                column += 1;
            }
        }

        byte -= 1;
    }

    Point { row, column }
}

pub struct TreeSitterCase(pub &'static str);

impl FrameworkCase for TreeSitterCase {
    fn name(&self) -> &'static str {
        self.0
    }

    #[inline(never)]
    fn bench_load(&self, text: &str) -> Duration {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(tree_sitter_json::language()).unwrap();

        let start = Instant::now();
        let result = parser.parse(text, None).unwrap();
        let time = start.elapsed();

        black_box(result);
        black_box(parser);

        time
    }

    #[inline(never)]
    fn bench_single_edit<'a>(&self, text: &'a str, span: SiteSpan, edit: &'a str) -> Duration {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(tree_sitter_json::language()).unwrap();

        let mut result = parser.parse(text, None).unwrap();

        let start_position = find_position(text, span.start);
        let old_end_position = find_position(text, span.end);
        let new_end_position = find_position(
            &format!("{}{}{}", &text[0..span.start], edit, &text[span.end..]),
            span.start + edit.len(),
        );

        let start = Instant::now();
        result.edit(&tree_sitter::InputEdit {
            start_byte: span.start,
            old_end_byte: span.end,
            new_end_byte: span.start + edit.len(),
            start_position,
            old_end_position,
            new_end_position,
        });
        result = parser.parse(edit, Some(&result)).unwrap();
        let time = start.elapsed();

        black_box(result);
        black_box(parser);

        time
    }

    #[inline(never)]
    fn bench_sequential_edits<'a>(
        &self,
        text: &'a str,
        edits: Vec<(SiteSpan, &'a str)>,
    ) -> Duration {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(tree_sitter_json::language()).unwrap();

        let mut text = text.to_string();
        let mut result = parser.parse(text.as_str(), None).unwrap();

        let mut total = Duration::ZERO;

        for (span, edit) in edits {
            let new_text = format!("{}{}{}", &text[0..span.start], edit, &text[span.end..]);

            let start_position = find_position(text.as_str(), span.start);
            let old_end_position = find_position(text.as_str(), span.end);
            let new_end_position = find_position(new_text.as_str(), span.start + edit.len());

            let start = Instant::now();
            result.edit(&tree_sitter::InputEdit {
                start_byte: span.start,
                old_end_byte: span.end,
                new_end_byte: span.start + edit.len(),
                start_position,
                old_end_position,
                new_end_position,
            });
            result = parser.parse(edit, Some(&result)).unwrap();
            let time = start.elapsed();

            total += time;

            text = new_text;
        }

        black_box(result);
        black_box(parser);

        total
    }
}
