<!------------------------------------------------------------------------------
  This file is part of "Lady Deirdre", a compiler front-end foundation
  technology.

  This work is proprietary software with source-available code.

  To copy, use, distribute, or contribute to this work, you must agree to
  the terms of the General License Agreement:

  https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md

  The agreement grants a Basic Commercial License, allowing you to use
  this work in non-commercial and limited commercial products with a total
  gross revenue cap. To remove this commercial limit for one of your
  products, you must acquire a Full Commercial License.

  If you contribute to the source code, documentation, or related materials,
  you must grant me an exclusive license to these contributions.
  Contributions are governed by the "Contributions" section of the General
  License Agreement.

  Copying the work in parts is strictly forbidden, except as permitted
  under the General License Agreement.

  If you do not or cannot agree to the terms of this Agreement,
  do not use this work.

  This work is provided "as is", without any warranties, express or implied,
  except where such disclaimers are legally invalid.

  Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).
  All rights reserved.
------------------------------------------------------------------------------->

# Node References

Instances of nodes in the syntax tree are owned by the syntax tree manager
(e.g., by the Document or ImmutableSyntaxTree).

Similar to the TokenRef reference used to access individual tokens in the source
code,
the [NodeRef](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/syntax/struct.NodeRef.html)
referential object is used to obtain access to instances of syntax tree nodes.

NodeRefs are cheap to copy and are lifetime-independent objects representing
globally unique composite numeric indices. However, their functions require
references to the syntax tree managers in order to dereference corresponding
nodes owned by the manager. It's important to note that a NodeRef could
potentially represent an invalid reference if the node was removed.

```rust,noplayground
use lady_deirdre::{
    syntax::{NodeRef, PolyRef, SyntaxTree},
    units::Document,
};

let doc = Document::<JsonNode>::new_immutable(r#"{
   "foo": true,
   "bar": [123, null]
}"#);

// Returns a referential object that points to the root of the syntax tree.
let root_ref: NodeRef = doc.root_node_ref();

// Documents always have a root.
assert!(root_ref.is_valid_ref(&doc));
assert!(!root_ref.is_nil());

let Some(JsonNode::Root {object,..}) = root_ref.deref(&doc) else {
    // Validity checked above.
    unreachable!();
};

// Nil NodeRefs are intentionally invalid references within any compilation unit.
assert!(!NodeRef::nil().is_valid_ref(&doc));
assert!(NodeRef::nil().is_nil());
assert!(NodeRef::nil().deref(&doc).is_none());
```

## Polymorphic References

Since both NodeRef and TokenRef can serve as types for the children of syntax
tree nodes, they both implement a generic
trait [PolyRef](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/syntax/trait.PolyRef.html)
that provides common functions for both.

For example,
[PolyRef::span](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/syntax/trait.PolyRef.html#tymethod.span)
returns the site span of the referred object's bounds.

The PolyRef trait is an object-safe trait, useful for handling tree children
without breaking the call chain. For instance, if you are confident that a
particular instance of a PolyRef type is a NodeRef, you can use
the [PolyRef::as_node_ref](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/syntax/trait.PolyRef.html#tymethod.as_node_ref)
function to cast the instance to a NodeRef; otherwise, it returns a nil NodeRef
without causing a panic if the instance is not a NodeRef.

```rust,noplayground
use lady_deirdre::{
    lexis::{Position, ToSpan},
    syntax::{PolyRef, SyntaxTree},
    units::Document,
};

let doc = Document::<JsonNode>::new_immutable(r#"{
   "foo": true,
   "bar": [123, null]
}"#);

let root_span = doc
    .root_node_ref()
    .as_node_ref() // We are confident that `root_node_ref` returns a NodeRef.
    .span(&doc)
    .unwrap() // We are confident that the root NodeRef is a valid reference.
    .to_position_span(&doc)
    .unwrap(); // Site span can be casted to a Position span.

assert_eq!(root_span, Position::new(1, 1)..Position::new(4, 2));
```

Finally, Lady Deirdre provides an owned version of the PolyRef trait, known
as [PolyVariant](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/syntax/enum.PolyVariant.html).
PolyVariant is a simple enum with NodeRef and TokenRef variants. You can convert
either of these referential objects into a PolyVariant using
the [PolyRef::as_variant](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/syntax/trait.PolyRef.html#tymethod.as_variant)
function whenever you need a generic owned referential object for the
compilation unit's content.

Note that PolyVariant itself also implements the PolyRef trait.
