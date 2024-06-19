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

# Grammar Setup

The central component of your compiler is
the [Analyzer](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Analyzer.html)
object. This object is responsible to manage the set of documents within the
compilation project and their semantic graph. Further details regarding the
Analyzer's API will be discussed in subsequent chapters. For now, our focus in
this chapter will be on configuring the programming language grammar.

The first generic parameter `N` of the Analyzer represents the type of the
language grammar. Essentially, this parameter denotes the type of the syntax
tree node. However, to fully describe the grammar, including semantics, you need
to extend this enum type with additional metadata:

1. Annotate enum variants that serve as the top nodes of the scopes with
   the `#[scope]` macro attribute.
2. Add a semantics field to each parsable (and denoted) enum variant, annotated
   with `#[semantics]`.
3. Optionally, you can specify the syntax tree classifier using
   the `#[classifier]` macro attribute.

From
the [Chain Analysis](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples/src/chain_analysis)
example:

```rust,noplayground
#[derive(Node)]
#[token(ChainToken)]
#[trivia($Whitespace)]
#[classifier(ChainNodeClassifier)] // Nodes classifier (this attribute is optional).
pub enum ChainNode {
    #[root]
    #[rule(block: Block)]
    Root {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        block: NodeRef,
        
        // Fields annotated with this macro attribute must be present in each
        // variant body, and they must be of type `Semantics`.
        #[semantics] 
        semantics: Semantics<VoidFeature<ChainNode>>,
    },

    #[rule($BraceOpen statements: (Block | Assignment)* $BraceClose)]
    #[scope] // This node is the top node of the scope.
    Block {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        statements: Vec<NodeRef>,
        #[semantics]
        semantics: Semantics<BlockSemantics>,
    },

    #[rule(key: Key $Assign value: (Ref | Num) $Semicolon)]
    Assignment {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        key: NodeRef,
        #[child]
        value: NodeRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ChainNode>>,
    },
    
    // ...
}
```

## Semantics Field

Each variant in the Node enum must contain a semantics field annotated with
the `#[semantics]` attribute and of
type [Semantics](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Semantics.html).

This field will be automatically initialized[^handwritten] and managed by the
macro-generated code.

Through this field, you can access semantic graph attributes that describe the
semantics specific to each node.

The Semantic object is parameterized by a user-defined type, typically a struct
type, enumerating all semantic attributes logically associated with the node. In
the example above, the Semantics of the `ChainNode::Block` variant is
parameterized by the `BlockSemantics` type.

If a node variant doesn't have any attributes, you can parameterize its
Semantics object with
the [VoidFeature](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.VoidFeature.html)
type, as seen in the `Root` and `Assignment` node variants.

[^handwritten]: To initialize this field manually in the hand-written parser,
use
the [Semantics::new](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Semantics.html#method.new)
function, passing the current NodeRef obtained from
the [SyntaxSession::node_ref](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/trait.SyntaxSession.html#tymethod.node_ref)
function.

## Feature Objects

The type you use as a parameter of the Semantics object is called a *feature*.

Typically, the semantic feature is a user-defined struct type derived from the
Feature trait using
the [Feature derive macro](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/derive.Feature.html).
This structure consists of fields that are either attributes or other feature
objects.

```rust,noplayground
#[derive(Feature)]
#[node(ChainNode)] // Required by the macro trait.
pub struct BlockSemantics {
    #[scoped]
    pub analysis: Attr<BlockAnalysis>,
    pub assignments: Attr<Shared<BlockAssignmentMap>>,
    pub blocks: Attr<Shared<BlockNamespaceMap>>,
    pub namespace: Attr<Shared<BlockNamespace>>,
}
```

In the above code, all fields are semantic
attributes ([Attr](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Attr.html)
types), but you are free to use other features as field types whenever you want
to create more complex nested structures. You can also reuse the same feature
type and attribute types in multiple places, as long as the feature or attribute
logically belongs to different syntax tree nodes. The Analyzer will treat them
as independent instances.

Additionally, in the above code, we annotated the `analysis` field
as `#[scoped]`. This annotation informs the Analyzer that this specific
attribute (or feature) is an entry point of the semantic model, performing the
initial inspection and mapping of the syntax tree's scoped branch to the
semantic model's initial objects.

Features with scoped attributes should be used as semantic objects of scoped
nodes (`BlockSemantics` is the semantics of the `ChainNode::Block`, which is
a `#[scope]`).

## Attributes

We will discuss attributes in more detail in the next chapters, but to give you
a brief overview, the generic parameter
of [Attr](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Attr.html)
specifies the type of the attribute value. This value is part of the semantic
model and can be any user-defined type (e.g., a struct or an enum) equipped with
a function that computes this value based on the syntax tree values and other
attribute values.

```rust,noplayground
#[derive(Default, Clone, PartialEq, Eq)]
pub struct BlockAnalysis {
    pub assignments: Shared<BlockAssignmentMap>,
    pub blocks: Shared<BlockNamespaceMap>,
}

impl Computable for BlockAnalysis {
    type Node = ChainNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        // Computing the BlockAnalysis instances based on the inputs provided
        // by the `context`.
    }
}
```

The general requirements imposed on this type are that it should implement the
Clone, Eq,
and [Computable](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/trait.Computable.html)
traits.
