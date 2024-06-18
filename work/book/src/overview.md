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

# Overview

## The Domain

The program you are developing could function both as a programming language
compiler and as a code editor extension simultaneously. Lady Deirdre does not
strongly distinguish between these domains, so we will collectively refer to the
developing program as the *Compiler*.

The input data for the Compiler is a *compilation project*: a set of
semantically interconnected source code text files. We refer to each individual
file within a project as a compilation unit. A compilation unit could be a real
file stored on disk or have a more abstract source (e.g., transferred to the
Compiler by the code editor through the LSP communication channel).

The primary purpose of the front-end part of the Compiler is to determine if the
compilation project is well-formed (i.e., if there are syntax or semantic errors
in the compilation units) and to infer the semantic connections between the
source code objects (e.g., all call sites of a function in the source code).

The compilation project is subject to frequent changes, as the end user may
modify the source code of the units with every keystroke. The Compiler should
keep its internal representation in sync with these changes in real time.

Moreover, the compilation project is often not well-formed. While the end user
is writing the source code, it is usually in an incomplete state, with syntax
and semantic errors. Therefore, the Compiler should be resilient to these
errors, able to continuously synchronize the program's abstract representation
with the current state of the source code without halting at the first
encountered error.

The Compiler's best effort is to infer as much metadata as possible from the
current state of the source code to assist the end user in the code editor:
highlighting references between identifiers, providing code completion
suggestions, and enabling semantically meaningful navigation between text
symbols.

## The Core Concepts

Lady Deirdre separates the processes of lexical scanning, syntax parsing, and
semantic analysis.

Lexical and syntax analysis are performed on each compilation unit eagerly using
an incremental reparsing approach. With every end-user keystroke, the framework
patches the token stream and syntax tree relative to the changes.

As a result, incremental reparsing is usually a fast process even if the unit's
source code is large. This reparsing process does not alter the outer parts of
the syntax tree outside the typically small reparsing area, which is important
for the semantic analysis that relies on the states of the syntax tree.

Semantic analysis, in contrast, is a lazy, demand-driven process for the entire
compilation project. Lady Deirdre infers individual features of the semantic
model only when you explicitly request these features.

The model is described in terms of user-defined computable functions that
compute specific node's semantic metadata based on the compilation units' syntax
tree states and other computable function values. Together, these computable
functions form the *Semantic Graph*.

Lady Deirdre computes and incrementally updates this graph partially and
automatically when you request a feature, taking into account the changes in the
compilation units.

### Compilation Units

The [Document](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/units/enum.Document.html)
object represents the source code text, token stream, and syntax tree of an
individual compilation unit.

Through the Document object, you can write to arbitrary fragments of the source
code and read its data at any time.

```rust,noplayground
let mut doc = Document::<JsonNode>::new_mutable(r#"{ "foo": 123 }"#);

 // Absolute zero-based index.
doc.write(3..6, "bar");

 // Line-column one-based index.
doc.write(Position::new(1, 4)..Position::new(1, 7), "baz");

assert_eq!(doc.substring(2..12), r#""baz": 123"#);

// Returns the root node of the syntax tree.
let _ = doc.root();

// Returns an iterator over the syntax errors.
let _ = doc.errors();

// Depth-first forth and back traverse of the syntax tree and its tokens.
doc.traverse_tree(&mut my_visitor);

// Reads tokens within the token stream.
let _ = doc.chunks(..);
```

The Document comes in two flavors: mutable and immutable. The mutable Document
supports incremental reparsing (as shown in the example above), while the
immutable Document does not support incremental reparsing but performs faster
when you load the source code text once.

There are no other API differences between these two document types, so you can
switch between the modes seamlessly. For example, if you want to switch off the
incremental compilation mode of your program, the program would function as a
pure one-pass compiler.

The Document is parameterized with the type that describes the lexical scanner
and syntax parser of the language, specifying the individual token type and the
syntax tree's node type.

### Lexis

First, you need to specify the type of the lexis. Typically, you can do this
using the [derive macro](todo) on your enum type, where the enum variants denote
individual token types. The token's lexical scanning rules are described in
terms of regular expressions.

From the [JSON example](todo):

