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
    lexis::{CodeContent, SimpleToken, ToSpan},
    syntax::{Node, NodeRef, SyntaxError, SyntaxTree},
    Document,
};

#[test]
fn test_balance() {
    static mut VERSION: usize = 0;

    #[derive(Node, Clone, Debug, PartialEq, Eq)]
    #[token(SimpleToken)]
    #[error(SyntaxError)]
    #[skip($Number | $Symbol | $Identifier | $String | $Char | $Whitespace | $Mismatch)]
    #[define(ANY = Parenthesis | Brackets | Braces)]
    enum DebugNode {
        #[root]
        #[rule(inner: ANY*)]
        Root {
            #[default(unsafe { VERSION })]
            version: usize,
            inner: Vec<NodeRef>,
        },

        #[rule($ParenOpen & inner: ANY* & $ParenClose)]
        #[synchronization]
        Parenthesis {
            #[default(unsafe { VERSION })]
            version: usize,
            inner: Vec<NodeRef>,
        },

        #[rule($BracketOpen & inner: ANY* & $BracketClose)]
        #[synchronization]
        Brackets {
            #[default(unsafe { VERSION })]
            version: usize,
            inner: Vec<NodeRef>,
        },

        #[rule($BraceOpen & inner: ANY* & $BraceClose)]
        #[synchronization]
        Braces {
            #[default(unsafe { VERSION })]
            version: usize,
            inner: Vec<NodeRef>,
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

                let errors = match document.get_cluster(&node_ref.cluster_ref) {
                    None => 0,

                    Some(cluster) => (&cluster.errors).into_iter().count(),
                };

                match node {
                    DebugNode::Root { version, inner } => {
                        format!(
                            "{}:{}<{}>",
                            version,
                            errors,
                            inner
                                .iter()
                                .map(|node_ref| traverse(document, node_ref))
                                .collect::<Vec<_>>()
                                .join("")
                        )
                    }

                    DebugNode::Parenthesis { version, inner } => {
                        format!(
                            "{}:{}({})",
                            version,
                            errors,
                            inner
                                .iter()
                                .map(|node_ref| traverse(document, node_ref))
                                .collect::<Vec<_>>()
                                .join("")
                        )
                    }

                    DebugNode::Brackets { version, inner } => {
                        format!(
                            "{}:{}[{}]",
                            version,
                            errors,
                            inner
                                .iter()
                                .map(|node_ref| traverse(document, node_ref))
                                .collect::<Vec<_>>()
                                .join("")
                        )
                    }

                    DebugNode::Braces { version, inner } => {
                        format!(
                            "{}:{}{{{}}}",
                            version,
                            errors,
                            inner
                                .iter()
                                .map(|node_ref| traverse(document, node_ref))
                                .collect::<Vec<_>>()
                                .join("")
                        )
                    }
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

    let mut document = Document::<DebugNode>::from("foo bar baz");

    assert_eq!(document.debug_print(), "0:0<>");
    assert_eq!(document.substring(..), "foo bar baz");
    assert_eq!(document.debug_errors(), "");

    unsafe { VERSION = 1 };

    document.write(0..0, "(");
    assert_eq!(document.debug_print(), "1:1<1:1()>");
    assert_eq!(document.substring(..), "(foo bar baz");
    assert_eq!(document.debug_errors(), "[1:13]: Parenthesis format mismatch. Expected Braces, Brackets, Parenthesis, or $ParenClose.");

    unsafe { VERSION = 2 };

    document.write(1..1, "[{");
    assert_eq!(document.debug_print(), "2:1<2:1(2:1[2:1{}])>");
    assert_eq!(document.substring(..), "([{foo bar baz");
    assert_eq!(
        document.debug_errors(),
        r#"[1:15]: Parenthesis format mismatch. Expected Braces, Brackets, Parenthesis, or $ParenClose.
[1:15]: Brackets format mismatch. Expected Braces, Brackets, Parenthesis, or $BracketClose.
[1:15]: Braces format mismatch. Expected Braces, Brackets, Parenthesis, or $BraceClose."#
    );

    unsafe { VERSION = 3 };

    document.write(6..6, ")");
    assert_eq!(document.debug_print(), "3:0<3:0(3:1[3:1{}])>");
    assert_eq!(document.substring(..), "([{foo) bar baz");
    assert_eq!(
        document.debug_errors(),
        r#"[1:7]: Brackets format mismatch. Expected Braces, Brackets, Parenthesis, or $BracketClose.
[1:7]: Braces format mismatch. Expected Braces, Brackets, Parenthesis, or $BraceClose."#
    );

    unsafe { VERSION = 4 };

    document.write(6..7, "");
    assert_eq!(document.debug_print(), "4:1<4:1(4:1[4:1{}])>");
    assert_eq!(document.substring(..), "([{foo bar baz");
    assert_eq!(
        document.debug_errors(),
        r#"[1:15]: Parenthesis format mismatch. Expected Braces, Brackets, Parenthesis, or $ParenClose.
[1:15]: Brackets format mismatch. Expected Braces, Brackets, Parenthesis, or $BracketClose.
[1:15]: Braces format mismatch. Expected Braces, Brackets, Parenthesis, or $BraceClose."#
    );

    unsafe { VERSION = 5 };
    document.write(6..6, "}()[]");
    unsafe { VERSION = 6 };
    document.write(15..15, "]");
    unsafe { VERSION = 7 };

    document.write(20..20, ")");
    assert_eq!(document.debug_print(), "7:0<7:0(6:0[5:0{}5:0()5:0[]])>");
    assert_eq!(document.substring(..), "([{foo}()[] bar] baz)");
    assert_eq!(document.debug_errors(), r#""#);

    unsafe { VERSION = 8 };

    document.write(7..8, "");
    assert_eq!(document.debug_print(), "7:0<7:0(8:1[8:0{}5:0[]])>");
    assert_eq!(document.substring(..), "([{foo})[] bar] baz)");
    assert_eq!(
        document.debug_errors(),
        r#"[1:8]: Brackets format mismatch. Expected Braces, Brackets, Parenthesis, or $BracketClose."#
    );

    unsafe { VERSION = 9 };

    document.write(12..12, "X");
    assert_eq!(document.debug_print(), "7:0<7:0(9:1[8:0{}5:0[]])>");
    assert_eq!(document.substring(..), "([{foo})[] bXar] baz)");
    assert_eq!(
        document.debug_errors(),
        r#"[1:8]: Brackets format mismatch. Expected Braces, Brackets, Parenthesis, or $BracketClose."#
    );

    unsafe { VERSION = 10 };

    document.write(2..2, "(");
    assert_eq!(document.debug_print(), "7:0<7:0(10:0[10:0(8:0{})5:0[]])>");
    assert_eq!(document.substring(..), "([({foo})[] bXar] baz)");
    assert_eq!(document.debug_errors(), r#""#);

    unsafe { VERSION = 11 };

    document.write(7..8, "");
    assert_eq!(document.debug_print(), "7:0<7:0(10:0[10:0(11:1{})5:0[]])>");
    assert_eq!(document.substring(..), "([({foo)[] bXar] baz)");
    assert_eq!(
        document.debug_errors(),
        r#"[1:8]: Braces format mismatch. Expected Braces, Brackets, Parenthesis, or $BraceClose."#
    );

    unsafe { VERSION = 12 };

    document.write(7..7, "}");
    assert_eq!(document.debug_print(), "7:0<7:0(10:0[10:0(12:0{})5:0[]])>");
    assert_eq!(document.substring(..), "([({foo})[] bXar] baz)");
    assert_eq!(document.debug_errors(), r#""#);
}
