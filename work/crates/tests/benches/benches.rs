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

use std::time::{Duration, Instant};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use lady_deirdre::{lexis::Scannable, syntax::VoidSyntax, units::Document};
use lady_deirdre_examples::json_grammar::{lexis::JsonToken, syntax::JsonNode};
use lady_deirdre_tests::{
    data::{BenchCommand, BenchData},
    lines::LineToken,
    logos::LogosJsonToken,
    nom::nom_parse,
    ts::TSParser,
};
use logos::Logos;
use ropey::Rope;

const PARSE: bool = true;
const REPARSE: bool = true;
const STORAGE: bool = true;
const SCAN: bool = true;

const SMALL: bool = true;
const LARGE: bool = true;

const LD: bool = true;
const TS: bool = true;
const NOM: bool = true;
const ROPEY: bool = true;
const LOGOS: bool = true;

pub fn bench_parsing(criterion: &mut Criterion) {
    if !PARSE {
        return;
    }

    let (small_file, large_file) = BenchData::load();

    let Some(BenchCommand::Init { text: small_text }) = small_file.iter().next() else {
        panic!("Missing Small File init command.");
    };

    let Some(BenchCommand::Init { text: large_text }) = large_file.iter().next() else {
        panic!("Missing Large File init command.");
    };

    let mut group = criterion.benchmark_group("Entire Text Parsing");

    if LD && SMALL {
        group.bench_function(
            BenchmarkId::new("Lady Deirdre (mutable)", "Small File"),
            |bencher| {
                bencher.iter_custom(|iters| {
                    let mut time = Duration::ZERO;

                    for _ in 0..iters {
                        let start = Instant::now();
                        let doc = Document::<JsonNode>::new_mutable(small_text);
                        time += start.elapsed();
                        black_box(doc);
                    }

                    time
                });
            },
        );
    }

    if LD && LARGE {
        group.bench_function(
            BenchmarkId::new("Lady Deirdre (mutable)", "Large File"),
            |bencher| {
                bencher.iter_custom(|iters| {
                    let mut time = Duration::ZERO;

                    for _ in 0..iters {
                        let start = Instant::now();
                        let doc = Document::<JsonNode>::new_mutable(large_text);
                        time += start.elapsed();
                        black_box(doc);
                    }

                    time
                });
            },
        );
    }

    if LD && SMALL {
        group.bench_function(
            BenchmarkId::new("Lady Deirdre (immutable)", "Small File"),
            |bencher| {
                bencher.iter_custom(|iters| {
                    let mut time = Duration::ZERO;

                    for _ in 0..iters {
                        let start = Instant::now();
                        let doc = Document::<JsonNode>::new_immutable(small_text);
                        time += start.elapsed();
                        black_box(doc);
                    }

                    time
                });
            },
        );
    }

    if LD && LARGE {
        group.bench_function(
            BenchmarkId::new("Lady Deirdre (immutable)", "Large File"),
            |bencher| {
                bencher.iter_custom(|iters| {
                    let mut time = Duration::ZERO;

                    for _ in 0..iters {
                        let start = Instant::now();
                        let doc = Document::<JsonNode>::new_immutable(large_text);
                        time += start.elapsed();
                        black_box(doc);
                    }

                    time
                });
            },
        );
    }

    if NOM && SMALL {
        group.bench_function(BenchmarkId::new("Nom", "Small File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let start = Instant::now();
                    nom_parse(small_text);
                    time += start.elapsed();
                }

                time
            });
        });
    }

    if NOM && LARGE {
        group.bench_function(BenchmarkId::new("Nom", "Large File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let start = Instant::now();
                    nom_parse(large_text);
                    time += start.elapsed();
                }

                time
            });
        });
    }

    if TS && SMALL {
        group.bench_function(BenchmarkId::new("Tree-Sitter", "Small File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let mut parser = tree_sitter::Parser::new();
                    parser
                        .set_language(&tree_sitter_json::LANGUAGE.into())
                        .unwrap();

                    let start = Instant::now();
                    let result = parser.parse(small_text, None).unwrap();
                    time += start.elapsed();

                    black_box(result);
                    black_box(parser);
                }

                time
            });
        });
    }

    if TS && LARGE {
        group.bench_function(BenchmarkId::new("Tree-Sitter", "Large File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let mut parser = tree_sitter::Parser::new();
                    parser
                        .set_language(&tree_sitter_json::LANGUAGE.into())
                        .unwrap();

                    let start = Instant::now();
                    let result = parser.parse(large_text, None).unwrap();
                    time += start.elapsed();

                    black_box(result);
                    black_box(parser);
                }

                time
            });
        });
    }

    group.finish();
}

