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

use std::ops::Range;

use lady_deirdre::{
    lexis::{Length, Site, SiteSpan, SourceCode},
    syntax::SyntaxTree,
    units::Document,
};
use lady_deirdre_examples::json::syntax::JsonNode;
use rand::{
    distributions::{Distribution, WeightedIndex},
    Rng,
};
use serde::{Deserialize, Serialize};

const BRANCHING: usize = 8;
const NESTING_MIN: usize = 1;
const NESTING_MAX: usize = 13;
const MB: usize = KB * KB;
const KB: usize = 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct BenchData {
    pub init: SourceSample,
    current: SourceSample,
    pub steps: Vec<SourceSample>,
}

impl BenchData {
    pub fn new(init: SourceSample) -> Self {
        Self {
            init: init.clone(),
            current: init,
            steps: Vec::default(),
        }
    }

    pub fn edit_short(&mut self, random: &mut impl Rng, mut edit: SourceSample) {
        match random.gen_range(1..6) == 1 {
            true => {
                self.current.replace(random, &mut edit);
                self.steps.push(edit);
            }

            false => {
                self.current.insert(random, &mut edit);
                self.steps.push(edit);
            }
        };
    }

    pub fn edit_long(&mut self, random: &mut impl Rng, mut edit: SourceSample) {
        match random.gen_range(1..2) == 1 {
            true => {
                self.current.replace(random, &mut edit);
                self.steps.push(edit);
            }

            false => {
                self.current.insert(random, &mut edit);
                self.steps.push(edit);
            }
        };
    }

    pub fn reset(&mut self) {
        self.current = self.init.clone();
    }

    pub fn verify_sequential(&self) {
        let mut document = Document::<JsonNode>::from(self.init.source.as_str());

        assert!(document.errors().next().is_none());

        for step in &self.steps {
            document.write(step.span.clone(), &step.source);

            assert!(document.errors().next().is_none());
        }

        assert_eq!(document.substring(..), self.current.source);
    }

    #[allow(dead_code)]
    pub fn verify_independent(&self) {
        if self.steps.is_empty() {
            let document = Document::<JsonNode>::from(self.init.source.as_str());

            assert!(document.errors().next().is_none());

            return;
        }

        for step in &self.steps {
            let mut document = Document::<JsonNode>::from(self.init.source.as_str());

            assert!(document.errors().next().is_none());

            document.write(step.span.clone(), &step.source);

            assert!(document.errors().next().is_none());
        }
    }

    pub fn describe_init(&self) -> String {
        let size = self.init.source.len();
        let lines = self.init.lines;

        if size >= MB {
            format!("{} Mb. ({} bytes, {} lines)", size / MB, size, lines)
        } else if size >= KB {
            format!("{} Kb. ({} bytes, {} lines)", size / KB, size, lines)
        } else {
            format!("{} bytes ({} lines)", size, lines)
        }
    }

    pub fn describe_average_edit(&self) -> String {
        let size = self.average_edit_size();

        if size >= MB {
            format!("{} Mb. (~{} bytes)", size / MB, size)
        } else if size >= KB {
            format!("{} Kb. (~{} bytes)", size / KB, size)
        } else {
            format!("~{} bytes", size)
        }
    }

    pub fn describe_total_edits(&self) -> String {
        let size = self.total_edit_size();

        if size >= MB {
            format!("({}) {} Mb. ({} bytes)", self.steps.len(), size / MB, size)
        } else if size >= KB {
            format!("({}) {} Kb. ({} bytes)", self.steps.len(), size / KB, size)
        } else {
            format!("({}) {} bytes", self.steps.len(), size)
        }
    }

    fn average_edit_size(&self) -> Length {
        if self.steps.is_empty() {
            return 0;
        }

        self.total_edit_size() / self.steps.len()
    }

