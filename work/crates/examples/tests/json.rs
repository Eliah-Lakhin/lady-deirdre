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
    lexis::{CodeContent, SourceCode, ToSpan, TokenBuffer, TokenRef},
    syntax::{Node, NodeRef, SyntaxError, SyntaxTree},
    Document,
};
use lady_deirdre_examples::json::{formatter::JsonFormatter, lexis::JsonToken, syntax::JsonNode};

#[test]
fn test_json_success() {
    static SNIPPET: &'static str =
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#;

    let code = TokenBuffer::<JsonToken>::from(SNIPPET);

    assert_eq!(SNIPPET, code.transduce(JsonFormatter));

    let tree = JsonNode::parse(code.cursor(..));

    assert!(tree.errors().collect::<Vec<_>>().is_empty());
}

#[test]
fn test_json_errors_1() {
    let code = TokenBuffer::<JsonToken>::from(
        r#"{FOO "foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
    );

    assert_eq!(
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
        code.transduce(JsonFormatter)
    );

    let tree = JsonNode::parse(code.cursor(..));

    assert_eq!(
        "[1:2] - [1:4]: Object format mismatch. Expected Entry or $BraceClose.",
        tree.errors()
            .map(|error| format!("{}: {}", error.span().format(&code), error))
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
        code.transduce(JsonFormatter)
    );

    let tree = JsonNode::parse(code.cursor(..));

    assert_eq!(
        "[1:14]: Missing $Comma in Array.",
        tree.errors()
            .map(|error| format!("{}: {}", error.span().format(&code), error))
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
        code.transduce(JsonFormatter)
    );

    let tree = JsonNode::parse(code.cursor(..));

    assert_eq!(
        "[1:15]: Array format mismatch. Expected Array, False, Null, Number, Object, String, or True.",
        tree.errors()
            .map(|error| format!("{}: {}", error.span().format(&code), error))
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
        code.transduce(JsonFormatter)
    );

    let tree = JsonNode::parse(code.cursor(..));

    assert_eq!(
        "[1:38] - [1:44]: Array format mismatch. Expected $BracketClose or $Comma.\n\
        [1:50] - [1:56]: Array format mismatch. Expected $BracketClose or $Comma.",
        tree.errors()
            .map(|error| format!("{}: {}", error.span().format(&code), error))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_json_errors_5() {
    let code = TokenBuffer::<JsonToken>::from(r#"{"outer": [{"a": "xyz",] "b": null}, "baz"]}"#);

    assert_eq!(
        r#"{"outer": [{"a": "xyz", "b": null}, "baz"]}"#,
        code.transduce(JsonFormatter)
    );

    let tree = JsonNode::parse(code.cursor(..));

    assert_eq!(
        "[1:24]: Object format mismatch. Expected Entry.",
        tree.errors()
            .map(|error| format!("{}: {}", error.span().format(&code), error))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_json_errors_6() {
    let code = TokenBuffer::<JsonToken>::from(r#"{"outer": [{"a": ], "b": null}, "baz"]}"#);

    assert_eq!(
        r#"{"outer": [{"a": ?, "b": null}, "baz"]}"#,
        code.transduce(JsonFormatter)
    );

    let tree = JsonNode::parse(code.cursor(..));

    assert_eq!(
        "[1:18]: Entry format mismatch. Expected Array, False, Null, Number, Object, String, or True.\n\
        [1:18]: Object format mismatch. Expected $BraceClose or $Comma.",
        tree.errors()
            .map(|error| format!("{}: {}", error.span().format(&code), error))
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
        code.transduce(JsonFormatter)
    );

    let tree = JsonNode::parse(code.cursor(..));

    assert_eq!(
        "[1:19]: Array format mismatch. Expected Array, False, Null, Number, Object, String, True, or $BracketClose.\n\
        [1:24] - [1:30]: Array format mismatch. Expected $BracketClose or $Comma.\n\
        [1:49] - [1:58]: Entry format mismatch. Expected $Colon.\n\
        [1:67]: Array format mismatch. Expected $BracketClose or $Comma.\n[1:67]: Object format mismatch. Expected $BraceClose or $Comma.",
        tree.errors()
            .map(|error| format!("{}: {}", error.span().format(&code), error))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_json_incremental() {
    static mut VERSION: usize = 0;

    #[derive(Node, Clone)]
    #[token(JsonToken)]
    #[error(SyntaxError)]
    #[skip($Whitespace)]
    #[define(ANY = Object | Array | True | False | String | Number | Null)]
    pub enum DebugNode {
        #[root]
        #[rule(object: Object)]
        Root {
            #[default(unsafe { VERSION })]
            version: usize,
            object: NodeRef,
        },

        #[rule($BraceOpen & (entries: Entry)*{$Comma} & $BraceClose)]
        #[synchronization]
        Object {
            #[default(unsafe { VERSION })]
            version: usize,
            entries: Vec<NodeRef>,
        },

        #[rule(key: $String & $Colon & value: ANY)]
        Entry {
            #[default(unsafe { VERSION })]
            version: usize,
            key: TokenRef,
            value: NodeRef,
        },

        #[rule($BracketOpen & (items: ANY)*{$Comma} & $BracketClose)]
        #[synchronization]
        Array {
            #[default(unsafe { VERSION })]
            version: usize,
            items: Vec<NodeRef>,
        },

        #[rule(value: $String)]
        String {
            #[default(unsafe { VERSION })]
            version: usize,
            value: TokenRef,
        },

        #[rule(value: $Number)]
        Number {
            #[default(unsafe { VERSION })]
            version: usize,
            value: TokenRef,
        },

        #[rule($True)]
        True {
            #[default(unsafe { VERSION })]
            version: usize,
        },

        #[rule($False)]
        False {
            #[default(unsafe { VERSION })]
            version: usize,
        },

        #[rule($Null)]
        Null {
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
            fn traverse(document: &Document<DebugNode>, node_ref: &NodeRef) -> String {
                let node = match node_ref.deref(document) {
                    None => return format!("?"),
                    Some(node) => node,
                };

                match node {
                    DebugNode::Root { version, object } => {
                        format!("{}({})", version, traverse(document, object))
                    }

                    DebugNode::Object { version, entries } => {
                        format!(
                            "{}({{{}}})",
                            version,
                            entries
                                .into_iter()
                                .map(|node_ref| traverse(document, node_ref))
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                    }

                    DebugNode::Array { version, items } => {
                        format!(
                            "{}([{}])",
                            version,
                            items
                                .into_iter()
                                .map(|node_ref| traverse(document, node_ref))
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                    }

                    DebugNode::Entry {
                        version,
                        key,
                        value,
                    } => {
                        format!(
                            "{}({:#}: {})",
                            version,
                            key.string(document).unwrap_or("?"),
                            traverse(document, value),
                        )
                    }

                    DebugNode::String { version, value } | DebugNode::Number { version, value } => {
                        format!("{}({})", version, value.string(document).unwrap_or("?"))
                    }

                    DebugNode::True { version } => format!("{}(true)", version),

                    DebugNode::False { version } => format!("{}(false)", version),

                    DebugNode::Null { version } => format!("{}(null)", version),
                }
            }

            traverse(self, self.root())
        }

        fn debug_errors(&self) -> String {
            self.errors()
                .map(|error| format!("{}: {}", error.span().format(self), error))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    unsafe { VERSION = 0 }

    let mut document = Document::<DebugNode>::from("");
    assert_eq!(document.substring(..), r#""#);
    assert_eq!(
        document.debug_errors(),
        "[1:1]: Root format mismatch. Expected Object.",
    );
    assert_eq!(document.transduce(JsonFormatter), r#"?"#);
    assert_eq!(document.debug_print(), r#"0(?)"#);

    unsafe { VERSION = 1 }

    document.write(0..0, "{");
    assert_eq!(document.substring(..), r#"{"#);
    assert_eq!(
        document.debug_errors(),
        "[1:2]: Object format mismatch. Expected Entry or $BraceClose.",
    );
    assert_eq!(document.transduce(JsonFormatter), r#"{}"#);
    assert_eq!(document.debug_print(), r#"1(1({}))"#);

    unsafe { VERSION = 2 }

    document.write(1..1, "}");
    assert_eq!(document.substring(..), r#"{}"#);
    assert_eq!(document.transduce(JsonFormatter), r#"{}"#);
    assert_eq!(document.debug_print(), r#"2(2({}))"#);

    unsafe { VERSION = 3 }

    document.write(1..1, r#""foo""#);
    assert_eq!(document.substring(..), r#"{"foo"}"#);
    assert_eq!(
        document.debug_errors(),
        "[1:7]: Entry format mismatch. Expected $Colon."
    );
    assert_eq!(document.transduce(JsonFormatter), r#"{"foo": ?}"#);
    assert_eq!(document.debug_print(), r#"3(3({3("foo": ?)}))"#);

    unsafe { VERSION = 4 }

    document.write(
        6..6,
        r#"[1, 3, true, false, null, {"a": "xyz", "b": null}]"#,
    );
    assert_eq!(
        document.substring(..),
        r#"{"foo"[1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(document.debug_errors(), "[1:7]: Missing $Colon in Entry.");
    assert_eq!(
        document.transduce(JsonFormatter),
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(
        document.debug_print(),
        r#"3(3({4("foo": 4([4(1), 4(3), 4(true), 4(false), 4(null), 4({4("a": 4("xyz")), 4("b": 4(null))})]))}))"#
    );

    unsafe { VERSION = 5 }

    document.write(6..6, r#" :"#);
    assert_eq!(
        document.substring(..),
        r#"{"foo" :[1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(document.debug_errors(), "");
    assert_eq!(
        document.transduce(JsonFormatter),
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(
        document.debug_print(),
        r#"3(3({5("foo": 4([4(1), 4(3), 4(true), 4(false), 4(null), 4({4("a": 4("xyz")), 4("b": 4(null))})]))}))"#
    );

    unsafe { VERSION = 6 }

    document.write(6..8, r#": "#);
    assert_eq!(
        document.substring(..),
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(
        document.transduce(JsonFormatter),
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(
        document.debug_print(),
        r#"3(3({6("foo": 4([4(1), 4(3), 4(true), 4(false), 4(null), 4({4("a": 4("xyz")), 4("b": 4(null))})]))}))"#
    );

    unsafe { VERSION = 7 }

    document.write(8..34, r#""#);
    assert_eq!(
        document.substring(..),
        r#"{"foo": {"a": "xyz", "b": null}]}"#
    );
    assert_eq!(
        document.debug_errors(),
        "[1:32]: Object format mismatch. Expected $BraceClose or $Comma."
    );
    assert_eq!(
        document.transduce(JsonFormatter),
        r#"{"foo": {"a": "xyz", "b": null}}"#
    );
    assert_eq!(
        document.debug_print(),
        r#"7(7({7("foo": 4({4("a": 4("xyz")), 4("b": 4(null))}))}))"#
    );

    unsafe { VERSION = 8 }

    document.write(31..32, r#""#);
    assert_eq!(
        document.substring(..),
        r#"{"foo": {"a": "xyz", "b": null}}"#
    );
    assert_eq!(document.debug_errors(), "");
    assert_eq!(
        document.transduce(JsonFormatter),
        r#"{"foo": {"a": "xyz", "b": null}}"#
    );
    assert_eq!(
        document.debug_print(),
        r#"8(8({8("foo": 8({4("a": 4("xyz")), 4("b": 4(null))}))}))"#
    );

    unsafe { VERSION = 9 }

    document.write(14..14, r#"111, "c": "#);
    assert_eq!(
        document.substring(..),
        r#"{"foo": {"a": 111, "c": "xyz", "b": null}}"#
    );
    assert_eq!(document.debug_errors(), "");
    assert_eq!(
        document.transduce(JsonFormatter),
        r#"{"foo": {"a": 111, "c": "xyz", "b": null}}"#
    );
    assert_eq!(
        document.debug_print(),
        r#"8(8({8("foo": 9({9("a": 9(111)), 9("c": 4("xyz")), 4("b": 4(null))}))}))"#
    );
}