pub fn bench_reparsing(criterion: &mut Criterion) {
    if !REPARSE {
        return;
    }

    let (small_file, large_file) = BenchData::load();

    let mut group = criterion.benchmark_group("Keystrokes Reparsing");

    if LD && SMALL {
        group.sample_size(100);
        group.bench_function(BenchmarkId::new("Lady Deirdre", "Small File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let mut doc = Document::<JsonNode>::new_mutable("");

                    for command in &small_file {
                        match command {
                            BenchCommand::Init { text } => {
                                doc.write(.., text);
                            }

                            BenchCommand::Edit {
                                site_span, text, ..
                            } => {
                                let start = Instant::now();
                                doc.write(site_span, text);
                                time += start.elapsed();
                            }

                            BenchCommand::Wait => (),
                        }
                    }

                    black_box(doc);
                }

                time
            });
        });
    }

    if LD && LARGE {
        group.sample_size(20);
        group.bench_function(BenchmarkId::new("Lady Deirdre", "Large File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let mut doc = Document::<JsonNode>::new_mutable("");

                    for command in &large_file {
                        match command {
                            BenchCommand::Init { text } => {
                                doc.write(.., text);
                            }

                            BenchCommand::Edit {
                                site_span, text, ..
                            } => {
                                let start = Instant::now();
                                doc.write(site_span, text);
                                time += start.elapsed();
                            }

                            BenchCommand::Wait => (),
                        }
                    }

                    black_box(doc);
                }

                time
            });
        });
    }

    if TS && SMALL {
        group.sample_size(100);
        group.bench_function(BenchmarkId::new("Tree-Sitter", "Small File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let mut parser = TSParser::new();

                    for command in &small_file {
                        match command {
                            BenchCommand::Init { text } => {
                                parser.parse(text);
                            }

                            BenchCommand::Edit {
                                site_span,
                                position_span,
                                new_end_position,
                                text,
                            } => {
                                time += parser.reparse(
                                    site_span,
                                    position_span,
                                    new_end_position,
                                    text,
                                );
                            }

                            BenchCommand::Wait => {}
                        }
                    }

                    black_box(parser);
                }

                time
            });
        });
    }

    if TS && LARGE {
        group.sample_size(20);
        group.bench_function(BenchmarkId::new("Tree-Sitter", "Large File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let mut parser = TSParser::new();

                    for command in &large_file {
                        match command {
                            BenchCommand::Init { text } => {
                                parser.parse(text);
                            }

                            BenchCommand::Edit {
                                site_span,
                                position_span,
                                new_end_position,
                                text,
                            } => {
                                time += parser.reparse(
                                    site_span,
                                    position_span,
                                    new_end_position,
                                    text,
                                );
                            }

                            BenchCommand::Wait => (),
                        }
                    }

                    black_box(parser);
                }

                time
            });
        });
    }

    group.finish();
}

