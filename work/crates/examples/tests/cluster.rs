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
    arena::Ref,
    lexis::{CodeContent, ToSpan},
    syntax::{SyntaxTree, TreeContent},
    Document,
};
use lady_deirdre_examples::json::syntax::JsonNode;

#[test]
fn test_clusters_traverse() {
    let mut doc = Document::<JsonNode>::default();

    assert_eq!(
        0..0,
        doc.root_node_ref()
            .cluster()
            .span(&doc)
            .to_span(&doc)
            .unwrap()
    );

    doc.write(
        ..,
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
    );

    assert_eq!(
        0..70,
        doc.root_node_ref()
            .cluster()
            .span(&doc)
            .to_span(&doc)
            .unwrap()
    );

    assert_eq!(
        0..70,
        doc.get_cluster_span(&Ref::Primary).to_span(&doc).unwrap()
    );

    let mut cluster = doc.root_node_ref().cluster();

    cluster = cluster.next(&doc);
    assert_eq!(8..58, cluster.span(&doc).to_span(&doc).unwrap());

    cluster = cluster.next(&doc);
    assert_eq!(34..57, cluster.span(&doc).to_span(&doc).unwrap());

    cluster = cluster.next(&doc);
    assert_eq!(67..69, cluster.span(&doc).to_span(&doc).unwrap());

    assert!(!cluster.next(&doc).is_valid_ref(&doc));

    cluster = cluster.previous(&doc);
    assert_eq!(34..57, cluster.span(&doc).to_span(&doc).unwrap());

    cluster = cluster.previous(&doc);
    assert_eq!(8..58, cluster.span(&doc).to_span(&doc).unwrap());

    cluster = cluster.previous(&doc);
    assert_eq!(0..70, cluster.span(&doc).to_span(&doc).unwrap());

    assert!(!cluster.previous(&doc).is_valid_ref(&doc));
}

#[test]
fn test_clusters_cover() {
    let doc = Document::<JsonNode>::from(
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
    );

    assert_eq!(
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
        doc.substring(doc.cover(..).span(&doc).to_span(&doc).unwrap())
    );

    assert_eq!(
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
        doc.substring(doc.cover(0..0).span(&doc).to_span(&doc).unwrap())
    );

    assert_eq!(
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
        doc.substring(doc.cover(1..1).span(&doc).to_span(&doc).unwrap())
    );

    assert_eq!(
        r#"[1, 3, true, false, null, {"a": "xyz", "b": null}]"#,
        doc.substring(doc.cover(9..9).span(&doc).to_span(&doc).unwrap())
    );

    assert_eq!(
        r#"[1, 3, true, false, null, {"a": "xyz", "b": null}]"#,
        doc.substring(doc.cover(15..15).span(&doc).to_span(&doc).unwrap())
    );

    assert_eq!(
        r#"[1, 3, true, false, null, {"a": "xyz", "b": null}]"#,
        doc.substring(doc.cover(16..16).span(&doc).to_span(&doc).unwrap())
    );
}
