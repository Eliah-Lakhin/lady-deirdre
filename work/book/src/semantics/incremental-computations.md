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

# Incremental Computations

Lady Deirdre does not compute attribute values instantly. Instead, the Analyzer
computes them or retrieves them from the cache whenever you explicitly read
them. Therefore, most of the time, some attribute values exist in a
not-yet-computed or outdated state.

However, the Analyzer is responsible for keeping the values of the graph up to
date whenever you observe corresponding attribute values.

The process of synchronizing the semantic graph is called *validation*.
Conversely, marking an attribute as subject for recomputation in the graph is
called *invalidation*.

The inner algorithm of the Analyzer is inspired by an approach similar to the
Rust compiler's query framework, also known
as [Salsa](https://github.com/salsa-rs/salsa). The algorithm attempts to avoid
unnecessary recomputations of semantic graph attributes whenever possible.
However, in general, it relies on the attribute's value equality (the Eq trait
implementation on the attribute value) to determine whether the value of an
attribute that depends on this one should be recomputed.

The validation procedure works as follows:

1. Whenever the end user edits the source code of the compilation unit, the
   Analyzer incrementally reparses the corresponding Document of this unit.
2. Then it detects all syntax tree nodes that have been altered during
   reparsing (nodes that have been updated, deleted, or created).
3. Next, the Analyzer collects all top scope nodes (the nodes annotated with
   the `#[scope]` macro attribute).
4. Then, the Analyzer marks the `#[scoped]` attributes of the scope nodes as
   *invalid* (subject to recomputation). At this point, the algorithm completes
   the "eager" stage of the validation. It does not make any further updates to
   the semantic graph values. This stage usually completes quickly.
5. When you request a particular attribute value (e.g., by traversing the syntax
   tree and fetching an attribute value from the node's semantics), the
   algorithm checks whether the direct or indirect dependencies of the requested
   attribute are invalid (or not yet computed). In such cases, the algorithm
   calls the computable functions on the *invalid* attributes, updating their
   caches and propagating the changes down to the requested attribute.

   This process may finish earlier if the algorithm determines that the
   recomputation process converges to the previously stored caches (based on
   equality between the cached values and the new results of the computable
   function).
6. Finally, the Analyzer returns an up-to-date clone of the attribute's value
   from its cache (hence, the value type should implement the Clone trait).

## Input Attributes

An important aspect of this algorithm is that the Analyzer automatically
invalidates only the `#[scoped]` attributes of the `#[scope]` syntax tree nodes
whenever the end user changes the content of the syntax tree within the scope.

Therefore, typically only these attributes should perform the initial mapping of
the scoped syntax tree structure to the initial semantic model objects.
Informally, you can think of these attributes as the *input attributes* of the
semantic graph.

Any other attributes should not directly rely on the current configuration of
the compilation unit state, such as the structure of children of nodes or the
strings covered by the scanned tokens. This metadata could change over time,
and, in general, will not be detected by the validator when it validates the
caches of these attributes. If this metadata is crucial to the attribute's
computable function implementation, it should be reflected in the initial
semantic model objects by the input attributes.

In the Chain Analysis example, only
the [BlockAnalysis](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/chain_analysis/semantics.rs#L202)
attribute (which is a `#[scoped]` attribute of the `#[scope]` node syntax tree
node) iterates through the block's inner let-statements and the inner blocks and
collects them into HashMaps usable for further analysis. Moreover, this
attribute does not inspect the inner structure of its nested blocks too, because
the sub-block's inner syntax structure is outside of the current block scope.

Other attributes directly (e.g.,
[BlockAssignmentMap](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/chain_analysis/semantics.rs#L310))
or indirectly (e.g.,
[LocalResolution](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/chain_analysis/semantics.rs#L155)
and [GlobalResolution](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/chain_analysis/semantics.rs#L85))
read the BlockAnalysis's HashMaps, but they do not perform deeper inspection of
the node's syntax tree structure inside their computable functions.
