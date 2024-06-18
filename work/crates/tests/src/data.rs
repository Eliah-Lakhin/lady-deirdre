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

use std::{
    fmt::{Debug, Formatter},
    fs::{read_to_string, remove_file, write, File},
    io::Write,
    thread::sleep,
    time::Duration,
};

use dirs::cache_dir;
use lady_deirdre::{
    lexis::{Column, Line, Position, PositionSpan, Site, SiteSpan, SourceCode, ToSite, ToSpan},
    syntax::SyntaxTree,
    units::Document,
};
use lady_deirdre_examples::json_grammar::syntax::JsonNode;
use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::gen::{BranchingWeights, JsonBootstrapGen, JsonEditsGen, JsonGenConfig};

#[derive(Serialize, Deserialize)]
pub struct BenchData {
    title: String,
    seed: u64,
    config: JsonGenConfig,
    edits: usize,
    bytes: usize,
    lines: usize,
    text: String,
    ops: Vec<Op>,
}

impl Debug for BenchData {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BenchData")
            .field("title", &self.title())
            .field("seed", &self.seed())
            .field("size", &self.size())
            .field("lines", &self.lines())
            .field("edits", &self.edits())
            .field("ops", &self.ops())
            .finish()
    }
}

impl<'a> IntoIterator for &'a BenchData {
    type Item = BenchCommand<'a>;
    type IntoIter = BenchDataIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        BenchDataIterator {
            data: self,
            text_index: 0,
            op_index: 0,
        }
    }
}

impl BenchData {
    pub const DEFAULT_SEED: u64 = 324601853275;

    pub const DEFAULT_EDITS: usize = 100;

    pub const SMALL_FILE_CONFIG: JsonGenConfig = JsonGenConfig {
        type_by_chars: true,
        tree_max_depth: 10,
        object_max_branching: 7,
        array_max_branching: 9,
        code_min_length: 60 * 1024,
        code_max_length: 65 * 1024,
        ..JsonGenConfig::new()
    };

    pub const LARGE_FILE_CONFIG: JsonGenConfig = JsonGenConfig {
        type_by_chars: true,
        tree_max_depth: 16,
        object_max_branching: 7,
        array_max_branching: 9,
        code_min_length: 1 * 1024 * 1024,
        code_max_length: 2 * 1024 * 1024,
        grow_weights: BranchingWeights {
            object: 10,
            array: 10,
            string: 5,
            num: 3,
            true_w: 1,
            false_w: 1,
            null: 1,
        },
        ..JsonGenConfig::new()
    };

    pub fn load() -> (Self, Self) {
        println!("Loading bench data...");

        let small = Self::load_from_cache(
            "Small File",
            Self::DEFAULT_SEED,
            Self::SMALL_FILE_CONFIG,
            Self::DEFAULT_EDITS,
        );

        let large = Self::load_from_cache(
            "Large File",
            Self::DEFAULT_SEED,
            Self::LARGE_FILE_CONFIG,
            Self::DEFAULT_EDITS,
        );

        (small, large)
    }

    fn load_from_cache(title: &str, seed: u64, config: JsonGenConfig, edits: usize) -> Self {
        let name = title.to_ascii_lowercase().replace(' ', "-");

        let mut path = match cache_dir() {
            Some(path) => path,
            None => {
                println!("Missing cache directory.");
                return Self::generate(title, seed, config, edits);
            }
        };

        path.push(format!(".ld-bench-data-{}-{}.json", name, seed));

        let mut save = false;
        let deserialized;

        loop {
            if !path.exists() {
                println!("{path:?}: File does not exist.",);

                save = true;
                deserialized = Self::generate(title, seed, config, edits);
                break;
            }

            match read_to_string(&path) {
                Ok(string) => {
                    deserialized = match serde_json::from_str::<Self>(string.as_str()) {
                        Ok(data) => match data.compare_content(title, seed, config, edits) {
                            false => {
                                println!("{path:?}: File content mismatch.");
                                save = true;
                                Self::generate(title, seed, config, edits)
                            }

                            true => data,
                        },

                        Err(error) => {
                            println!("{path:?}: Deserialization error. {error}",);

                            save = true;
                            Self::generate(title, seed, config, edits)
                        }
                    };
                }

                Err(error) => {
                    println!("{path:?}: Read error. {error}",);

                    save = true;
                    deserialized = Self::generate(title, seed, config, edits);
                }
            };

            break;
        }

        match save {
            true => {
                let serialized = match serde_json::to_string(&deserialized) {
                    Ok(data) => {
                        println!("{path:?}: Serialization finished.");
                        data
                    }

                    Err(error) => {
                        println!("{path:?}: Serialization error. {error}.");
                        return deserialized;
                    }
                };

                match write(&path, serialized) {
                    Ok(()) => {
                        println!("{path:?}: Data saved to file.");
                    }

                    Err(error) => {
                        println!("{path:?}: File save error. {error}");
                    }
                }
            }

            false => {
                println!("Bench data {name:?} with seed {seed} loaded from file.",);
            }
        }

        deserialized
    }

