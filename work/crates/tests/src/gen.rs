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

use std::{mem::take, ops::Range};

use lady_deirdre::{
    lexis::{Length, Site, SiteSpan, SourceCode, ToSpan},
    syntax::{NodeRef, PolyRef, SyntaxTree},
    units::{CompilationUnit, Document},
};
use lady_deirdre_examples::json_grammar::syntax::JsonNode;
use petname::{Generator, Petnames};
use rand::{
    distributions::{uniform::SampleRange, Distribution, WeightedIndex},
    Rng,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct JsonEdit {
    pub span: SiteSpan,
    pub text: String,
}

impl JsonEdit {
    fn new(span: SiteSpan, text: impl Into<String>) -> Self {
        Self {
            span,
            text: text.into(),
        }
    }

    fn apply_to_doc(&self, doc: &mut Document<JsonNode>) {
        doc.write(&self.span, &self.text);
    }

    fn apply_to_string(&self, string: &mut String) {
        let start = string.chars().take(self.span.start).count();
        let end = string.chars().take(self.span.end).count();

        let mut right = string.split_off(start);
        right = right.split_off(end - start);

        string.push_str(&self.text);
        string.push_str(&right);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionWeights {
    pub grow: u8,
    pub shrink: u8,
    pub rewrite: u8,
}

impl ActionWeights {
    fn enumerate(&self) -> [u8; 3] {
        [self.grow, self.shrink, self.rewrite]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BranchingWeights {
    pub object: u8,
    pub array: u8,
    pub string: u8,
    pub num: u8,
    pub true_w: u8,
    pub false_w: u8,
    pub null: u8,
}

impl BranchingWeights {
    fn enumerate(&self) -> [u8; 7] {
        [
            self.object,
            self.array,
            self.string,
            self.num,
            self.true_w,
            self.false_w,
            self.null,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct JsonGenConfig {
    pub auto_close: bool,
    pub type_by_chars: bool,
    pub indent: usize,
    pub edits_after_grow: usize,
    pub grow_limit: usize,
    pub tree_max_depth: usize,
    pub object_max_branching: usize,
    pub array_max_branching: usize,
    pub code_min_length: Length,
    pub code_max_length: Length,
    pub descend_probability: f64,
    pub local_edits: usize,
    pub action_weights: ActionWeights,
    pub grow_weights: BranchingWeights,
    pub rewrite_weights: BranchingWeights,
    pub key_string_max_words: usize,
    pub value_string_max_words: usize,
    pub floats_probability: f64,
    pub checks: bool,
}

impl Default for JsonGenConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonGenConfig {
    pub const fn new() -> Self {
        Self {
            auto_close: true,
            type_by_chars: false,
            indent: 4,
            edits_after_grow: 10,
            grow_limit: 10,
            tree_max_depth: 8,
            object_max_branching: 5,
            array_max_branching: 7,
            code_min_length: 60 * 100,
            code_max_length: 60 * 200,
            descend_probability: 0.7,
            local_edits: 5,
            action_weights: ActionWeights {
                grow: 5,
                shrink: 1,
                rewrite: 2,
            },
            grow_weights: BranchingWeights {
                object: 7,
                array: 6,
                string: 5,
                num: 3,
                true_w: 1,
                false_w: 1,
                null: 1,
            },
            rewrite_weights: BranchingWeights {
                object: 1,
                array: 1,
                string: 5,
                num: 5,
                true_w: 5,
                false_w: 5,
                null: 5,
            },
            key_string_max_words: 2,
            value_string_max_words: 4,
            floats_probability: 0.3,
            checks: true,
        }
    }
}

pub struct JsonEditsGen {
    config: JsonGenConfig,
    doc: Document<JsonNode>,
    edits: Vec<JsonEdit>,
    committed: usize,
    action_dist: WeightedIndex<u8>,
    grow_dist: WeightedIndex<u8>,
    rewrite_dist: WeightedIndex<u8>,
    key_names: Petnames<'static>,
    value_names: Petnames<'static>,
    content: String,
}

impl JsonEditsGen {
    pub fn new(config: JsonGenConfig, text: impl AsRef<str>) -> Self {
        let text = text.as_ref();

        if config.checks && !text.is_empty() {
            let doc = Document::<JsonNode>::new_immutable(&text);

            assert!(doc.errors().next().is_none());
        }

        let grow_dist = WeightedIndex::new(&config.grow_weights.enumerate()).unwrap();
        let rewrite_dist = WeightedIndex::new(&config.rewrite_weights.enumerate()).unwrap();
        let action_dist = WeightedIndex::new(&config.action_weights.enumerate()).unwrap();

        let content = match config.checks {
            true => String::from(text),
            false => String::new(),
        };

        Self {
            config,
            doc: Document::new_mutable(text),
            edits: Vec::new(),
            committed: 0,
            action_dist,
            grow_dist,
            rewrite_dist,
            key_names: Petnames::small(),
            value_names: Petnames::large(),
            content,
        }
    }

    pub fn doc(&self) -> &Document<JsonNode> {
        &self.doc
    }

    pub fn check(&self) {
        if !self.config.checks {
            return;
        }

        if self.content != self.doc.substring(..) {
            panic!("Content mismatch.");
        }
    }

    pub fn take_edits(&mut self) -> Vec<JsonEdit> {
        let result = take(&mut self.edits);

        self.committed = 0;

        result
    }

    pub fn gen_edits(&mut self, rng: &mut impl Rng) {
        let mut action = self.choose_action(rng);
        let mut inner_attempt = 0;

        loop {
            inner_attempt += 1;

            let Some((depth, node_ref)) =
                self.pick_node(rng, &self.doc.root_node_ref(), 0, &action)
            else {
                continue;
            };

            let Some(node) = node_ref.deref(&self.doc) else {
                panic!("Malformed node.");
            };

            match node {
                JsonNode::Root { .. } => {
                    self.write_to_root();
                    self.commit();
                }

                JsonNode::Object { .. } => {
                    if !self.modify_struct(rng, node_ref, depth, true, &action) {
                        if inner_attempt > self.config.grow_limit {
                            action = Action::Rewrite;
                        }
                        continue;
                    }

                    self.commit();

                    let mut more_edits = rng.gen_range(0..self.config.local_edits);

                    for _ in 0..more_edits {
                        if !self.modify_struct(rng, node_ref, depth, true, &action) {
                            break;
                        }

                        self.commit();
                    }
                }

                JsonNode::Entry { .. } => {
                    if !self.modify_entry(rng, node_ref, &action) {
                        continue;
                    }

                    self.commit();
                }

                JsonNode::Array { .. } => {
                    if !self.modify_struct(rng, node_ref, depth, false, &action) {
                        if inner_attempt > self.config.grow_limit {
                            action = Action::Rewrite;
                        }
                        continue;
                    }

                    self.commit();

                    let mut more_edits = rng.gen_range(0..self.config.local_edits);

                    for _ in 0..more_edits {
                        if !self.modify_struct(rng, node_ref, depth, false, &action) {
                            break;
                        }

                        self.commit();
                    }
                }

                JsonNode::String { .. }
                | JsonNode::Number { .. }
                | JsonNode::True { .. }
                | JsonNode::False { .. }
                | JsonNode::Null { .. } => {
                    if !self.modify_leaf(rng, node_ref, &action) {
                        continue;
                    }

                    self.commit();
                }
            }

            break;
        }
    }

    fn write_to_root(&mut self) {
        let mut site = 0;

        self.write_empty_object(&mut site);
        self.write_break(&mut site);
    }

    fn modify_struct(
        &mut self,
        rng: &mut impl Rng,
        node_ref: NodeRef,
        depth: usize,
        as_object: bool,
        action: &Action,
    ) -> bool {
        let max_branching;
        let components;

        match node_ref.deref(&self.doc) {
            Some(JsonNode::Object { entries, .. }) => {
                max_branching = self.config.object_max_branching;
                components = entries
            }

            Some(JsonNode::Array { items, .. }) => {
                max_branching = self.config.array_max_branching;
                components = items
            }
            _ => return false,
        };

        match action {
            Action::Grow => {
                if components.len() >= self.config.array_max_branching {
                    return false;
                }

                let Some(span) = node_ref.span(&self.doc) else {
                    panic!("Malformed node.");
                };

                let index = rng.gen_range(0..=components.len());

                let is_first = index == 0;
                let is_last = index == components.len();

                let mut site;

                match is_first {
                    true => {
                        site = span.start + 1;
                    }

                    false => {
                        let previous = &components[index - 1];

                        let Some(span) = previous.span(&self.doc) else {
                            panic!("Malformed node.");
                        };

                        site = span.end;

                        self.write_word(&mut site, ",");
                    }
                }

                self.write_break(&mut site);
                self.write_indent(&mut site, depth + 1);

                if as_object {
                    self.write_new_string(rng, &mut site, false);
                    self.write_word(&mut site, ": ");
                }

                self.write_leaf(rng, &mut site, &Action::Grow);

                match (is_first, is_last) {
                    (true, true) => {
                        self.write_break(&mut site);
                        self.write_indent(&mut site, depth);
                    }

                    (true, false) => {
                        self.write_word(&mut site, ",");
                    }

                    _ => (),
                }
            }

            Action::Shrink => match components.len() {
                0 => return false,

                1 => {
                    let Some(mut span) = node_ref.span(&self.doc) else {
                        panic!("Malformed node.");
                    };

                    if self.doc.length() < self.config.code_min_length + span.len() {
                        return false;
                    }

                    let mut site = span.start + 1;

                    self.erase_right(&mut site, span.end - span.start - 2);
                }

                _ => {
                    let mut chosen_length = Length::MAX;
                    let mut chosen_index = components.len() - 1;

                    for (probe, component_ref) in components.iter().rev().enumerate() {
                        let Some(mut span) = component_ref.span(&self.doc) else {
                            panic!("Malformed node.");
                        };

                        let length = span.end - span.start;

                        if length < chosen_length {
                            continue;
                        }

                        chosen_length = length;
                        chosen_index = probe;
                    }

                    let Some(component_span) = components[chosen_index].span(&self.doc) else {
                        panic!("Malformed node.");
                    };

                    if self.doc.length() < self.config.code_min_length + component_span.len() {
                        return false;
                    }

                    let is_last = chosen_index == components.len() - 1;

                    let mut site = component_span.start;

                    self.erase_right(&mut site, component_span.end - component_span.start);

                    let indent_length = (depth + 1) * self.config.indent;

                    self.erase_left(&mut site, indent_length + 1);

                    match is_last {
                        true => self.erase_left(&mut site, 1),
                        false => self.erase_right(&mut site, 1),
                    }
                }
            },

            Action::Rewrite => {
                let Some(span) = node_ref.span(&self.doc) else {
                    panic!("Malformed node.");
                };

                let length = span.end - span.start;

                if self.doc.length() < self.config.code_min_length + length {
                    return false;
                }

                let mut site = span.start;

                self.erase_right(&mut site, length);
                self.write_leaf(rng, &mut site, &Action::Rewrite);
            }
        }

        true
    }

    fn modify_entry(&mut self, rng: &mut impl Rng, node_ref: NodeRef, action: &Action) -> bool {
        let Some(JsonNode::Entry { key, .. }) = node_ref.deref(&self.doc) else {
            return false;
        };

        let Action::Rewrite = action else {
            return false;
        };

        let Some(span) = key.span(&self.doc) else {
            panic!("Malformed node.");
        };

        let mut site = span.start;

        self.erase_right(&mut site, span.end - span.start);

        self.write_new_string(rng, &mut site, false);

        true
    }

    fn modify_leaf(&mut self, rng: &mut impl Rng, node_ref: NodeRef, action: &Action) -> bool {
        match node_ref.deref(&self.doc) {
            Some(
                JsonNode::String { .. }
                | JsonNode::Number { .. }
                | JsonNode::True { .. }
                | JsonNode::False { .. }
                | JsonNode::Null { .. },
            ) => (),

            _ => return false,
        }

        let Action::Rewrite = action else {
            return false;
        };

        let Some(span) = node_ref.span(&self.doc) else {
            panic!("Malformed node.");
        };

        let mut site = span.start;

        self.erase_right(&mut site, span.end - span.start);

        match node_ref.parent(&self.doc).deref(&self.doc) {
            Some(JsonNode::Entry { key, .. }) => match key == &node_ref {
                true => self.write_new_string(rng, &mut site, false),
                false => self.write_leaf(rng, &mut site, action),
            },

            Some(_) => self.write_leaf(rng, &mut site, action),

            None => panic!("Malformed node."),
        }

        true
    }

    fn write_leaf(&mut self, rng: &mut impl Rng, site: &mut Site, action: &Action) {
        let weights = match action.is_grow() {
            true => &self.grow_dist,
            false => &self.rewrite_dist,
        };

        match weights.sample(rng) {
            0 => self.write_empty_object(site),
            1 => self.write_empty_array(site),
            2 => self.write_new_string(rng, site, true),
            3 => self.write_new_num(rng, site),
            4 => self.write_word(site, "true"),
            5 => self.write_word(site, "false"),
            6 => self.write_word(site, "null"),

            _ => panic!("Malformed leaf index."),
        }
    }

    fn write_empty_object(&mut self, site: &mut Site) {
        match self.config.auto_close {
            true => {
                self.write_whole_word(site, "{}");
            }
            false => {
                self.write_word(site, "{");
                self.write_word(site, "}");
            }
        }
    }

    fn write_empty_array(&mut self, site: &mut Site) {
        match self.config.auto_close {
            true => {
                self.write_whole_word(site, "[]");
            }
            false => {
                self.write_word(site, "[");
                self.write_word(site, "]");
            }
        }
    }

    fn write_new_string(&mut self, rng: &mut impl Rng, site: &mut Site, as_value: bool) {
        match self.config.auto_close {
            true => {
                self.write_whole_word(site, "\"\"");
                *site -= 1;
            }
            false => {
                self.write_word(site, "\"");
            }
        }

        let words = match as_value {
            true => rng.gen_range(1..=self.config.value_string_max_words),
            false => rng.gen_range(1..=self.config.key_string_max_words),
        };

        for index in 0..words {
            match as_value {
                true => {
                    if index > 0 {
                        self.write_word(site, " ");
                    }

                    let word = self.value_names.generate(rng, 1, "").unwrap();

                    self.write_word(site, word);
                }

                false => {
                    if index > 0 {
                        self.write_word(site, "-");
                    }

                    let word = self.key_names.generate(rng, 1, "").unwrap();

                    self.write_word(site, word);
                }
            }
        }

        match self.config.auto_close {
            true => {
                *site += 1;
            }
            false => {
                self.write_word(site, "\"");
            }
        }
    }

    fn write_new_num(&mut self, rng: &mut impl Rng, site: &mut Site) {
        match rng.gen_bool(self.config.floats_probability) {
            true => {
                let head = rng.gen_range(0..=0xFFFFu32).to_string();
                let tail = rng.gen_range(0..=0xFFFFu32).to_string();

                self.write_word(site, head);
                self.write_word(site, ".");
                self.write_word(site, tail);
            }

            false => {
                let head = rng.gen_range(0..=0xFFFFFFFFu32).to_string();

                self.write_word(site, head);
            }
        }
    }

    fn write_word(&mut self, site: &mut Site, word: impl Into<String>) {
        if !self.config.type_by_chars {
            self.write_whole_word(site, word);
            return;
        }

        let word = word.into();

        for ch in word.chars() {
            self.edits.push(JsonEdit::new(*site..*site, ch));
            *site += 1;
        }
    }

    fn write_whole_word(&mut self, site: &mut Site, word: impl Into<String>) {
        let word = word.into();
        let chars = word.chars().count();

        self.edits.push(JsonEdit::new(*site..*site, word));

        *site += chars;
    }

    fn write_break(&mut self, site: &mut Site) {
        self.edits.push(JsonEdit::new(*site..*site, "\n"));
        *site += 1;
    }

    fn write_blank(&mut self, site: &mut Site, chars: usize) {
        let whitespaces = " ".repeat(chars);

        self.edits.push(JsonEdit::new(*site..*site, whitespaces));
        *site += chars;
    }

    fn write_indent(&mut self, site: &mut Site, depth: usize) {
        self.write_blank(site, depth * self.config.indent)
    }

    fn erase_right(&mut self, site: &mut Site, length: Length) {
        self.edits.push(JsonEdit::new(*site..(*site + length), ""));
    }

    fn erase_left(&mut self, site: &mut Site, length: Length) {
        self.edits.push(JsonEdit::new((*site - length)..*site, ""));

        *site -= length;
    }

    fn commit(&mut self) {
        if self.committed == self.edits.len() {
            panic!("Empty commit.");
        }

        for edit in &self.edits[self.committed..self.edits.len()] {
            edit.apply_to_doc(&mut self.doc);

            if self.config.checks {
                edit.apply_to_string(&mut self.content);
            }
        }

        if self.config.checks {
            let has_errors = self.doc.errors().next().is_some();

            if has_errors {
                self.check();

                for error in self.doc.errors() {
                    println!("{:#}", error.display(&self.doc));
                }

                let doc2 = Document::<JsonNode>::new_immutable(&self.content);

                match doc2.errors().next().is_some() {
                    true => panic!("Syntax errors committed."),
                    false => {
                        panic!(
                            "Mutable document has syntax errors, \
                            but Immutable document did not detect errors.",
                        );
                    }
                }
            }
        }

        self.committed = self.edits.len();
    }

    fn pick_node(
        &self,
        rng: &mut impl Rng,
        node_ref: &NodeRef,
        depth: usize,
        action: &Action,
    ) -> Option<(usize, NodeRef)> {
        if depth > self.config.tree_max_depth {
            return Some((depth, *node_ref));
        }

        let Some(node) = node_ref.deref(&self.doc) else {
            return None;
        };

        match node {
            JsonNode::Root { object, .. } => {
                if !object.is_valid_ref(&self.doc) {
                    return Some((depth, *node_ref));
                }

                self.pick_node(rng, object, depth, action)
            }

            JsonNode::Object { entries, .. } => {
                if !rng.gen_bool(self.config.descend_probability) || entries.is_empty() {
                    return Some((depth, *node_ref));
                }

                let index = rng.gen_range(0..entries.len());

                self.pick_node(rng, &entries[index], depth + 1, action)
                    .or(Some((depth, *node_ref)))
            }

            JsonNode::Entry { key, value, .. } => {
                if !rng.gen_bool(self.config.descend_probability) {
                    return Some((depth, *key));
                }

                self.pick_node(rng, value, depth, action)
            }

            JsonNode::Array { items, .. } => {
                if !rng.gen_bool(self.config.descend_probability) || items.is_empty() {
                    return Some((depth, *node_ref));
                }

                let index = rng.gen_range(0..items.len());

                self.pick_node(rng, &items[index], depth + 1, action)
                    .or(Some((depth, *node_ref)))
            }

            JsonNode::String { .. }
            | JsonNode::Number { .. }
            | JsonNode::True { .. }
            | JsonNode::False { .. }
            | JsonNode::Null { .. }
                if !action.is_grow() =>
            {
                Some((depth, *node_ref))
            }

            _ => None,
        }
    }

    fn choose_action(&self, rng: &mut impl Rng) -> Action {
        if self.doc.length() < self.config.code_min_length {
            return Action::Grow;
        }

        if self.doc.length() > self.config.code_max_length {
            return Action::Shrink;
        }

        match self.action_dist.sample(rng) {
            0 => Action::Grow,
            1 => Action::Shrink,
            2 => Action::Rewrite,

            _ => panic!("Malformed action index."),
        }
    }
}

pub struct JsonBootstrapGen {
    config: JsonGenConfig,
    target: String,
    key_names: Petnames<'static>,
    value_names: Petnames<'static>,
    grow_dist: WeightedIndex<u8>,
}

impl JsonBootstrapGen {
    pub fn gen(config: JsonGenConfig, rng: &mut impl Rng, mut attempts: usize) -> String {
        let grow_dist = WeightedIndex::new(&config.grow_weights.enumerate()).unwrap();

        let mut generator = Self {
            config,
            target: String::new(),
            grow_dist,
            key_names: Petnames::small(),
            value_names: Petnames::large(),
        };

        let mut best_text = String::new();
        let mut best_diff = usize::MAX;

        loop {
            attempts = match attempts.checked_sub(1) {
                Some(left) => left,
                None => break,
            };

            generator.gen_struct(rng, 0, true);
            generator.target.push('\n');

            let target_len = generator.target.len();

            if target_len >= generator.config.code_min_length
                && target_len <= generator.config.code_max_length
            {
                best_text = generator.target;
                break;
            }

            let target_diff = target_len
                .abs_diff(generator.config.code_min_length)
                .min(target_len.abs_diff(generator.config.code_max_length));

            match target_diff < best_diff {
                true => {
                    best_text = take(&mut generator.target);
                    best_diff = target_diff;
                }

                false => generator.target.clear(),
            }
        }

        best_text
    }

    fn gen_struct(&mut self, rng: &mut impl Rng, depth: usize, as_object: bool) {
        let max = match as_object {
            true => {
                self.target.push('{');
                self.config.object_max_branching
            }
            false => {
                self.target.push('[');
                self.config.array_max_branching
            }
        };

        let components = match depth >= self.config.tree_max_depth {
            true => 0,

            false => {
                let limit = 1.0 - (depth as f64) / (self.config.tree_max_depth as f64);

                let min = ((max as f64 / 2.0) * limit) as usize;
                let max = (max as f64 * limit) as usize;

                rng.gen_range(min..=max)
            }
        };

        if components > 0 {
            for index in 1..=components {
                self.new_line(depth + 1);
                if as_object {
                    self.random_string(rng, false);
                    self.target.push_str(": ");
                }

                self.random_branch(rng, depth + 1);

                if index < components {
                    self.target.push(',');
                }
            }

            self.new_line(depth);
        }

        match as_object {
            true => self.target.push('}'),
            false => self.target.push(']'),
        };
    }

    fn new_line(&mut self, depth: usize) {
        self.target
            .push_str(&format!("\n{}", " ".repeat(self.config.indent * depth)));
    }

    fn random_branch(&mut self, rng: &mut impl Rng, depth: usize) {
        match self.grow_dist.sample(rng) {
            0 => self.gen_struct(rng, depth, true),
            1 => self.gen_struct(rng, depth, false),
            2 => self.random_string(rng, true),
            3 => self.random_num(rng),
            4 => self.target.push_str("true"),
            5 => self.target.push_str("false"),
            6 => self.target.push_str("null"),

            _ => panic!("Malformed leaf index."),
        }
    }

    fn random_string(&mut self, rng: &mut impl Rng, as_value: bool) {
        self.target.push('"');

        let words = match as_value {
            true => rng.gen_range(1..=self.config.value_string_max_words),
            false => rng.gen_range(1..=self.config.key_string_max_words),
        };

        for index in 0..words {
            match as_value {
                true => {
                    if index > 0 {
                        self.target.push(' ');
                    }

                    let word = self.value_names.generate(rng, 1, "").unwrap();

                    self.target.push_str(&word);
                }

                false => {
                    if index > 0 {
                        self.target.push('-');
                    }

                    let word = self.key_names.generate(rng, 1, "").unwrap();

                    self.target.push_str(&word);
                }
            }
        }

        self.target.push('"');
    }

    fn random_num(&mut self, rng: &mut impl Rng) {
        match rng.gen_bool(self.config.floats_probability) {
            true => {
                let head = rng.gen_range(0..=0xFFFFu32).to_string();
                let tail = rng.gen_range(0..=0xFFFFu32).to_string();

                self.target.push_str(&head);
                self.target.push('.');
                self.target.push_str(&tail);
            }

            false => {
                let head = rng.gen_range(0..=0xFFFFFFFFu32).to_string();

                self.target.push_str(&head);
            }
        }
    }
}

#[derive(Debug)]
enum Action {
    Grow,
    Shrink,
    Rewrite,
}

impl Action {
    fn is_grow(&self) -> bool {
        match self {
            Self::Grow => true,
            _ => false,
        }
    }
}