```rust,noplayground
#[derive(Token)]
pub enum JsonToken {
    #[rule("true")]
    True,

    #[rule('{')]
    BraceOpen,

    #[rule('-'? ('0' | POSITIVE) ('.' DEC+)? (['e', 'E'] ['-', '+']? DEC+)?)]
    Number,
    
    //...
}
```

The macro will generate a highly optimized lexical scanner based on the provided
regex rules.

In Lady Deirdre, the lexical scanning process is infallible. If there are source
code fragments that do not match the specified rules, these fragments will be
recognized as fallback "mismatch" tokens, which will generate syntax errors
during the syntax parsing stage.

### Syntax

The syntax grammar is described similarly using enum types and
the [derive macro](todo).

The node's parsing rules are described in terms of LL(1) grammars, but you can
also implement your own custom parsers for individual node types, allowing for
custom parse logic with unlimited recursion, including possibly left recursion.

Within the macro's parsing rules, you can capture the results of descending rule
applications and reference these results in the enum variant fields.

This system of references forms the node-to-child relationships between the
syntax tree nodes, which is useful for depth-first tree traversal. Additionally,
the parser establishes ascending node-to-parent relationships, allowing
traversal from nodes to the tree root. Lady Deirdre's incremental reparser
ensures both kinds of references are kept up to date.

From the [JSON example](todo):

```rust,noplayground
#[derive(Node)]
#[token(JsonToken)] // Specifies a type of the Token.
pub enum JsonNode {
    #[rule($BracketOpen (items: ANY)*{$Comma} $BracketClose)]
    Array {
        #[parent] // Node-to-Parent relation.
        parent: NodeRef,
        #[child]// Node-to-Child relation.
        items: Vec<NodeRef>,
    },
    
    //...
}
```

Most of the language syntax constructs, which can be easily expressed in terms
of LL(1) grammar rules, will be described this way. However, some complex
parsing rules, such as infix expressions, will be implemented manually using
hand-written recursive-descent parsers.

The syntax trees created by Lady Deirdre are, informally speaking, abstract
syntax trees where all trivial elements such as whitespaces and comments are
intentionally omitted. However, it is worth noting that you can also build a
full [ParseTree](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/struct.ParseTree.html)
based on the same grammar, which has a different structure useful for
implementing code formatters.

The parser generated by the macro is an error-resistant parser capable of
recovering from syntax errors in the end user's code. It recovers from syntax
errors using standard "panic mode" algorithm and based on internal heuristics
statically inferred from the grammar. You have the option to explicitly
configure the recovery rules for the entire grammar and for individual parsing
rules for fine-tuning.

### Ownership

In Lady Deirdre, the source code tokens and syntax tree nodes are owned by the
[Document](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/units/enum.Document.html).

The [NodeRef](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/struct.NodeRef.html)
and [TokenRef](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/lexis/struct.TokenRef.html)
objects are globally unique (composite) numerical indices that point to a
specific node or token inside the Document. They are unique in the sense that
whenever the incremental reparser removes a node or token, the corresponding
index object becomes obsolete forever, and the newly created node and token
instance always receives a unique index object that will never clash with the
index objects created previously by any Document.

Lady Deirdre uses NodeRefs, in particular, to establish parent-child relations
between syntax tree nodes.

This approach is convenient in that NodeRefs/TokenRefs, being just numerical
indices, are cheap and easy to Copy and are memory-allocation independent. You
can easily spread them across the program to address specific objects within a
particular document.

But the downside is that to dereference the addressed instance, you always
have to have access to the corresponding Document at the dereferencing point.

```rust,noplayground
let doc: Document<JsonNode>;
let token_ref: NodeRef;

let Some(token) = token_ref.deref(&doc) else {
    panic!("TokenRef obsolete.");
}
```

### Traversing

You can traverse the syntax tree either manually by dereferencing the NodeRef
index object and inspecting the enum variant fields, or generically, using the
NodeRef's grammar-independent functions. These functions include getting the
node's parent, children, or siblings, and addressing their children by string or
numerical keys.

```rust,noplayground
let doc: Document<JsonNode>;
let node_ref: NodeRef;

let foo_ref: NodeRef = node_ref
    .parent(&doc)
    .last_child(&doc)
    .get_child(&doc, 3)
    .prev_sibling(&doc)
    .get_child(&doc, "foo");
```

You can also traverse the entire syntax tree or a branch of the tree generically
using a visitor.

