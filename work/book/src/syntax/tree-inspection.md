<!------------------------------------------------------------------------------
  This file is a part of the "Lady Deirdre" work,
  a compiler front-end foundation technology.

  This work is proprietary software with source-available code.

  To copy, use, distribute, and contribute to this work, you must agree to
  the terms of the General License Agreement:

  https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.

  The agreement grants you a Commercial-Limited License that gives you
  the right to use my work in non-commercial and limited commercial products
  with a total gross revenue cap. To remove this commercial limit for one of
  your products, you must acquire an Unrestricted Commercial License.

  If you contribute to the source code, documentation, or related materials
  of this work, you must assign these changes to me. Contributions are
  governed by the "Derivative Work" section of the General License
  Agreement.

  Copying the work in parts is strictly forbidden, except as permitted under
  the terms of the General License Agreement.

  If you do not or cannot agree to the terms of this Agreement,
  do not use this work.

  This work is provided "as is" without any warranties, express or implied,
  except to the extent that such disclaimers are held to be legally invalid.

  Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).
  All rights reserved.
------------------------------------------------------------------------------->

# Tree Inspection

## Query Nodes Manually

When you have a NodeRef reference, you can inspect the structure of the tree
locally around this node by directly dereferencing node instances.

```rust,noplayground
use lady_deirdre::{syntax::SyntaxTree, units::Document};

let doc = Document::<JsonNode>::new_immutable(r#"{
   "foo": true,
   "bar": [123, null]
}"#);

let root_ref = doc.root_node_ref();

let Some(JsonNode::Root { object, .. }) = root_ref.deref(&doc) else {
    panic!();
};

let Some(JsonNode::Object { entries, .. }) = object.deref(&doc) else {
    panic!();
};

let Some(JsonNode::Entry { value, .. }) = entries[1].deref(&doc) else {
    panic!();
};

let Some(JsonNode::Array { items, .. }) = value.deref(&doc) else {
    panic!();
};

let Some(JsonNode::Number { value, .. }) = items[0].deref(&doc) else {
    panic!();
};

let Some(string) = value.string(&doc) else {
    panic!();
};

assert_eq!(string, "123");
```

Alternatively, the above code could be rewritten in a more compact way using the
NodeRef's inspection functions without breaking the call chain.

```rust,noplayground
use lady_deirdre::{syntax::SyntaxTree, units::Document};

let doc = Document::<JsonNode>::new_immutable(r#"{
   "foo": true,
   "bar": [123, null]
}"#);

let string = doc
    .root_node_ref()
    .get_child(&doc, "object")
    .get_child(&doc, "entries") // returns the first entry
    .next_sibling(&doc) // gets the second entry
    .get_child(&doc, "value")
    .get_child(&doc, "items") // returns the first item
    .get_token(&doc, "value")
    .string(&doc)
    .unwrap();

assert_eq!(string, "123");
```

Each of these functions is infallible; they will return
a [nil](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/struct.NodeRef.html#method.nil)
NodeRef if they cannot fulfill the request. Therefore, we should be confident
about the node configuration we are trying to query.

## Depth-First Traversing

You can perform a depth-first traversal of the entire syntax tree or a specific
branch using
the [SyntaxTree::traverse_tree](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.SyntaxTree.html#method.traverse_tree)
and [SyntaxTree::traverse_subtree](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.SyntaxTree.html#method.traverse_subtree)
functions, respectively.

Both functions require a visitor object to be passed as an argument. This object
should implement
a [Visitor](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.Visitor.html)
trait, which includes functions that will be triggered when the traversal
procedure visits a node or token in the tree, according to the node-child
relationships between the nodes.

```rust,noplayground
use lady_deirdre::{
    lexis::TokenRef,
    syntax::{PolyRef, SyntaxTree, Visitor},
    units::Document,
};

let doc = Document::<JsonNode>::new_immutable( r#"{
    "foo": true,
    "bar": [123, null]
}"#);

doc.traverse_tree(&mut MyVisitor(&doc));

struct MyVisitor<'a>(&'a Document<JsonNode>);

impl<'a> Visitor for MyVisitor<'a> {
    fn visit_token(&mut self, token_ref: &TokenRef) {
        println!("Token\n{}", token_ref.display(self.0));
    }
    
    fn enter_node(&mut self, node_ref: &NodeRef) -> bool {
        println!("Enter\n{}", node_ref.display(self.0));
    
        // Tells the traverser to continue descending into this node's branch.
        true
    }
    
    fn leave_node(&mut self, node_ref: &NodeRef) {
        println!("Leave\n{}", node_ref.display(self.0));
    }
}
```

The visitor is a stateful object that you can mutate during tree traversal. You
can use this mechanism to collect common metadata from the syntax tree.

The *enter_node* function returns a boolean value that controls whether to
further descend into the entered node branch.

The *leave_node* function effectively visits the tree in reverse order.
