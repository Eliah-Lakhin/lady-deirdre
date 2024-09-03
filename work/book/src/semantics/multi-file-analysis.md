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

# Multi-File Analysis

A compilation project usually consists of multiple compilation units that are
semantically connected to each other.

For example, a Java file may declare a class with signatures that reference
classes declared in other files within the same Java package.

To establish semantic relationships between these compilation units, you can
define a special analyzer-wide feature object.

From the [Shared Semantics](https://github.com/Eliah-Lakhin/lady-deirdre/tree/f350aaed30373a67694c3aba4d2cfd9874c2a656/work/crates/examples/src/shared_semantics) example:

```rust,noplayground
#[derive(Node)]

// Defines a semantic feature that is shared across all documents in the Analyzer.
#[semantics(CommonSemantics)]

pub enum SharedSemanticsNode {
    // ...
}

#[derive(Feature)]
#[node(SharedSemanticsNode)]
pub struct CommonSemantics {
    pub modules: Slot<SharedSemanticsNode, HashMap<String, Id>>,
}
```

## Common Semantics

The common semantics feature is a typical feature object, except that it is not
bound to any specific node within a compilation unit and is instantiated during
the creation of the Analyzer.

This feature is not tied to any syntax tree scope. Therefore, its members will
not be directly invalidated during the editing of the Analyzer's documents.

However, the members of this feature are part of the semantic graph and are
subject to the normal rules of the semantic graph, such as the prohibition of
cycles between computable functions.

Common semantic features typically include:

- Analyzer-wide reducing attributes, such as an attribute that collects all
  syntax and semantic issues detected across all managed documents.
- External configuration metadata specified via the system of
  [Slots](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Slot.html).
  For instance, a map between file names and their document IDs within the
  Analyzer (as in the example above).

You can access common semantics both inside and outside of computable
functions. Inside a computable function, you can access common semantics
using the [AttrContext::common](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.AttrContext.html#method.common)
method. To access the semantics outside, you would use the
[AbstractTask::common](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/trait.AbstractTask.html#method.common)
method.

```rust,noplayground
#[derive(Clone, PartialEq, Eq)]
pub enum KeyResolution {
    Unresolved,
    Recusrive,
    Number(usize),
}

impl Computable for KeyResolution {
    type Node = SharedSemanticsNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        // ...

        // Reading the common semantics inside the computable function.
        let modules = context.common().modules.read(context).unwrap_abnormal()?;
        
        // ...
    }
}

let handle = TriggerHandle::new();
let mut task = analyzer.mutate(&handle, 1).unwrap();

let doc_id = task.add_mutable_doc("x = 10; y = module_2::b; z = module_2::c;");
doc_id.set_name("module_1");

// Modifying the Slot value of the common semantics outside.
task.common()
    .modules
    .mutate(&task, |modules| {
        let _ = modules.insert(String::from("module_1"), doc_id);

        true
    })
    .unwrap();
```

## Slots

The primary purpose of a [Slot](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Slot.html)
is to provide a convenient mechanism for injecting configuration metadata
external to the Analyzer into the semantic graph. For instance, mapping between
file system names and the Analyzer's document IDs can be injected through a
common semantics Slot.

Slot is a special feature of the semantic graph that is quite similar to
attributes, except that a Slot does not have an associated computable function.
Instead, Slots have associated values of a specified type (the second generic
argument of the `Slot<Node, ValueType>` signature).

You can [snapshot](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Slot.html#method.snapshot)
the current Slot value outside of computable functions, and you can
[read](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Slot.html#method.read)
Slot values within the computable functions of attributes, thereby subscribing
those attributes to changes in the Slot, much like with normal attributes.

By default, Slot values are set to the `Default` of the value type. You can
modify the content of the Slot value using the
[Slot::mutate](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Slot.html#method.mutate)
method with a mutable (or exclusive) task.

```rust,noplayground
task.common()
    .modules
    // The `task` is a MutationTask or an ExclusiveTask.
    //
    // The provided callback accepts a mutable reference to the current
    // value of the Slot, and returns a boolean flag indicating whether the
    // value has changed.
    .mutate(&task, |modules| {
        let _ = modules.insert(String::from("module_1"), doc_id);

        // Indicates that the `modules` content has been changed.
        true
    })
    .unwrap();
```