```rust,noplayground
let doc: Document<JsonNode>;
let branch_ref: NodeRef;

doc.traverse_subtree(&branch_ref, &mut MyVisitor);

struct MyVisitor;

impl Visitor for MyVisitor {
    fn visit_token(&mut self, _token_ref: &TokenRef) {}
    fn enter_node(&mut self, _node_ref: &NodeRef) -> bool { true }
    fn leave_node(&mut self, _node_ref: &NodeRef) {}
}
```

### Semantics

The semantic graph of the compilation project consists
of [attributes](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Attr.html).

An attribute represents a value of arbitrary user-defined type, along with the
function that computes this value based on the values of other attributes read
within the function.

The value of the attribute can be of any type that implements Clone and Eq.

```rust,noplayground
#[derive(Clone, PartialEq, Eq)]
struct MyAttrValue {
    //...
}

impl Computable for MyAttrValue {
    type Node = MyNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        // Computes and returns attribute's value using the `context` object.
    }
}
```

The purpose of an attribute is to infer meaningful information related to a
particular node of the syntax tree. For instance, one attribute of the variable
declaration node could infer the type of that variable, while another attribute
might infer all variable uses across the code.

Attribute instances are owned by the syntax tree nodes. When defining the node,
attributes can be placed inside a special enum variant's `#[semantics]` field:

```rust,noplayground
#[derive(Node)]
struct MyNode {
    #[rule(<parse rule>)]
    SomeNodeVariant {
        //...
        #[semantics]
        semantics: Semantics<Attr<MyAttrValue>>,
    }
}
```

If a node has more than one attribute, you should define a dedicated struct
where you would put these attributes. Then, you would use this struct as a
parameter of the `Semantics<...>` object.

Lady Deirdre computes attribute values only when you query them explicitly.

```rust,noplayground
let analysis_task; // You gets this object from the Analyzer (see next sections).
let node_ref: NodeRef;

let Some(MyNode::SomeNodeVariant { semantics, ... }) = node_ref.deref() else {...}

let (_, attribute_value) = semantics
    .get().unwrap()
    .my_attr.snapshot(&analysis_task).unwrap();
```

You can find a complete setup example of the syntax tree with semantics in the
[Chain Analysis](todo) example.

### Concurrent Analysis

Lady Deirdre is capable of computing the semantic graph of your compilation
project in a multi-threaded environment, handling concurrent requests to the
semantic attributes. Specifically, the language server can manage parallel
requests from the language client (code editor) in dedicated worker threads.

The [Analyzer](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Analyzer.html)
object manages a collection of documents of the compilation project and the
semantic graph of the project.

It's assumed that this object will serve as the central synchronization point of
your compiler. You can interact with the Analyzer from multiple threads if
you're developing a multi-threaded compiler.

At any point in time, you can either edit the source code or query the semantic
graph. Due to this reason, the Analyzer doesn't allow direct access to its inner
content. Instead, it provides functions that grant access to specific
operations.

These functions return RAII-guard-like objects called "tasks" through which
necessary operations can be performed.

From the [Chain Analysis](todo) example:

```rust,noplayground
let analyzer = Analyzer::<ChainNode>::new(AnalyzerConfig::default());

let doc_id;

{
    // A handle object through which we can signalize the task's worker
    // to cancel it's job. 
    let handle = TriggerHandle::new();

    // Requests Mutation task through which we can add new
    // or edit existing documents.
    let mut task = analyzer.mutate(&handle, 1).unwrap();

    // Returns a unique identifier of the new document
    doc_id = task.add_mutable_doc(INPUT);
}

{
    let handle = TriggerHandle::new();
    
    // Requests semantic-analysis task.
    let task = analyzer.analyze(&handle, 1).unwrap();

    // Here we can fetch the document by `doc_id`, traverse its nodes,
    // and query their semantics using the `task` object.
}
```

The Analyzer's task system supports task priorities and a graceful shutdown
mechanism. The inner task manager of the Analyzer can signal the task's worker
thread to temporary interrupt its job based on the currently requested task
priorities.

It's important to note that the Analyzer itself is not a thread manager and does
not spawn any threads. Thread job management is not a goal of Lady Deirdre.

You can also use the Analyzer from a single main thread only. For example, you
can build your compiler to the wasm target and use the Analyzer's tasks
sequentially.