    pub fn generate(
        title: impl Into<String>,
        seed: u64,
        config: JsonGenConfig,
        edits: usize,
    ) -> Self {
        let title = title.into();

        println!("{title}: Generating data with seed {seed}...");

        sleep(Duration::from_secs(1));

        const GEN_ATTEMPTS: usize = 100;

        let mut rng = StdRng::seed_from_u64(seed);

        let text = JsonBootstrapGen::gen(config, &mut rng, GEN_ATTEMPTS);
        let text_len = text.len();

        if text_len < config.code_min_length || text_len > config.code_max_length {
            panic!(
                "Enable to generate bench data.\n\
                Seed: {seed}.\n\
                Attempts: {GEN_ATTEMPTS}.\n\
                Best result: {text_len}\n\
                Configuration: {config:#?}"
            );
        }

        println!("{title}: Initial text generated.");

        let mut builder = Self::build(title.clone(), seed, config, edits);

        builder.init(text.clone());

        let mut edits_gen = JsonEditsGen::new(config, text);

        for edit in 1..=edits {
            println!("{title}: Text edit {edit} generated.");

            edits_gen.gen_edits(&mut rng);

            for edit in edits_gen.take_edits() {
                builder.edit(edit.text, edit.span);
            }

            builder.wait();
        }

        let result = builder.finish();

        println!("{title}: Benchmark data is ready. {result:#?}");

        result
    }

    pub fn build(
        title: impl Into<String>,
        seed: u64,
        config: JsonGenConfig,
        edits: usize,
    ) -> BenchDataBuilder {
        BenchDataBuilder {
            result: BenchData {
                title: title.into(),
                seed,
                config,
                edits,
                bytes: 0,
                lines: 0,
                text: String::new(),
                ops: Vec::new(),
            },

            doc: Document::new_mutable(""),
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn size(&self) -> String {
        Self::format_size(self.bytes)
    }

    pub fn lines(&self) -> usize {
        self.lines
    }

    pub fn edits(&self) -> usize {
        self.edits
    }

    pub fn ops(&self) -> usize {
        self.ops.len()
    }

    pub fn iter(&self) -> BenchDataIterator {
        self.into_iter()
    }

    pub fn check(&self) {
        let mut doc = Document::<JsonNode>::new_mutable("");

        for (index, command) in self.iter().enumerate() {
            match command {
                BenchCommand::Init { text } => {
                    doc.write(.., text);

                    let has_errors = doc.errors().next().is_some();

                    if has_errors {
                        for error in doc.errors() {
                            println!("{:#}", error.display(&doc))
                        }

                        panic!(
                            "{}: Command {index}. Initial data with syntax errors.",
                            self.title,
                        );
                    }
                }

                BenchCommand::Edit {
                    site_span,
                    position_span,
                    new_end_position,
                    text,
                } => {
                    let Some(prototype) = site_span.to_position_span(&doc) else {
                        panic!("{}: Command {index}. Invalid edit site span.", self.title);
                    };

                    if prototype != position_span {
                        panic!(
                            "{}: Command {index}. Site and Position spans mismatch.",
                            self.title,
                        );
                    }

                    doc.write(&site_span, text);

                    let Some(prototype) = (site_span.start + text.len()).to_position(&doc) else {
                        panic!("{}: Command {index}. Invalid edit site span.", self.title);
                    };

                    if prototype != new_end_position {
                        panic!(
                            "{}: Command {index}. Site and Position spans mismatch.",
                            self.title,
                        );
                    }
                }

                BenchCommand::Wait => {
                    let has_errors = doc.errors().next().is_some();

                    if has_errors {
                        for error in doc.errors() {
                            println!("{:#}", error.display(&doc))
                        }

                        panic!("{}: Command {index}. Wait after syntax errors.", self.title);
                    }
                }
            }
        }

        println!("OK. {:#?}", self);
    }

    pub fn format_size(bytes: usize) -> String {
        if bytes < 1024 {
            return format!("{} Bs", bytes);
        }

        if bytes < 1024 * 1024 {
            return format!("{} KBs", bytes / 1024);
        }

        format!("{} MBs", bytes / 1024 / 1024)
    }

    fn compare_content(&self, title: &str, seed: u64, config: JsonGenConfig, edits: usize) -> bool {
        if self.title != title {
            return false;
        }

        if self.seed != seed {
            return false;
        }

        if self.config != config {
            return false;
        }

        if self.edits != edits {
            return false;
        }

        true
    }
}

pub enum BenchCommand<'a> {
    Init {
        text: &'a str,
    },

    Edit {
        site_span: SiteSpan,
        position_span: PositionSpan,
        new_end_position: Position,
        text: &'a str,
    },

    Wait,
}

pub struct BenchDataBuilder {
    result: BenchData,
    doc: Document<JsonNode>,
}

impl BenchDataBuilder {
    pub fn init(&mut self, text: impl Into<String>) {
        let text = text.into();
        let bytes = text.len();

        self.result.text.push_str(&text);
        self.doc.write(.., text);

        let lines = self.doc.lines().lines_count();
        let length = self.doc.length();

        self.result.ops.push(Op::Init { length });

        self.result.bytes = bytes;
        self.result.lines = lines;
    }

