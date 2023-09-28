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

//TODO check warnings regularly
#![allow(warnings)]

use lady_deirdre::{
    lexis::{SourceCode, TokenBuffer, TokenRef},
    syntax::{Child, Node, NodeRef, ParseError, PolyRef, SyntaxTree},
    Document,
};
use lady_deirdre_examples::json::{formatter::ToJsonString, lexis::JsonToken, syntax::JsonNode};

#[test]
fn test_json_success() {
    static SNIPPET: &'static str =
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#;

    let code = TokenBuffer::<JsonToken>::from(SNIPPET);

    assert_eq!(SNIPPET, code.to_json_string());

    let doc = code.into_immutable_unit::<JsonNode>();

    assert!(doc.errors().collect::<Vec<_>>().is_empty());
}

#[test]
fn test_json_errors_1() {
    let code = TokenBuffer::<JsonToken>::from(
        r#"{FOO "foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
    );

    assert_eq!(
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
        code.to_json_string()
    );

    let doc = code.into_immutable_unit::<JsonNode>();

    assert_eq!(
        "1:2 (3 chars): Unexpected input in Object.",
        doc.errors()
            .map(|error| error.display(&doc).to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_json_errors_2() {
    let code = TokenBuffer::<JsonToken>::from(
        r#"{"foo": [1, 3 true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
    );

    assert_eq!(
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
        code.to_json_string()
    );

    let doc = code.into_immutable_unit::<JsonNode>();

    assert_eq!(
        "1:14 (1 char): Missing ',' in Array.",
        doc.errors()
            .map(|error| error.display(&doc).to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_json_errors_3() {
    let code = TokenBuffer::<JsonToken>::from(
        r#"{"foo": [1, 3,, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
    );

    assert_eq!(
        r#"{"foo": [1, 3, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
        code.to_json_string()
    );

    let doc = code.into_immutable_unit::<JsonNode>();

    assert_eq!(
        "1:15 (1 char): Unexpected input in Array.",
        doc.errors()
            .map(|error| error.display(&doc).to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_json_errors_4() {
    let code = TokenBuffer::<JsonToken>::from(
        r#"{"foo": [1, 3, true, false, null, "a": "xyz", "b": null}], "baz": {}}"#,
    );

    assert_eq!(
        r#"{"foo": [1, 3, true, false, null, "a", "b"], "baz": {}}"#,
        code.to_json_string()
    );

    let doc = code.into_immutable_unit::<JsonNode>();

    assert_eq!(
        "1:38 (7 chars): Unexpected input in Array.\n\
        1:50 (7 chars): Unexpected input in Array.",
        doc.errors()
            .map(|error| error.display(&doc).to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_json_errors_5() {
    let code = TokenBuffer::<JsonToken>::from(r#"{"outer": [{"a": "xyz",] "b": null}, "baz"]}"#);

    assert_eq!(
        r#"{"outer": [{"a": "xyz", "b": null}, "baz"]}"#,
        code.to_json_string()
    );

    let doc = code.into_immutable_unit::<JsonNode>();

    assert_eq!(
        "1:24 (1 char): Unexpected input in Object.",
        doc.errors()
            .map(|error| error.display(&doc).to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_json_errors_6() {
    let code = TokenBuffer::<JsonToken>::from(r#"{"outer": [{"a": ], "b": null}, "baz"]}"#);

    assert_eq!(
        r#"{"outer": [{"a": ?, "b": null}, "baz"]}"#,
        code.to_json_string()
    );

    let doc = code.into_immutable_unit::<JsonNode>();

    assert_eq!(
        "1:17: Missing Array, False, Null, Number, Object, String, True in Entry.\n\
        1:18 (1 char): Unexpected input in Object.",
        doc.errors()
            .map(|error| error.display(&doc).to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_json_errors_7() {
    let code = TokenBuffer::<JsonToken>::from(
        r#"{"outer": [{"a": [, "b": null}, "baz"], "outer2", "outer3": 12345}"#,
    );

    assert_eq!(
        r#"{"outer": [{"a": ["b", "baz"], "outer2": 12345}]}"#,
        code.to_json_string()
    );

    let doc = code.into_immutable_unit::<JsonNode>();

    assert_eq!(
        "1:19 (1 char): Unexpected input in Array.\n\
        1:24 (7 chars): Unexpected input in Array.\n\
        1:49 (10 chars): Unexpected input in Entry.\n\
        1:67: Unexpected end of input in Array.",
        doc.errors()
            .map(|error| error.display(&doc).to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_json_incremental() {
    static mut VERSION: usize = 0;

    #[derive(Node, Clone, Debug)]
    #[token(JsonToken)]
    #[error(ParseError)]
    #[trivia($Whitespace)]
    #[define(ANY = Object | Array | True | False | String | Number | Null)]
    #[recovery(
        $BraceClose,
        $BracketClose,
        [$BraceOpen..$BraceClose],
        [$BracketOpen..$BracketClose],
    )]
    pub enum DebugNode {
        #[root]
        #[rule(object: Object)]
        Root {
            #[parent]
            parent_ref: NodeRef,
            #[node]
            node_ref: NodeRef,
            #[default(unsafe { VERSION })]
            version: usize,
            #[child]
            object: NodeRef,
        },

        #[rule(start: $BraceOpen & (entries: Entry)*{$Comma} & end: $BraceClose)]
        #[recovery(
            [$BraceOpen..$BraceClose],
            [$BracketOpen..$BracketClose],
        )]
        Object {
            #[parent]
            parent_ref: NodeRef,
            #[node]
            node_ref: NodeRef,
            #[default(unsafe { VERSION })]
            version: usize,
            #[child]
            start: TokenRef,
            #[child]
            entries: Vec<NodeRef>,
            #[child]
            end: TokenRef,
        },

        #[rule(key: $String & $Colon & value: ANY)]
        Entry {
            #[parent]
            parent_ref: NodeRef,
            #[node]
            node_ref: NodeRef,
            #[default(unsafe { VERSION })]
            version: usize,
            #[child]
            key: TokenRef,
            #[child]
            value: NodeRef,
        },

        #[rule(start: $BracketOpen & (items: ANY)*{$Comma} & end: $BracketClose)]
        #[recovery(
            [$BraceOpen..$BraceClose],
            [$BracketOpen..$BracketClose],
        )]
        Array {
            #[parent]
            parent_ref: NodeRef,
            #[node]
            node_ref: NodeRef,
            #[default(unsafe { VERSION })]
            version: usize,
            #[child]
            start: TokenRef,
            #[child]
            items: Vec<NodeRef>,
            #[child]
            end: TokenRef,
        },

        #[rule(value: $String)]
        String {
            #[parent]
            parent_ref: NodeRef,
            #[node]
            node_ref: NodeRef,
            #[default(unsafe { VERSION })]
            version: usize,
            #[child]
            value: TokenRef,
        },

        #[rule(value: $Number)]
        Number {
            #[parent]
            parent_ref: NodeRef,
            #[node]
            node_ref: NodeRef,
            #[default(unsafe { VERSION })]
            version: usize,
            #[child]
            value: TokenRef,
        },

        #[rule($True)]
        True {
            #[parent]
            parent_ref: NodeRef,
            #[node]
            node_ref: NodeRef,
            #[default(unsafe { VERSION })]
            version: usize,
        },

        #[rule($False)]
        False {
            #[parent]
            parent_ref: NodeRef,
            #[node]
            node_ref: NodeRef,
            #[default(unsafe { VERSION })]
            version: usize,
        },

        #[rule($Null)]
        Null {
            #[parent]
            parent_ref: NodeRef,
            #[node]
            node_ref: NodeRef,
            #[default(unsafe { VERSION })]
            version: usize,
        },
    }

    trait DebugPrint {
        fn debug_print(&self) -> String;

        fn debug_errors(&self) -> String;
    }

    impl DebugPrint for Document<DebugNode> {
        fn debug_print(&self) -> String {
            fn traverse(
                document: &Document<DebugNode>,
                node_ref: &NodeRef,
                parent_ref: &NodeRef,
            ) -> String {
                let node: &DebugNode = match node_ref.deref(document) {
                    None => return format!("?"),
                    Some(node) => node,
                };

                assert_eq!(&node.node_ref(), node_ref);

                assert_eq!(&node.parent_ref(), parent_ref);

                match node {
                    DebugNode::Root {
                        version, object, ..
                    } => {
                        assert_eq!(
                            node.children(),
                            vec![("object", Child::from(object))].into_iter().collect(),
                        );
                        format!("{}({})", version, traverse(document, object, node_ref))
                    }

                    DebugNode::Object {
                        version,
                        start,
                        entries,
                        end,
                        ..
                    } => {
                        assert_eq!(
                            node.children(),
                            vec![
                                ("start", Child::from(start)),
                                ("entries", Child::from(entries)),
                                ("end", Child::from(end)),
                            ]
                            .into_iter()
                            .collect(),
                        );
                        format!(
                            "{}({{{}}})",
                            version,
                            entries
                                .into_iter()
                                .map(|entry_ref| traverse(document, entry_ref, node_ref))
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                    }

                    DebugNode::Array {
                        version,
                        start,
                        items,
                        end,
                        ..
                    } => {
                        assert_eq!(
                            node.children(),
                            vec![
                                ("start", Child::from(start)),
                                ("items", Child::from(items)),
                                ("end", Child::from(end)),
                            ]
                            .into_iter()
                            .collect(),
                        );
                        format!(
                            "{}([{}])",
                            version,
                            items
                                .into_iter()
                                .map(|item_ref| traverse(document, item_ref, node_ref))
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                    }

                    DebugNode::Entry {
                        version,
                        key,
                        value,
                        ..
                    } => {
                        assert_eq!(
                            node.children(),
                            vec![("key", Child::from(key)), ("value", Child::from(value))]
                                .into_iter()
                                .collect(),
                        );
                        format!(
                            "{}({:#}: {})",
                            version,
                            key.string(document).unwrap_or("?"),
                            traverse(document, value, node_ref),
                        )
                    }

                    DebugNode::String { version, value, .. }
                    | DebugNode::Number { version, value, .. } => {
                        assert_eq!(
                            node.children(),
                            vec![("value", Child::from(value))].into_iter().collect(),
                        );
                        format!("{}({})", version, value.string(document).unwrap_or("?"))
                    }

                    DebugNode::True { version, .. } => format!("{}(true)", version),

                    DebugNode::False { version, .. } => format!("{}(false)", version),

                    DebugNode::Null { version, .. } => format!("{}(null)", version),
                }
            }

            traverse(self, &self.root_node_ref(), &NodeRef::nil())
        }

        fn debug_errors(&self) -> String {
            self.errors()
                .map(|error| error.display(self).to_string())
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    unsafe { VERSION = 0 }

    let mut doc = Document::<DebugNode>::from("");
    assert_eq!(doc.substring(..), r#""#);
    assert_eq!(doc.debug_errors(), "1:1: Missing Object.",);
    assert_eq!(doc.to_json_string(), r#"?"#);
    assert_eq!(doc.debug_print(), r#"0(?)"#);

    unsafe { VERSION = 1 }

    let node_ref = doc.write(0..0, "{");
    assert_eq!(doc.substring(node_ref.span(&doc).unwrap()), r#"{"#,);
    assert_eq!(doc.substring(..), r#"{"#);
    assert_eq!(
        doc.debug_errors(),
        "1:2: Unexpected end of input in Object.",
    );
    assert_eq!(doc.to_json_string(), r#"{}"#);
    assert_eq!(doc.debug_print(), r#"1(1({}))"#);

    unsafe { VERSION = 2 }

    let node_ref = doc.write(1..1, "}");
    assert_eq!(doc.substring(node_ref.span(&doc).unwrap()), r#"{}"#,);
    assert_eq!(doc.substring(..), r#"{}"#);
    assert_eq!(doc.to_json_string(), r#"{}"#);
    assert_eq!(doc.debug_print(), r#"2(2({}))"#);

    unsafe { VERSION = 3 }

    let node_ref = doc.write(1..1, r#""foo""#);
    assert_eq!(doc.substring(node_ref.span(&doc).unwrap()), r#"{"foo"}"#,);
    assert_eq!(doc.substring(..), r#"{"foo"}"#);
    assert_eq!(doc.debug_errors(), "1:7: Missing ':' in Entry.");
    assert_eq!(doc.to_json_string(), r#"{"foo": ?}"#);
    assert_eq!(doc.debug_print(), r#"3(3({3("foo": ?)}))"#);

    unsafe { VERSION = 4 }

    let node_ref = doc.write(
        6..6,
        r#"[1, 3, true, false, null, {"a": "xyz", "b": null}]"#,
    );
    assert_eq!(
        doc.substring(node_ref.span(&doc).unwrap()),
        r#""foo"[1, 3, true, false, null, {"a": "xyz", "b": null}]"#
    );
    assert_eq!(
        doc.substring(..),
        r#"{"foo"[1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(doc.debug_errors(), "1:7: Missing ':' in Entry.");
    assert_eq!(
        doc.to_json_string(),
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(
        doc.debug_print(),
        r#"3(3({4("foo": 4([4(1), 4(3), 4(true), 4(false), 4(null), 4({4("a": 4("xyz")), 4("b": 4(null))})]))}))"#
    );

    unsafe { VERSION = 5 }

    let node_ref = doc.write(6..6, r#" :"#);
    assert_eq!(
        doc.substring(node_ref.span(&doc).unwrap()),
        r#""foo" :[1, 3, true, false, null, {"a": "xyz", "b": null}]"#
    );
    assert_eq!(
        doc.substring(..),
        r#"{"foo" :[1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(doc.debug_errors(), "");
    assert_eq!(
        doc.to_json_string(),
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(
        doc.debug_print(),
        r#"3(3({5("foo": 4([4(1), 4(3), 4(true), 4(false), 4(null), 4({4("a": 4("xyz")), 4("b": 4(null))})]))}))"#
    );

    unsafe { VERSION = 6 }

    let node_ref = doc.write(6..8, r#": "#);
    assert_eq!(
        doc.substring(node_ref.span(&doc).unwrap()),
        r#""foo": [1, 3, true, false, null, {"a": "xyz", "b": null}]"#
    );
    assert_eq!(
        doc.substring(..),
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(
        doc.to_json_string(),
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(
        doc.debug_print(),
        r#"3(3({6("foo": 4([4(1), 4(3), 4(true), 4(false), 4(null), 4({4("a": 4("xyz")), 4("b": 4(null))})]))}))"#
    );

    unsafe { VERSION = 7 }

    let node_ref = doc.write(8..34, r#""#);
    assert_eq!(
        doc.substring(node_ref.span(&doc).unwrap()),
        r#"{"foo": {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(doc.substring(..), r#"{"foo": {"a": "xyz", "b": null}]}"#);
    assert_eq!(
        doc.debug_errors(),
        "1:32 (1 char): Unexpected input in Object."
    );
    assert_eq!(doc.to_json_string(), r#"{"foo": {"a": "xyz", "b": null}}"#);
    assert_eq!(
        doc.debug_print(),
        r#"7(7({7("foo": 4({4("a": 4("xyz")), 4("b": 4(null))}))}))"#
    );

    unsafe { VERSION = 8 }

    let node_ref = doc.write(31..32, r#""#);
    assert_eq!(
        doc.substring(node_ref.span(&doc).unwrap()),
        r#"{"foo": {"a": "xyz", "b": null}}"#
    );
    assert_eq!(doc.substring(..), r#"{"foo": {"a": "xyz", "b": null}}"#);
    assert_eq!(doc.debug_errors(), "");
    assert_eq!(doc.to_json_string(), r#"{"foo": {"a": "xyz", "b": null}}"#);
    assert_eq!(
        doc.debug_print(),
        r#"8(8({8("foo": 8({4("a": 4("xyz")), 4("b": 4(null))}))}))"#
    );

    unsafe { VERSION = 9 }

    let node_ref = doc.write(14..14, r#"111, "c": "#);
    assert_eq!(
        doc.substring(node_ref.span(&doc).unwrap()),
        r#"{"a": 111, "c": "xyz", "b": null}"#
    );
    assert_eq!(
        doc.substring(..),
        r#"{"foo": {"a": 111, "c": "xyz", "b": null}}"#
    );
    assert_eq!(doc.debug_errors(), "");
    assert_eq!(
        doc.to_json_string(),
        r#"{"foo": {"a": 111, "c": "xyz", "b": null}}"#
    );
    assert_eq!(
        doc.debug_print(),
        r#"8(8({8("foo": 9({9("a": 9(111)), 9("c": 4("xyz")), 4("b": 4(null))}))}))"#
    );
}
