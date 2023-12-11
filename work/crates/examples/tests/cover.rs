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
    lexis::{SourceCode, TokenRef},
    syntax::{Node, NodeRef, ParseError, PolyRef, SyntaxTree},
    units::{CompilationUnit, Document},
};
use lady_deirdre_examples::json::lexis::JsonToken;

#[derive(Node, Clone)]
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
        #[child]
        object: NodeRef,
    },

    #[rule(start: $BraceOpen & (entries: Entry)*{$Comma} & end: $BraceClose)]
    #[recovery(
        [$BraceOpen..$BraceClose],
        [$BracketOpen..$BracketClose],
    )]
    Object {
        #[child]
        start: TokenRef,
        #[child]
        entries: Vec<NodeRef>,
        #[child]
        end: TokenRef,
    },

    #[rule(key: $String & $Colon & value: ANY)]
    #[secondary]
    Entry {
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
        #[child]
        start: TokenRef,
        #[child]
        items: Vec<NodeRef>,
        #[child]
        end: TokenRef,
    },

    #[rule(value: $String)]
    #[secondary]
    String {
        #[child]
        value: TokenRef,
    },

    #[rule(value: $Number)]
    #[secondary]
    Number {
        #[child]
        value: TokenRef,
    },

    #[rule(lit: $True)]
    #[secondary]
    True {
        #[child]
        lit: TokenRef,
    },

    #[rule(lit: $False)]
    #[secondary]
    False {
        #[child]
        lit: TokenRef,
    },

    #[rule(lit: $Null)]
    #[secondary]
    Null {
        #[child]
        lit: TokenRef,
    },
}

#[test]
fn test_clusters_traverse() {
    let mut doc = Document::<DebugNode>::default();

    assert!(doc.root_node_ref().span(&doc).is_none());

    doc.write(
        ..,
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
    );

    assert_eq!(0..70, doc.root_node_ref().span(&doc).unwrap());

    let mut cluster = doc.root_cluster_ref();

    cluster = cluster.next(&doc);
    assert_eq!(8..58, cluster.primary_node_ref().span(&doc).unwrap());

    cluster = cluster.next(&doc);
    assert_eq!(34..57, cluster.primary_node_ref().span(&doc).unwrap());

    cluster = cluster.next(&doc);
    assert_eq!(67..69, cluster.primary_node_ref().span(&doc).unwrap());

    assert!(!cluster.next(&doc).is_valid_ref(&doc));

    cluster = cluster.previous(&doc);
    assert_eq!(34..57, cluster.primary_node_ref().span(&doc).unwrap());

    cluster = cluster.previous(&doc);
    assert_eq!(8..58, cluster.primary_node_ref().span(&doc).unwrap());

    cluster = cluster.previous(&doc);
    assert_eq!(0..70, cluster.primary_node_ref().span(&doc).unwrap());

    assert!(!cluster.previous(&doc).is_valid_ref(&doc));
}

#[test]
fn test_nodes_cover() {
    let doc = Document::<DebugNode>::from(
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
    );

    assert_eq!(
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
        doc.substring(doc.cover(..).span(&doc).unwrap())
    );

    assert_eq!(
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
        doc.substring(doc.cover(0..0).span(&doc).unwrap())
    );

    assert_eq!(
        r#""foo": [1, 3, true, false, null, {"a": "xyz", "b": null}]"#,
        doc.substring(doc.cover(1..1).span(&doc).unwrap())
    );

    assert_eq!(
        r#"[1, 3, true, false, null, {"a": "xyz", "b": null}]"#,
        doc.substring(doc.cover(8..8).span(&doc).unwrap())
    );

    assert_eq!(
        r#"true"#,
        doc.substring(doc.cover(15..15).span(&doc).unwrap())
    );

    assert_eq!(
        r#"[1, 3, true, false, null, {"a": "xyz", "b": null}]"#,
        doc.substring(doc.cover(14..14).span(&doc).unwrap())
    );
}
