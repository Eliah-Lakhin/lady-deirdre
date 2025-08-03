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

pub mod data;
pub mod gen;
pub mod lines;
pub mod logos;
pub mod nom;
pub mod ts;

#[cfg(test)]
mod tests {
    use lady_deirdre::{
        lexis::{Scannable, SourceCode, TokenBuffer},
        syntax::VoidSyntax,
        units::Document,
    };
    use lady_deirdre_examples::json_grammar::lexis::JsonToken;
    use logos::Logos;
    use rand::prelude::*;

    use crate::{
        data::{BenchCommand, BenchData},
        gen::{JsonBootstrapGen, JsonEditsGen, JsonGenConfig},
        lines::LineToken,
        logos::LogosJsonToken,
        ts::TSParser,
    };

    #[test]
    fn test_json_generation() {
        const SEED: u64 = 1000;
        const ITERATIONS: u64 = 100;
        const GEN_ATTEMPTS: usize = 100;
        const EDITS_PER_ITERATION: usize = 100;

        for iteration in 1..=ITERATIONS {
            println!("Iteration: {iteration}:");

            let mut rng = StdRng::seed_from_u64(SEED + iteration);

            let config = JsonGenConfig::new();

            let text = JsonBootstrapGen::gen(config, &mut rng, GEN_ATTEMPTS);

            println!(
                "    Init length: {:#} / {}..{}",
                text.len(),
                config.code_min_length,
                config.code_max_length
            );

            let mut edits_gen = JsonEditsGen::new(config, text);

            for _ in 0..EDITS_PER_ITERATION {
                edits_gen.gen_edits(&mut rng);
            }

            let edits = edits_gen.take_edits();

            println!(
                "    Length after edits: {} / {}..{}",
                edits_gen.doc().length(),
                config.code_min_length,
                config.code_max_length
            );
            println!("    Total edits: {}", edits.len());

            edits_gen.check();
        }
    }

    #[test]
    fn test_bench_data() {
        let (small, large) = BenchData::load();

        small.check();
        large.check();
    }

    #[test]
    fn test_ts_parser() {
        let (small, _) = BenchData::load();

        let mut ld_parser = Document::new_mutable("");
        let mut ts_parser = TSParser::new();

        for (index, command) in small.iter().enumerate() {
            match command {
                BenchCommand::Init { text } => {
                    ld_parser.write(.., text);
                    ts_parser.parse(text);

                    ts_parser.check_length();
                    ts_parser.compare_trees(&ld_parser);
                }

                BenchCommand::Edit {
                    site_span,
                    position_span,
                    new_end_position,
                    text,
                } => {
                    ld_parser.write(&site_span, text);
                    ts_parser.reparse(site_span, position_span, new_end_position, text);
                }

                BenchCommand::Wait => {
                    ts_parser.check_length();
                    ts_parser.compare_trees(&ld_parser);
                }
            }

            println!("Command {index} OK.");
        }
    }

    #[test]
    fn test_line_lexer() {
        let (small, _) = BenchData::load();

        let mut json_doc = Document::<VoidSyntax<JsonToken>>::new_mutable("");
        let mut line_doc = Document::<VoidSyntax<LineToken>>::new_mutable("");

        for (index, command) in small.iter().enumerate() {
            match command {
                BenchCommand::Init { text } => {
                    json_doc.write(.., text);
                    line_doc.write(.., text);
                }
                BenchCommand::Edit {
                    site_span, text, ..
                } => {
                    json_doc.write(&site_span, text);
                    line_doc.write(site_span, text);
                }
                BenchCommand::Wait => (),
            }

            assert_eq!(json_doc.substring(..), line_doc.substring(..));
            assert_eq!(line_doc.lines().lines_count(), line_doc.tokens() + 1);

            println!("Command {index} OK.");
        }
    }

    #[test]
    fn test_stateless_scanner() {
        let (small, large) = BenchData::load();

        let Some(BenchCommand::Init { text }) = small.iter().next() else {
            panic!("Missing Small File init command.");
        };

        let tokens = text.tokens::<JsonToken>().collect::<Vec<_>>();
        let buffer = TokenBuffer::<JsonToken>::parse(text);

        assert_eq!(tokens.len(), buffer.tokens());

        for (a, b) in tokens.iter().zip(buffer.chunks(..)) {
            assert_eq!(a, &b.token);
        }

        let Some(BenchCommand::Init { text }) = large.iter().next() else {
            panic!("Missing Small File init command.");
        };

        let tokens = text.tokens::<JsonToken>().collect::<Vec<_>>();
        let buffer = TokenBuffer::<JsonToken>::parse(text);

        assert_eq!(tokens.len(), buffer.tokens());

        for (a, b) in tokens.iter().zip(buffer.chunks(..)) {
            assert_eq!(a, &b.token);
        }
    }

    #[test]
    fn test_logos() {
        let (small, large) = BenchData::load();

        let Some(BenchCommand::Init { text }) = small.iter().next() else {
            panic!("Missing Small File init command.");
        };

        let tokens = LogosJsonToken::lexer(text)
            .map(|result| result.unwrap())
            .collect::<Vec<_>>();
        let buffer = TokenBuffer::<JsonToken>::parse(text);

        assert_eq!(tokens.len(), buffer.tokens());

        for (a, b) in tokens.iter().zip(buffer.chunks(..)) {
            assert_eq!(a.into_ld(), b.token);
        }

        let Some(BenchCommand::Init { text }) = large.iter().next() else {
            panic!("Missing Large File init command.");
        };

        let tokens = LogosJsonToken::lexer(text)
            .map(|result| result.unwrap())
            .collect::<Vec<_>>();
        let buffer = TokenBuffer::<JsonToken>::parse(text);

        assert_eq!(tokens.len(), buffer.tokens());

        for (a, b) in tokens.iter().zip(buffer.chunks(..)) {
            assert_eq!(a.into_ld(), b.token);
        }
    }
}
