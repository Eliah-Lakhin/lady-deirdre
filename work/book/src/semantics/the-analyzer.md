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

# The Analyzer

To recap,
the [Analyzer](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Analyzer.html)
serves as the central object of the compiler, managing the compilation project's
set of documents and the semantic graph.

The state of this object is presumed to be shared among multiple
threads[^singlethread]. Specifically, numerous threads can edit various
documents concurrently without blocking (provided they edit distinct documents).
Additionally, multiple threads can query the semantic graph concurrently and
often without blocking (especially when the threads query independent
attributes). However, it's not possible to edit the documents and query their
attributes simultaneously. When querying an attribute, the graph undergoes
incremental recomputations that require synchronization of its state with
changes in documents. Therefore, the content of the documents should remain
fixed at the synchronization point.

For this reason, the API restricts access to the Analyzer's state: at any given
time, you either *mutate* the state of the Analyzer (e.g., apply edits to the
documents) or *analyze* the current state (e.g., query attribute values).

The Analyzer grants access to specific operations with its data through a system
of *task objects*. You can think of a "task" as an RAII guard, through which you
gain access to specific operations on the Analyzer's data[^tasks].

The Analyzer offers three types of task objects:

- The
  [AnalysisTask](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.AnalysisTask.html):
  This task allows you to query semantic graph attributes. You can have as many
  simultaneous task objects of this type as you need.
- The
  [MutationTask](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.MutationTask.html):
  With this task, you can create, edit, or remove documents, and you can trigger
  analyzer-wide events. Similar to AnalysisTask, you can have multiple
  simultaneous task objects of this type.
- The
  [ExclusiveTask](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.ExclusiveTask.html):
  This task enables you to sequentially perform analysis and mutation operations
  within a single thread. However, you cannot have more than one task of this
  type simultaneously.

