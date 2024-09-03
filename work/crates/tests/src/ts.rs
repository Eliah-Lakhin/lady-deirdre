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

use std::{
    mem::take,
    time::{Duration, Instant},
};

use lady_deirdre::{
    lexis::{Position, PositionSpan, SiteSpan, SourceCode, ToSite},
    syntax::{Node, NodeRef, SyntaxTree},
    units::{Document, MutableUnit},
};
use lady_deirdre_examples::json_grammar::syntax::JsonNode;
use ropey::Rope;
use tree_sitter::{InputEdit, Parser, Point, Tree, TreeCursor};

pub struct TSParser {
    source: Rope,
    parser: Parser,
    tree: Option<Tree>,
}

impl TSParser {
    pub fn new() -> Self {
        let source = Rope::new();

        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_json::language()).unwrap();

        Self {
            source,
            parser,
            tree: None,
        }
    }

    pub fn parse(&mut self, text: &str) -> Duration {
        self.source = Rope::from_str(text);

        let start = Instant::now();
        self.tree = Some(self.parser.parse(text, None).unwrap());
        let time = start.elapsed();

        time
    }

    pub fn reparse(
        &mut self,
        site_span: SiteSpan,
        position_span: PositionSpan,
        new_end_position: Position,
        text: &str,
    ) -> Duration {
        let mut tree = take(&mut self.tree).unwrap();

        self.source.remove(site_span.clone());
        self.source.insert(site_span.start, text);

        let edit = InputEdit {
            start_byte: site_span.start,
            old_end_byte: site_span.end,
            new_end_byte: site_span.start + text.len(),
            start_position: ts_point(position_span.start),
            old_end_position: ts_point(position_span.end),
            new_end_position: ts_point(new_end_position),
        };

        let start = Instant::now();

        tree.edit(&edit);
        tree = self
            .parser
            .parse_with(
                &mut |byte, _pos| {
                    let (chunk, begin, _, _) = self.source.chunk_at_byte(byte);

                    &chunk[(byte - begin)..]
                },
                Some(&tree),
            )
            .unwrap();

        let time = start.elapsed();

        self.tree = Some(tree);

        time
    }

    pub fn check_length(&self) {
        let tree = self.tree.as_ref().unwrap();

        let root_node = tree.root_node();
        let length = root_node.end_byte();

        assert_eq!(length, self.source.len_chars());
    }
}

impl TSParser {
    pub fn compare_trees(&self, doc: &Document<JsonNode>) {
        let tree = self.tree.as_ref().unwrap();

        let mut cursor = tree.walk();
        let ts_root = tree.root_node();
        let ld_root = doc.root_node_ref();

        assert_eq!(ts_root.end_byte(), doc.length());

        self.compare(&mut cursor, ts_root, doc, &ld_root);
    }

    fn compare<'a>(
        &self,
        cursor: &mut TreeCursor<'a>,
        ts_node: tree_sitter::Node<'a>,
        doc: &Document<JsonNode>,
        ld_node: &NodeRef,
    ) {
        let ld_node = ld_node.deref(doc).unwrap();

        match ld_node {
            JsonNode::Root { object, .. } => {
                assert_eq!(ts_node.kind(), "document");
                assert_eq!(ts_node.named_child_count(), 1);

                let ts_object = ts_node.child(0).unwrap();

                self.compare(cursor, ts_object, doc, object)
            }

            JsonNode::Object { entries, .. } => {
                assert_eq!(ts_node.kind(), "object");
                assert_eq!(ts_node.named_child_count(), entries.len());

                let children = ts_node
                    .children(cursor)
                    .filter(|node| node.is_named())
                    .collect::<Vec<_>>();

                for (ts_child, ld_child) in children.into_iter().zip(entries.iter()) {
                    self.compare(cursor, ts_child, doc, ld_child);
                }
            }

            JsonNode::Entry { key, value, .. } => {
                assert_eq!(ts_node.kind(), "pair");
                assert_eq!(ts_node.named_child_count(), 2);

                let ts_key = ts_node.named_child(0).unwrap();
                let ts_value = ts_node.named_child(1).unwrap();

                self.compare(cursor, ts_key, doc, key);
                self.compare(cursor, ts_value, doc, value);
            }

            JsonNode::Array { items, .. } => {
                assert_eq!(ts_node.kind(), "array");
                assert_eq!(ts_node.named_child_count(), items.len());

                let children = ts_node
                    .children(cursor)
                    .filter(|node| node.is_named())
                    .collect::<Vec<_>>();

                for (ts_child, ld_child) in children.into_iter().zip(items.iter()) {
                    self.compare(cursor, ts_child, doc, ld_child);
                }
            }

            JsonNode::String { .. } => {
                assert_eq!(ts_node.kind(), "string");
            }

            JsonNode::Number { .. } => {
                assert_eq!(ts_node.kind(), "number");
            }

            JsonNode::True { .. } => {
                assert_eq!(ts_node.kind(), "true");
            }

            JsonNode::False { .. } => {
                assert_eq!(ts_node.kind(), "false");
            }

            JsonNode::Null { .. } => {
                assert_eq!(ts_node.kind(), "null");
            }
        }
    }
}

fn ts_point(position: Position) -> Point {
    Point {
        row: position.line - 1,
        column: position.column - 1,
    }
}
