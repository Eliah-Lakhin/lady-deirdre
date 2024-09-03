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

# Semantic Graph

A semantic
attribute ([Attr](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Attr.html)
object) consists of a cache for a value of an arbitrary user-defined type and a
function that computes this value when invoked by the Analyzer's inner
algorithm.

Inside
the [Computable](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/trait.Computable.html)
function that computes the value, you can access other attribute values, the
syntax and lexical content of the compilation units, and other Analyzer-related
elements from the `context` argument of
the [AttrContext](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.AttrContext.html)
type. This argument is the source of the inputs to the function, allowing you to
infer and return the resulting attribute value.

The implementation of the function should be deterministic and generally free of
side effects. Typically, it should compute the output value solely based on the
inputs.

## Attribute Value

What you compute inside the function depends on your semantics design. It could
be the type of a variable introduced in the source code, or the occurrences of a
particular identifier throughout the source code. Essentially, it encompasses
anything needed to express the programming language's semantic rules and to
enhance the language server, thereby assisting the end user in the code
editor[^compiler].

Typically, an attribute computes a value that logically belongs to the syntax
tree node on which it is instantiated. From the `context` argument, you can
access the NodeRef that points to the node owning this attribute. Using this
NodeRef reference, you can determine the document (by the document's id) that
contains this node and read the corresponding node instance.

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
        // A NodeRef that points to the syntax tree node owning this
        // attribute (assumed to be a `ChainNode::Block` enum variant in this case).
        let block_ref = context.node_ref();
        
        // Requests the Document object that stores the corresponding
        // compilation unit and its syntax tree in particular.
        let doc_read = context.read_doc(block_ref.id).unwrap_abnormal()?;
        let doc: &Document<ChainNode> = doc_read.deref();

        // Dereferences the instance of the syntax tree node to iterate through
        // the block statements.
        let Some(ChainNode::Block { statements, .. }) = block_ref.deref(doc) else {
            // If we encounter that the syntax tree is broken for any reason,
            // we return the (possibly unfinished) state of the computable
            // value regardless.
            //
            // The computable functions strive to infer as much metadata
            // as possible without panicking.
            return Ok(Self::default());
        };
        
        // Traversing through the block statements.
        for st_ref in statements {
            match st_ref.deref(doc) {
                // ...
            }
        }
        
        // ...
    }
}
```

Similarly to the syntax analysis stage, semantic analysis should be resilient to
errors. If the computable function cannot fully infer the target value, it
attempts to compute as much metadata as possible or fallback to reasonable
defaults without causing a panic. For this reason, most semantic model objects
in the [Chain Analysis](https://github.com/Eliah-Lakhin/lady-deirdre/blob/f350aaed30373a67694c3aba4d2cfd9874c2a656/work/crates/examples/src/chain_analysis/semantics.rs#L147)
example implement the Default trait.

For instance, in Rust source code, when introducing a variable with `let x;`,
the variable's type depends on the initialization expression. In the
type-inference attribute's computable function, we attempt to infer the Rust
type of the variable based on known initialization points. If we cannot fully
infer the type, we may infer it to a reasonable possibility or possibilities.

[^compiler]: Attributes are general-purpose; you can store any arbitrary data
inside them, not necessarily related to language semantics only. For example,
when implementing a programming language compiler, you can store middle-end or
even back-end artifacts of the compiler in some attributes. In this sense, Lady
Deirdre's semantic analysis framework could serve as an entry point to the
middle- or back-end compiler, even though these compilation stages are not the
direct purpose of Lady Deirdre.

## The Graph

Inside the computable function of the attribute, you can read other attribute
values. This mechanism allows you to infer more specific semantic facts from
more general facts.

For instance, in the Chain Analysis example,
the [LocalResolution](https://github.com/Eliah-Lakhin/lady-deirdre/blob/f350aaed30373a67694c3aba4d2cfd9874c2a656/work/crates/examples/src/chain_analysis/semantics.rs#L155)
attribute infers let-statement references within the local block in which it was
declared based on all local assignments
([BlockAssignmentMap](https://github.com/Eliah-Lakhin/lady-deirdre/blob/f350aaed30373a67694c3aba4d2cfd9874c2a656/work/crates/examples/src/chain_analysis/semantics.rs#L306) attribute)
within this block.

```rust,noplayground
#[derive(Default, Clone, PartialEq, Eq)]
pub enum LocalResolution {
    #[default]
    Broken,
    Resolved(usize),
    External(String),
}

impl SharedComputable for LocalResolution {
    type Node = ChainNode;

    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        // The NodeRef reference points to the key `ChainNode::Key` enum variant.
        let key_ref = context.node_ref();

        // The Document that owns this node's syntax tree.
        let doc_read = context.read_doc(key_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        // Dereferencing the "Key" node to access its semantics.
        let Some(ChainNode::Key { semantics, .. }) = key_ref.deref(doc) else {
            return Ok(Shared::default());
        };

        // Fetches the local `ChainNode::Block`'s NodeRef within the scope of
        // which the Key node resides.
        let block_ref = semantics
            .scope_attr()
            .unwrap_abnormal()?
            .read(context)?
            .scope_ref;

        // Dereferencing this block node instance.
        let Some(ChainNode::Block { semantics, .. }) = block_ref.deref(doc) else {
            return Ok(Shared::default());
        };

        // Accessing the block's semantics.
        let block_semantics = semantics.get().unwrap_abnormal()?;

        // Reading the `BlockAssignmentMap` attribute of the Block.
        let assignments = block_semantics
            .assignments
            .read(context)
            .unwrap_abnormal()?;

        // Looking up for an entry inside this map that belongs to the key.
        let Some(resolution) = assignments.as_ref().map.get(key_ref) else {
            return Ok(Shared::default());
        };

        //  Cloning the value from the entry, which will be the value of
        // the computable attribute.
        Ok(resolution.clone())
    }
}
```

In this snippet, particularly on the
line `block_semantics.assignments.read(context)`, we are reading the value of
another attribute.
The [Attr::read](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Attr.html#method.read)
function takes the current `context` reference and returns a RAII read-guard of
the attribute's value.

When we read an attribute inside another attribute, we're indicating to
the `context` that the value of the reader depends on the value of what's being
read.

The act of reading establishes dependency relations between the attributes, so
that the cached value of the reader is subject to recomputations whenever any of
its dependencies change.

The system of attributes and their dependencies forms a *Semantic Graph*.

You don't specify this graph upfront; Lady Deirdre reveals the structure of the
graph at runtime when it calls the computable function, which tells the Analyzer
how one specific attribute depends on another.

This graph is dynamically evolving and potentially subject to reconfiguration as
the computable function passes through different control flow paths. However,
Lady Deirdre imposes one important limitation: the
graph [should not have cycles](https://en.wikipedia.org/wiki/Directed_acyclic_graph).
In other words, a computable function of an attribute cannot read the value of
an attribute that directly or indirectly reads its own value.

In the example above, the *LocalResolution* attribute depends on the
*BlockAssignmentMap* attribute, which in turn depends on the *BlockAnalysis*
attribute, an entry-point attribute that does not read any other attributes.
Thus, this dependency chain is straightforward and does not have any cycles by
design.

Avoiding cyclic dependencies between attributes is a rule that you should
manually implement when designing the programming language semantics. Lady
Deirdre provides some tools to detect possible errors in the graph design, which
we will discuss in the next chapters.