You obtain the task objects by requesting them from the Analyzer. For instance,
the [Analyzer::analyze](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Analyzer.html#method.analyze)
function returns an *AnalysisTask* instance.

Each of these request functions could block the current thread if the Analyzer
cannot grant requested access instantly. For instance, if two threads request
analysis tasks, both of them will obtain access. However, if one thread requests
an analysis task and another thread requests a mutation task, one of the threads
will be blocked until the other releases the task object.

Informally, you can view the task system as a "RwLock" with complex
access-granting rules, and the task objects as "RAII guards".

[^singlethread]: Even though, it's perfectly acceptable to use it from a single
thread in a single-threaded process too.

[^tasks]: Don't be confused by the term "task". A Task Object simply grants
access to specific operations. While it's assumed that the task object would be
associated with a thread worker in the end application architecture, Lady
Deirdre doesn't manage threads itself, nor does it spawn any threads
specifically. Managing threads isn't the focus of the crate; you have the
freedom to organize the multithreaded (or single-threaded) architecture of your
program however you see fit.

## Mutation Task

The mutation task is utilized for creating, editing, or removing documents, as
well as triggering analyzer-wide events.

```rust,noplayground
let analyzer = Analyzer::<ChainNode>::new(AnalyzerConfig::default());

// A handle through which the task's thread could be gracefully interrupted.
// This interruption can be triggered either manually or by the Analyzer's inner
// task manager.
let handle = TriggerHandle::new();

// Requests the MutationTask.
let mut task = analyzer.mutate(&handle, 1).unwrap();

// Creates a new mutable document inside the Analyzer with the initial source
// code "test".
// The function returns an identifier for the created document.
let doc_id = task.add_mutable_doc("{ x: 10; }");

// Edits the document by its ID.
// This function may block if the document is currently being edited in another
// thread within another mutation task.
task.write_to_doc(doc_id, .., "{ y: 10; }").unwrap();

// Invalidates all attributes that have been subscribed to the event.
task.trigger_event(doc_id, 1234);

// Removes the document.
task.remove_doc(doc_id).unwrap();

// Ensures that the document no longer exists in the Analyzer.
assert!(!task.contains_doc(doc_id));
```

In the above code, the `add_mutable_doc` function
resembles [Document::new_mutable](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/units/enum.Document.html#method.new_mutable),
and the `write_to_doc` function
resembles [Document::write](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/units/enum.Document.html#method.write),
except that the Document instance is managed by the Analyzer.

## Analysis Task

With the analysis task, you can read attributes of the semantic graph, but you
cannot edit existing documents.

```rust,noplayground
// Requests the AnalysisTask.
let task = analyzer.analyze(&handle, 1).unwrap();

// Gets read-only access to the document by its id.
let doc_read = task.read_doc(doc_id).unwrap();
let doc = doc_read.deref();

// Searching for a `ChainNode::Key` node within the syntax tree.

let Some(ChainNode::Root { block, .. }) = doc.root_node_ref().deref(doc) else {
    panic!();
};

let Some(ChainNode::Block { statements, .. }) = block.deref(doc) else {
    panic!();
};

let Some(ChainNode::Assignment { key, .. }) = statements[0].deref(doc) else {
    panic!();
};

let Some(ChainNode::Key { semantics, .. }) = key.deref(doc) else {
    panic!();
};

let (attribute_version, resolution) = semantics
    .get()
    .unwrap()
    // The attribute of the node's semantic feature.
    .global_resolution
    // Returns a clone of the attribute's current value.
    .snapshot(&task)
    .unwrap();

assert_eq!(resolution, GlobalResolution::Resolved(100));
```

Note
the [snapshot](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Attr.html#method.snapshot)
function in the above code that we're calling on the `global_resolution`
attribute of the node's semantics.

This function executes the validation procedure and returns a pair of objects:
the Analyzer's inner version at which the value of the attribute was updated,
and a copy of the attribute's value.

The version number represents the inner version of the semantic graph state. The
Analyzer increments its version number each time it updates the values within
the semantic graph. This number always increases and never decreases.

The *snapshot* function returns the version at which the cache was updated. This
number is useful for quickly checking if the attribute has a new value by
comparing it with the version number received from this function previously.

The second object of the pair is a copy of the attribute's value. Unlike
the [Attr::read](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Attr.html#method.read)
function used within computable functions, which returns a reference to the
value, the *snapshot* function used externally copies the value (by cloning it).

For this reason, it's recommended to make the attribute's value type cheap
to copy if the attribute is intended to be observed from outside of computable
functions. Otherwise, you can wrap the value type
into [Shared](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/sync/struct.Shared.html).

## Exclusive Task

You obtain the exclusive task using
the [Analyzer::exclusive](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/struct.Analyzer.html#method.exclusive)
function.

The Analyzer grants only one instance of this type of task at a time, but this
task object provides both the analysis task and mutation task APIs.

The exclusive task is useful for both single-threaded and multi-threaded
compilers.

In some language servers and code editors, a technique used to implement
code-completion suggestions involves probing the source code by inserting a
special secret word at the position of the end user cursor. This allows
traversal of the tree to find the syntax tree node containing this word, thus
identifying the part of the syntax the user was attempting to complete. Finally,
the source code is restored by removing the inserted probing word.

All three steps — writing the probe word, analyzing the probed text, and
removing the probe word — should be done as a single transaction to ensure
atomicity. The exclusive task provides this atomicity, preventing other threads
from reading or changing the probed text in between.

## Documents Reading

From any kind of task, you can read the content of the document (both lexical
and syntactical).
The [read_doc](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/analysis/trait.AbstractTask.html#method.read_doc)
function returns a *DocumentReadGuard* RAII guard, through which you access the
Document object immutably. While this guard is held, attempts to mutate this
specific document (edit or remove) will be blocked. However, semantic analysis
(e.g., querying attributes) is not affected because analysis does not require
mutation of compilation units.

## Shared Analyzer

As the Analyzer is going to be a central object of the compiler, it's
recommended to either place it in a
[Shared](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/sync/struct.Shared.html)
or a
[Lazy](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/sync/struct.Lazy.html)
static for easy access from multiple threads. This is somewhat analogous to
placing a Mutex or RwLock with the program-wide state into an Arc to share it
across threads.

All methods of the Analyzer's API are `&self` functions.

```rust,noplayground
use lady_deirdre::{
    analysis::{Analyzer, AnalyzerConfig, TriggerHandle},
    sync::Lazy,
};

// This static will be initialized once you dereference it.
static MY_COMPILER: Lazy<Analyzer<ChainNode>> =
    Lazy::new(|| Analyzer::new(AnalyzerConfig::default()));

let handle = TriggerHandle::new();

let task = MY_COMPILER.mutate(&handle, 1).unwrap();
```

## Single Document Compiler

Sometimes, the compiler you're developing is intended to compile a programming
language without modules. For instance, vanilla JavaScript doesn't have modules;
the entire JavaScript compilation project consists of just one file (one
document).

In this case, you can configure the Analyzer when you instantiate it to manage
no more than a single document.

```rust,noplayground
use lady_deirdre::analysis::{Analyzer, AnalyzerConfig};

let mut config = AnalyzerConfig::default();

config.single_document = true;

let analyzer = Analyzer::<ChainNode>::new(config);
```

With this configuration option, you are turning off some inner memory and
performance overhead that the Analyzer consumes to handle more than one
document.

However, note that the single document Analyzer is still capable of managing
more than one document, but it is likely that multi-document management would be
less efficient. Therefore, you can use this configuration option to design the
compiler to usually manage a single compilation unit but not strictly limit it
to just one unit.

## Custom Hasher

The semantic analysis framework under the hood utilizes hash maps and hash sets
to store various kinds of inner metadata. By default, these maps and sets use
Rust's
standard [RandomState](https://doc.rust-lang.org/std/hash/struct.RandomState.html)
hasher, which prioritizes stability for specific kinds of cryptography attacks
relevant for network services. However, it is slower than other alternatives
without such guarantees.

Since compilers and language servers intended to run solely on local machines
usually don't require this level of security, the performance of the Analyzer
could be improved by replacing the standard hasher with a faster compatible
alternative from the Rust ecosystem, such
as [aHash](https://crates.io/crates/ahash).

To replace the hashing algorithm, you need to explicitly specify the third
generic parameter of the Analyzer with the hashing algorithm type of your
choice.
