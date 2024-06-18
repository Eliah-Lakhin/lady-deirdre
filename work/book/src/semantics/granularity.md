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

# Granularity

As a general rule, it's preferable to maintain the semantic graph in a divided
manner, with small attributes that are easy to clone and compare for equality.
This ensures that each attribute value remains logically isolated from others.

With a granular semantic graph, the validation procedure is likely to complete
the propagation of incremental changes throughout the dependent attributes of
the graph more efficiently. This is achieved by comparing newly computed
attribute values with previous caches and stopping the propagation process if
they are found to be equal.

In the Chain Analysis example, the [BlockAnalysis](todo) input attribute
initially collects all assignment statements and inner blocks into two dedicated
maps: `assignments` and `blocks`.

```rust,noplayground
#[derive(Default, Clone, PartialEq, Eq)]
pub struct BlockAnalysis {
    pub assignments: Shared<BlockAssignmentMap>,
    pub blocks: Shared<BlockNamespaceMap>,
}
```

Later on, these maps are utilized in the [LocalResolution](todo)
and [GlobalResolution](todo) attributes. In theory, we could directly read the
*BlockAnalysis* attribute from these computable functions. However, in practice,
when the end user modifies the content of a block, it's likely that one of the
BlockAnalysis maps may remain unchanged. Therefore, depending solely on changes
in the overall BlockAnalysis attribute to read just one of the two maps is
probably unnecessary[^blockanalysis].

For these reasons, we spread both maps into the
intermediate [BlockAssignmentMap](todo) and [BlockNamespaceMap](todo) attributes
by cloning the hash maps into them. Subsequently, we read these maps in the
final attributes through these intermediaries independently.

If the *BlockAnalysis* attribute becomes invalid, both *BlockAssignmentMap* and
*BlockNamespaceMap* will be recomputed when the validation procedure refreshes
the semantic graph. However, it's possible that some of the *LocalResolution*
and *GlobalResolution* attributes will remain unaffected if the validator
detects that the intermediate attribute values haven't changed. As a result, the
entire validation procedure would proceed faster by skipping some of the heavy
computation steps.

[^blockanalysis]: This example might seem artificial, but in real-world
applications, it's probable that the input attributes for scope analysis would
comprise many more potentially independent fields.

## Shared Object

The [Shared](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/sync/struct.Shared.html)
helper object is Lady Deirdre's reference-counting thread-safe container, akin
to Rust's standard Arc, with two notable distinctions:

1. Shared, unlike Arc, lacks a Weak counterpart. If a weak counterpart isn't
   required, Shared's computation and memory performance are slightly better
   than Arc's.
2. The
   [Shared::get_mut](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/sync/struct.Shared.html#method.get_mut)
   function accepts `&mut self`. This makes it more convenient to use when
   constructing Shared in place.

Shared was initially designed for Lady Deirdre's semantic analysis framework.
However, you are free to utilize it anywhere you don't require Arc's weak
counterpart as well.

```rust,noplayground
use lady_deirdre::sync::Shared;

let mut shared_a = Shared::new(100);

// You can mutate the inner data in place when Shared does not have any clones yet.
*shared_a.get_mut().unwrap() += 20;

// However, to read the inner data, you need to explicitly call `.as_ref()`.
assert_eq!(*shared_a.as_ref(), 120);

// Does not clone the inner allocated data; it only creates another smart
// pointer to this allocation, similar to `Arc::clone`.
let shared_b = shared_a.clone();

assert_eq!(*shared_b.as_ref(), 120);
```

## Shared Computable

The [SharedComputable](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/trait.SharedComputable.html)
is a specialized helper trait that automatically implements the Computable trait
on the type `Shared<T>` if *SharedComputable* is implemented on `T`.

The `SharedComputable::compute_shared` function is a mandatory computation
function through which you return `Shared<T>` instead of `T`.

This trait is especially handy for propagating the Shared value through
intermediate attributes. For instance, the [BlockAssignmentMap](todo) simply
clones a shared map from the [BlockAnalysis](todo) (which is cheap, as it
merely creates a new smart pointer to the same allocation).

```rust,noplayground
#[derive(Default, Clone, PartialEq, Eq)]
pub struct BlockAssignmentMap {
    pub map: HashMap<NodeRef, Shared<LocalResolution>>,
}

impl SharedComputable for BlockAssignmentMap {
    type Node = ChainNode;

    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        let block_ref = context.node_ref();
        let doc_read = context.read_doc(block_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let Some(ChainNode::Block { semantics, .. }) = block_ref.deref(doc) else {
            return Ok(Shared::default());
        };

        let block_semantics = semantics.get().unwrap_abnormal()?;

        // Cloning the `BlockAnalysis::assignments` field as the result value of
        // this computable function.
        Ok(block_semantics.analysis.read(context)?.assignments.clone())
    }
}
```
