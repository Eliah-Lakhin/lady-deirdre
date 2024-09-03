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

use lady_deirdre::{
    format::{PrettyPrintConfig, PrettyPrinter},
    lexis::{SourceCode, TokenBuffer},
    syntax::{ParseNode, ParseNodeChild, ParseTree},
};

use crate::json_grammar::{lexis::JsonToken, syntax::JsonNode};

pub fn format_json(text: impl Into<TokenBuffer<JsonToken>>) -> String {
    let token_buffer = text.into();
    let parse_tree = ParseTree::<JsonNode, _>::new(&token_buffer, ..);
    let mut printer = PrettyPrinter::new(PrettyPrintConfig::new());

    format_json_node(&mut printer, &parse_tree, parse_tree.parse_tree_root());

    printer.finish()
}

fn format_json_node(
    printer: &mut PrettyPrinter,
    tree: &ParseTree<JsonNode, TokenBuffer<JsonToken>>,
    parse_node: &ParseNode,
) {
    if !parse_node.well_formed {
        let node_source_code = tree.substring(&parse_node.site_span);

        let mut first = true;

        for line in node_source_code.split("\n") {
            match first {
                true => first = false,
                false => printer.hardbreak(),
            }

            printer.word(line);
        }

        return;
    }

    for child in &parse_node.children {
        match child {
            ParseNodeChild::Blank(_) => (),

            ParseNodeChild::Token(child) => {
                let Some(token) = child.token_ref.deref(tree) else {
                    continue;
                };

                match token {
                    JsonToken::BraceOpen => {
                        printer.word("{");
                        printer.cbox(1);
                        printer.blank();
                    }

                    JsonToken::BraceClose => {
                        printer.blank();
                        printer.indent(-1);
                        printer.end();
                        printer.word("}")
                    }

                    JsonToken::BracketOpen => {
                        printer.word("[");
                        printer.ibox(1);
                        printer.softbreak();
                    }

                    JsonToken::BracketClose => {
                        printer.softbreak();
                        printer.indent(-1);
                        printer.end();
                        printer.word("]")
                    }

                    JsonToken::Comma => {
                        printer.word(",");
                        printer.blank();
                    }

                    JsonToken::Colon => {
                        printer.word(": ");
                    }

                    JsonToken::String
                    | JsonToken::Number
                    | JsonToken::True
                    | JsonToken::False
                    | JsonToken::Null => {
                        let Some(string) = child.token_ref.string(tree) else {
                            continue;
                        };

                        printer.word(string);
                    }

                    _ => (),
                }
            }

            ParseNodeChild::Node(child) => format_json_node(printer, tree, child),
        }
    }
}