    fn total_edit_size(&self) -> Length {
        let mut total = 0;

        for step in &self.steps {
            total += step.source.len();
        }

        total
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SourceSample {
    pub source: String,
    pub span: SiteSpan,
    lines: usize,
}

impl SourceSample {
    pub fn gen_init(random: &mut impl Rng, lines_range: Range<usize>) -> Self {
        loop {
            let nesting = random.gen_range(NESTING_MIN..NESTING_MAX);

            let source = gen_inner(random, BRANCHING, nesting);

            if source.chars().next().unwrap() != '{' {
                continue;
            }

            let lines = source.split('\n').count();

            if lines < lines_range.start {
                continue;
            }

            if lines >= lines_range.end {
                continue;
            }

            return Self {
                source,
                lines,
                span: 0..0,
            };
        }
    }

    pub fn gen_long(random: &mut impl Rng, lines_range: Range<usize>) -> Self {
        loop {
            let nesting = random.gen_range(NESTING_MIN..NESTING_MAX);

            let source = gen_inner(random, BRANCHING, nesting);

            let lines = source.split('\n').count();

            if lines < lines_range.start {
                continue;
            }

            if lines >= lines_range.end {
                continue;
            }

            return Self {
                source,
                lines,
                span: 0..0,
            };
        }
    }

    pub fn gen_short(random: &mut impl Rng, size_limit: usize) -> Self {
        let mut source;

        let nesting = match random.gen::<f64>() < 4.0 / (size_limit as f64) {
            true => 3,
            false => 1,
        };

        loop {
            source = gen_inner(random, BRANCHING, nesting);

            if source.split('\n').count() > 1 {
                continue;
            }

            if source.len() >= size_limit {
                continue;
            }

            break;
        }

        return Self {
            source,
            lines: 1,
            span: 0..0,
        };
    }

    pub fn insert(&mut self, random: &mut impl Rng, edit: &mut Self) {
        enum Candidate {
            ArrayStart(Site),
            ObjectStart(Site),
            ArrayItem(Site),
            ObjectItem(Site),
            ArrayEmpty(Site),
            ObjectEmpty(Site),
        }

        enum Context {
            Array,
            Object,
        }

        let mut stack = Vec::with_capacity(self.source.len() / 5 + 1);
        let mut candidates = Vec::with_capacity(self.source.len() / 3 + 1);

        let mut site = 0;
        let mut characters = self.source.chars().peekable();

        loop {
            let character = match characters.next() {
                Some(character) => character,

                None => break,
            };

            match character {
                '[' => {
                    stack.push(Context::Array);

                    match characters.peek().unwrap() == &']' {
                        false => {
                            candidates.push(Candidate::ArrayStart(site + 1));
                        }

                        true => {
                            candidates.push(Candidate::ArrayEmpty(site + 1));
                        }
                    }
                }

                '{' => {
                    stack.push(Context::Object);

                    match characters.peek().unwrap() == &'}' {
                        false => {
                            candidates.push(Candidate::ObjectStart(site + 1));
                        }

                        true => {
                            candidates.push(Candidate::ObjectEmpty(site + 1));
                        }
                    }
                }

                ']' | '}' => {
                    let _ = stack.pop().unwrap();
                }

                ',' => match stack.last().unwrap() {
                    Context::Array => {
                        candidates.push(Candidate::ArrayItem(site));
                    }
                    Context::Object => {
                        candidates.push(Candidate::ObjectItem(site));
                    }
                },

                _ => (),
            }

            site += 1;
        }

        let candidate = &candidates[random.gen_range(0..candidates.len())];

        match *candidate {
            Candidate::ArrayStart(site) => {
                edit.span = site..site;
                edit.source.push_str(", ");
                self.source.insert_str(site, edit.source.as_str())
            }

            Candidate::ObjectStart(site) => {
                edit.source = format!("\"key\": {}, ", edit.source);
                edit.span = site..site;
                self.source.insert_str(site, edit.source.as_str())
            }

            Candidate::ArrayItem(site) => {
                edit.source = format!(", {}", edit.source);
                edit.span = site..site;
                self.source.insert_str(site, edit.source.as_str())
            }

            Candidate::ObjectItem(site) => {
                edit.source = format!(", \"key\": {}", edit.source);
                edit.span = site..site;
                self.source.insert_str(site, edit.source.as_str())
            }
            Candidate::ArrayEmpty(site) => {
                edit.span = site..site;
                self.source.insert_str(site, edit.source.as_str())
            }

            Candidate::ObjectEmpty(site) => {
                edit.source = format!("\"key\": {}", edit.source);
                edit.span = site..site;
                self.source.insert_str(site, edit.source.as_str())
            }
        }
    }

    pub fn replace(&mut self, random: &mut impl Rng, edit: &mut Self) {
        enum Context {
            Array(Site),
            Object(Site),
        }

        impl Context {
            fn start(self) -> Site {
                match self {
                    Self::Array(site) => site,
                    Self::Object(site) => site,
                }
            }
        }

        let deletion_limit = edit.source.len() * 3;

        let mut stack = Vec::with_capacity(self.source.len() / 5 + 1);
        let mut candidates = Vec::with_capacity(self.source.len() / 3 + 1);

        let mut site = 0;
        let mut characters = self.source.chars().peekable();

        loop {
            let character = match characters.next() {
                Some(character) => character,

                None => break,
            };

            match character {
                '[' => {
                    stack.push(Context::Array(site));
                }

                '{' => {
                    stack.push(Context::Object(site));
                }

                ']' | '}' => {
                    let start = stack.pop().unwrap().start();

                    if start > 0 && site - start < deletion_limit {
                        candidates.push(start..(site + 1))
                    }
                }

                '"' if characters.peek().unwrap() == &'s' => candidates.push(site..(site + 8)),

                't' if characters.peek().unwrap() == &'r' => candidates.push(site..(site + 4)),

                'n' if characters.peek().unwrap() == &'u' => candidates.push(site..(site + 4)),

                'f' | '1' => candidates.push(site..(site + 5)),

                _ => (),
            }

            site += 1;
        }

        let candidate = candidates[random.gen_range(0..candidates.len())].clone();

        edit.span = candidate.clone();

        self.source = format!(
            "{}{}{}",
            &self.source[0..candidate.start],
            edit.source.as_str(),
            &self.source[candidate.end..]
        );
    }
}

fn gen_inner(random: &mut impl Rng, branching: usize, nesting: usize) -> String {
    let distribution = match nesting == 0 {
        true => WeightedIndex::new(&[1usize, 1, 1, 1, 1]).unwrap(),
        false => WeightedIndex::new(&[1, 1, 1, 1, 1, 7, 7]).unwrap(),
    };

    match distribution.sample(random) + 1 {
        1 => String::from(r#"true"#),
        2 => String::from(r#"false"#),
        3 => String::from(r#"null"#),
        4 => String::from(r#"12345"#),
        5 => String::from(r#""STRING""#),

        6 => match random.gen_range(0..branching) {
            0 => String::from(r#"[]"#),
            1 => format!("[{}]", gen_inner(random, branching, nesting - 1)),

            other => {
                let mut result = String::from('[');

                match nesting == 1 {
                    true => {
                        for index in 0..other {
                            if index > 0 {
                                result.push_str(", ");
                            }

                            result.push_str(&gen_inner(random, branching, nesting - 1))
                        }

                        result.push_str("]");
                    }

                    false => {
                        for index in 0..other {
                            match index == 0 {
                                true => result.push_str("\n    "),
                                false => result.push_str(",\n    "),
                            }

                            result.push_str(&shift(gen_inner(random, branching, nesting - 1)))
                        }

                        result.push_str("\n]");
                    }
                }

                result
            }
        },

        7 => match random.gen_range(0..branching) {
            0 => String::from(r#"{}"#),
            1 => format!(
                r#"{{"key": {}}}"#,
                gen_inner(random, branching, nesting - 1)
            ),

            other => {
                let mut result = String::from('{');

                match nesting == 1 {
                    true => {
                        for index in 0..other {
                            if index > 0 {
                                result.push_str(", ");
                            }

                            result.push_str(r#""key": "#);
                            result.push_str(&gen_inner(random, branching, nesting - 1))
                        }

                        result.push_str("}");
                    }

                    false => {
                        for index in 0..other {
                            match index == 0 {
                                true => result.push_str("\n    "),
                                false => result.push_str(",\n    "),
                            }

                            result.push_str(r#""key": "#);
                            result.push_str(&shift(gen_inner(random, branching, nesting - 1)))
                        }

                        result.push_str("\n}");
                    }
                }

                result
            }
        },

        _ => unreachable!(),
    }
}

fn shift(text: String) -> String {
    text.split('\n')
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                return line.to_string();
            }

            return format!("    {}", line);
        })
        .collect::<Vec<_>>()
        .join("\n")
}