pub fn bench_storage(criterion: &mut Criterion) {
    if !STORAGE {
        return;
    }

    let (small_file, large_file) = BenchData::load();

    let Some(BenchCommand::Init { text: small_text }) = small_file.iter().next() else {
        panic!("Missing Small File init command.");
    };

    let Some(BenchCommand::Init { text: large_text }) = large_file.iter().next() else {
        panic!("Missing Large File init command.");
    };

    let mut group = criterion.benchmark_group("Entire Text Input");

    if LD && SMALL {
        group.bench_function(
            BenchmarkId::new("Lady Deirdre (immutable)", "Small File"),
            |bencher| {
                bencher.iter_custom(|iters| {
                    let mut time = Duration::ZERO;

                    for _ in 0..iters {
                        let start = Instant::now();
                        let doc = Document::<VoidSyntax<LineToken>>::new_immutable(small_text);
                        time += start.elapsed();

                        black_box(doc);
                    }

                    time
                });
            },
        );
    }

    if LD && LARGE {
        group.bench_function(
            BenchmarkId::new("Lady Deirdre (immutable)", "Large File"),
            |bencher| {
                bencher.iter_custom(|iters| {
                    let mut time = Duration::ZERO;

                    for _ in 0..iters {
                        let start = Instant::now();
                        let doc = Document::<VoidSyntax<LineToken>>::new_immutable(large_text);
                        time += start.elapsed();

                        black_box(doc);
                    }

                    time
                });
            },
        );
    }

    if LD && SMALL {
        group.bench_function(
            BenchmarkId::new("Lady Deirdre (mutable)", "Small File"),
            |bencher| {
                bencher.iter_custom(|iters| {
                    let mut time = Duration::ZERO;

                    for _ in 0..iters {
                        let start = Instant::now();
                        let doc = Document::<VoidSyntax<LineToken>>::new_mutable(small_text);
                        time += start.elapsed();

                        black_box(doc);
                    }

                    time
                });
            },
        );
    }

    if LD && LARGE {
        group.bench_function(
            BenchmarkId::new("Lady Deirdre (mutable)", "Large File"),
            |bencher| {
                bencher.iter_custom(|iters| {
                    let mut time = Duration::ZERO;

                    for _ in 0..iters {
                        let start = Instant::now();
                        let doc = Document::<VoidSyntax<LineToken>>::new_mutable(large_text);
                        time += start.elapsed();

                        black_box(doc);
                    }

                    time
                });
            },
        );
    }

    if ROPEY && SMALL {
        group.bench_function(BenchmarkId::new("Ropey", "Small File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let start = Instant::now();
                    let rope = Rope::from_str(small_text);
                    time += start.elapsed();

                    black_box(rope);
                }

                time
            });
        });
    }

    if ROPEY && LARGE {
        group.bench_function(BenchmarkId::new("Ropey", "Large File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let start = Instant::now();
                    let rope = Rope::from_str(large_text);
                    time += start.elapsed();

                    black_box(rope);
                }

                time
            });
        });
    }

    group.finish();

    let mut group = criterion.benchmark_group("Keystroke Writes");

    if LD && SMALL {
        group.bench_function(BenchmarkId::new("Lady Deirdre", "Small File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let mut doc = Document::<VoidSyntax<LineToken>>::new_mutable("");

                    for command in &small_file {
                        match command {
                            BenchCommand::Init { text } => {
                                doc.write(.., text);
                            }

                            BenchCommand::Edit {
                                site_span, text, ..
                            } => {
                                let start = Instant::now();
                                doc.write(site_span, text);
                                time += start.elapsed();
                            }

                            BenchCommand::Wait => (),
                        }
                    }

                    black_box(doc);
                }

                time
            });
        });
    }

    if LD && LARGE {
        group.bench_function(BenchmarkId::new("Lady Deirdre", "Large File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let mut doc = Document::<VoidSyntax<LineToken>>::new_mutable("");

                    for command in &large_file {
                        match command {
                            BenchCommand::Init { text } => {
                                doc.write(.., text);
                            }

                            BenchCommand::Edit {
                                site_span, text, ..
                            } => {
                                let start = Instant::now();
                                doc.write(site_span, text);
                                time += start.elapsed();
                            }

                            BenchCommand::Wait => (),
                        }
                    }

                    black_box(doc);
                }

                time
            });
        });
    }

    if ROPEY && SMALL {
        group.bench_function(BenchmarkId::new("Ropey", "Small File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let mut rope = Rope::new();

                    for command in &small_file {
                        match command {
                            BenchCommand::Init { text } => {
                                rope = Rope::from_str(text);
                            }

                            BenchCommand::Edit {
                                site_span, text, ..
                            } => {
                                let start = Instant::now();
                                rope.remove(site_span.clone());
                                rope.insert(site_span.start, text);
                                time += start.elapsed();
                            }

                            BenchCommand::Wait => (),
                        }
                    }

                    black_box(rope);
                }

                time
            });
        });
    }

    if ROPEY && LARGE {
        group.bench_function(BenchmarkId::new("Ropey", "Large File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let mut rope = Rope::new();

                    for command in &large_file {
                        match command {
                            BenchCommand::Init { text } => {
                                rope = Rope::from_str(text);
                            }

                            BenchCommand::Edit {
                                site_span, text, ..
                            } => {
                                let start = Instant::now();
                                rope.remove(site_span.clone());
                                rope.insert(site_span.start, text);
                                time += start.elapsed();
                            }

                            BenchCommand::Wait => (),
                        }
                    }

                    black_box(rope);
                }

                time
            });
        });
    }

    group.finish();
}

pub fn bench_scanning(criterion: &mut Criterion) {
    if !SCAN {
        return;
    }

    let (small_file, large_file) = BenchData::load();

    let Some(BenchCommand::Init { text: small_text }) = small_file.iter().next() else {
        panic!("Missing Small File init command.");
    };

    let Some(BenchCommand::Init { text: large_text }) = large_file.iter().next() else {
        panic!("Missing Large File init command.");
    };

    let mut group = criterion.benchmark_group("Scanner");

    if LD && SMALL {
        group.bench_function(BenchmarkId::new("Lady Deirdre", "Small File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let scanner = small_text.tokens::<JsonToken>();

                    let start = Instant::now();
                    let last = scanner.last();
                    time += start.elapsed();

                    assert!(last.is_some());

                    black_box(last);
                }

                time
            });
        });
    }

    if LD && LARGE {
        group.bench_function(BenchmarkId::new("Lady Deirdre", "Large File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let scanner = large_text.tokens::<JsonToken>();

                    let start = Instant::now();
                    let last = scanner.last();
                    time += start.elapsed();

                    assert!(last.is_some());

                    black_box(last);
                }

                time
            });
        });
    }

    if LOGOS && SMALL {
        group.bench_function(BenchmarkId::new("Logos", "Small File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let scanner = LogosJsonToken::lexer(small_text);

                    let start = Instant::now();
                    let last = scanner.last();
                    time += start.elapsed();

                    assert!(last.is_some());

                    black_box(last);
                }

                time
            });
        });
    }

    if LOGOS && LARGE {
        group.bench_function(BenchmarkId::new("Logos", "Large File"), |bencher| {
            bencher.iter_custom(|iters| {
                let mut time = Duration::ZERO;

                for _ in 0..iters {
                    let scanner = LogosJsonToken::lexer(large_text);

                    let start = Instant::now();
                    let last = scanner.last();
                    time += start.elapsed();

                    assert!(last.is_some());

                    black_box(last);
                }

                time
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parsing,
    bench_reparsing,
    bench_storage,
    bench_scanning,
);
criterion_main!(benches);