    pub fn edit(&mut self, text: impl Into<String>, span: SiteSpan) {
        let text = text.into();
        let length = text.len();
        let position_span = span.to_position_span(&self.doc).unwrap();

        self.result.text.push_str(&text);
        self.doc.write(&span, text);

        let new_end_position = (span.start + length).to_position(&self.doc).unwrap();

        self.result.ops.push(Op::Edit {
            length,
            start_site: span.start,
            start_line: position_span.start.line,
            start_column: position_span.start.column,
            end_site: span.end,
            end_line: position_span.end.line,
            end_column: position_span.end.column,
            new_end_line: new_end_position.line,
            new_end_column: new_end_position.column,
        });
    }

    pub fn wait(&mut self) {
        self.result.ops.push(Op::Wait);
    }

    pub fn finish(self) -> BenchData {
        self.result
    }
}

pub struct BenchDataIterator<'a> {
    data: &'a BenchData,
    text_index: usize,
    op_index: usize,
}

impl<'a> Iterator for BenchDataIterator<'a> {
    type Item = BenchCommand<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let op = self.data.ops.get(self.op_index)?;

        match op {
            Op::Init { length } => {
                let text = &self.data.text[self.text_index..(self.text_index + *length)];
                self.text_index += *length;
                self.op_index += 1;

                Some(BenchCommand::Init { text })
            }

            Op::Edit {
                length,
                start_site,
                start_line,
                start_column,
                end_site,
                end_line,
                end_column,
                new_end_line,
                new_end_column,
            } => {
                let text = &self.data.text[self.text_index..(self.text_index + length)];
                self.text_index += length;
                self.op_index += 1;

                Some(BenchCommand::Edit {
                    site_span: *start_site..*end_site,
                    position_span: Position::new(*start_line, *start_column)
                        ..Position::new(*end_line, *end_column),
                    new_end_position: Position::new(*new_end_line, *new_end_column),
                    text,
                })
            }

            Op::Wait => {
                self.op_index += 1;

                Some(BenchCommand::Wait)
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
enum Op {
    Init {
        length: usize,
    },

    Edit {
        length: usize,
        start_site: Site,
        start_line: Line,
        start_column: Column,
        end_site: Site,
        end_line: Line,
        end_column: Column,
        new_end_line: Line,
        new_end_column: Column,
    },

    Wait,
}
