use lady_deirdre::{lexis::ToSpan, syntax::SyntaxTree, Document};
use lady_deirdre_examples::json::syntax::JsonNode;

#[test]
fn test_cluster_span() {
    let mut doc = Document::<JsonNode>::default();

    assert_eq!(0..0, doc.root().cluster().span(&doc).to_span(&doc).unwrap());

    doc.write(
        ..,
        r#"{"foo": [1, 3, true, false, null, {"a": "xyz", "b": null}], "baz": {}}"#,
    );

    assert_eq!(
        0..70,
        doc.root().cluster().span(&doc).to_span(&doc).unwrap()
    );
}
