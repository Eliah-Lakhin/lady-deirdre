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

# Scope Access

For any syntax tree node with semantics, you can obtain a NodeRef reference to
the top node of the scope in which this node is nested.

The [Semantics::scope_attr](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/analysis/struct.Semantics.html#method.scope_attr)
function returns a special built-in attribute that contains a NodeRef of the top
node within the node's scope. The Analyzer is responsible for maintaining the
accuracy of this attribute's value, and you can utilize it within any computable
functions.

For instance, in the Chain Analysis example,
the [LocalResolution](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/chain_analysis/semantics.rs#L172)
function accesses the scope block of the `ChainNode::Key` node by utilizing this
attribute.

```rust,noplayground
impl SharedComputable for LocalResolution {
    type Node = ChainNode;

    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        let key_ref = context.node_ref();
        let doc_read = context.read_doc(key_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let Some(ChainNode::Key { semantics, .. }) = key_ref.deref(doc) else {
            return Ok(Shared::default());
        };

        let block_ref = semantics // The semantics of the Key node.
            .scope_attr() // The scope attribute of the `Key` node.
            .unwrap_abnormal()?
            .read(context)? // Reading this attribute.
            .scope_ref; // The NodeRef of the `Block` into which this `Key` node is nested.

        let Some(ChainNode::Block { semantics, .. }) = block_ref.deref(doc) else {
            return Ok(Shared::default());
        };
        
        // ...
    }
}
```

Note that the top nodes themselves are considered to be nested within their
parent scopes. The `ChainNode::Block` node, which serves as a top node of a
scope, is nested within its parent, Block[^parent]. By iteratively climbing up,
you will eventually reach the root of the syntax tree.

The [GlobalResolution](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/chain_analysis/semantics.rs#L85)
attribute leverages this feature to calculate the ultimate resolution of
the `ChainNode::Key` value by ascending through the hierarchy of nested blocks.

```rust,noplayground
impl Computable for GlobalResolution {
    type Node = ChainNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        let key_ref = context.node_ref();
        let doc_read = context.read_doc(key_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let Some(ChainNode::Key { semantics, .. }) = key_ref.deref(doc) else {
            return Ok(Self::default());
        };

        let key_semantics = semantics.get().unwrap_abnormal()?;

        let local_resolution = key_semantics
            .local_resolution
            .read(context)
            .unwrap_abnormal()?;

        // Checks if the `Key` has already been resolved locally.

        let mut ref_name = match local_resolution.as_ref() {
            LocalResolution::Broken => return Ok(Self::Broken),
            LocalResolution::Resolved(num) => return Ok(Self::Resolved(*num)),
            LocalResolution::External(name) => String::from(name),
        };
        
        // Otherwise, it climbs up through the system of nested blocks.

        // Fetches the NodeRef of the `Key`'s block node in a similar manner to
        // the `LocalResolution` computable function.
        let mut block_ref = semantics
            .scope_attr()
            .unwrap_abnormal()?
            .read(context)
            .unwrap_abnormal()?
            .scope_ref;

        loop {
            // Checks if the current block has the resolution we are seeking.
        
            let Some(ChainNode::Block { semantics, .. }) = block_ref.deref(doc) else {
                return Ok(Self::default());
            };

            let block_semantics = semantics.get().unwrap_abnormal()?;

            let block_namespace = block_semantics.namespace.read(context).unwrap_abnormal()?;

            match block_namespace.as_ref().namespace.get(&ref_name) {
                Some(LocalResolution::Broken) => return Ok(Self::Broken),
                Some(LocalResolution::Resolved(num)) => return Ok(Self::Resolved(*num)),
                Some(LocalResolution::External(name)) => ref_name = String::from(name),
                None => (),
            }

            // Otherwise, sets the `block_ref` to the parent block of
            // the current block to continue the climbing-up iteration.

            block_ref = semantics
                .scope_attr()
                .unwrap_abnormal()?
                .read(context)
                .unwrap_abnormal()?
                .scope_ref;
        }
    }
}
```

[^parent]: Or within the root node of the syntax tree. The root node is treated
as the default scope for the entire syntax tree.
