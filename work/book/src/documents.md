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

# Documents

A Document is the primary object designed to store the source code text of a
compilation unit, along with its lexical and syntax structures. It ensures all
three components remain synchronized.

This object has two
constructors: [Document::new_mutable](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/units/enum.Document.html#method.new_mutable)
and [Document::new_immutable](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/units/enum.Document.html#method.new_immutable).

Both constructors take the source code text as the initial input for the
Document. The first constructor creates a Document that supports write
operations, allowing for the editing of arbitrary source code spans within the
document's text. The second constructor creates a Document that does not support
write operations but is slightly faster during the document's creation.

To edit a mutable Document, you use
the [Document::write](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/units/enum.Document.html#method.write)
function. Thisfunction takes an arbitrary span of the source code text that you
wish to rewrite and the text you want to insert in place of the specified span.
It rescans the tokens of the affected source code fragment (localized to the
span) and incrementally reparses the part of the syntax tree related to these
changes. The mutable Document is specifically designed to be efficient for write
operations. Incremental updates typically take milliseconds, even for large
compilation units, making it feasible to write into the mutable Document with
each end-user keystroke.

```rust,noplayground
use lady_deirdre::{lexis::SourceCode, units::Document};

let mut doc = Document::<JsonNode>::new_mutable(r#"{ "foo": 123 }"#);

doc.write(9..12, "456");

assert_eq!(doc.substring(..), r#"{ "foo": 456 }"#);
```

If the compiler serves the dual purpose of being a programming language compiler
that compiles the entire codebase at once, and a language server that
continuously analyzes a dynamically evolving compilation project, you can
optimize the program's performance by switching between immutable and mutable
documents depending on the current mode of the program.

## Loading By Parts

When the content of a file is being transferred in parts, for example, through a
network or by loading the file from disk in chunks, you can create
a [TokenBuffer](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/lexis/struct.TokenBuffer.html)
and continuously append these chunks into the buffer.

Once the file loading is complete, you can use this token buffer as input for
the Document constructor.

```rust,noplayground
use lady_deirdre::{
    lexis::{SourceCode, TokenBuffer},
    units::Document,
};

// Ideally, you can use `TokenBuffer::with_capacity()` if the final length of
// the file is known upfront.
let mut buf = TokenBuffer::new();

buf.append(r#"{ "foo": "#);
buf.append(r#"123 }"#);

let doc = Document::<JsonNode>::new_immutable(buf);

assert_eq!(doc.substring(..), r#"{ "foo": 123 }"#);
```

This approach is likely more efficient than writing the chunks to the end of a
mutable Document. TokenBuffer is more efficient for lexical scanning when new
fragments are being appended, and this method postpones the syntax parsing of
the not-yet-completed source code text.

## Syntax-less Documents

Sometimes, you may want to use the Document to store just the source code text
and the lexical analysis of the file, bypassing the syntax analysis stage. For
example, a mutable Document can be used as a simple storage of strings with
random read/write access.

In this case, you can use
the [VoidSyntax](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/struct.VoidSyntax.html)
helper object to enforce the Document to bypass syntax analysis.

```rust,noplayground
use lady_deirdre::{lexis::SourceCode, syntax::VoidSyntax, units::Document};

let mut doc = Document::<VoidSyntax<JsonToken>>::new_mutable(r#"{ "foo": 123 }"#);

doc.write(9..12, "456");

assert_eq!(doc.substring(..), r#"{ "foo": 456 }"#);
```

The above document has full capabilities of
the [SourceCode](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/lexis/trait.SourceCode.html)
trait, but
the [SyntaxTree](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/trait.SyntaxTree.html)
implementation represents a dummy syntax tree with just a single root node that
covers empty text.

## Documents Identification

Each instance of the Document (and similar source code storage objects such as
the TokenBuffer) has a globally unique identifier within the current process.

The [Document::id](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/units/enum.Document.html#method.id)
function returns an object of
type [Id](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/arena/struct.Id.html).
This object is Copy, Eq, and Hash, ensuring that two distinct instances of
documents have distinct identifiers.

Related objects of a Document, such
as [NodeRef](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/struct.NodeRef.html),
[TokenRef](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/lexis/struct.TokenRef.html),
and others, store the identifier of the document to which they belong.

For example, from a NodeRef referential object, you can determine the identifier
of the document to which the referred node belongs. This is particularly useful
when working with multi-document compilers.

```rust,noplayground
use lady_deirdre::{
    arena::Identifiable,
    syntax::{NodeRef, SyntaxTree},
    units::Document,
};

let doc = Document::<JsonNode>::new_immutable(r#"{ "foo": 123 }"#);

let root: NodeRef = doc.root_node_ref();

assert_eq!(doc.id(), root.id());
```

## Documents Naming

You can assign a possibly non-unique string name to the document to simplify
document identification during debugging. For instance, you can use a file name
as a document's name.

The [Id::set_name](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/arena/struct.Id.html#method.set_name)
and [Id::name](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/arena/struct.Id.html#method.name)
functions set and retrieve the current document name, respectively.
Additionally, the crate API uses the document's name in various debugging
functions. For example, the Display implementation of the Document object prints
the source code text to the terminal with the document name as the snippet's
caption.

```rust,noplayground
use lady_deirdre::{arena::Identifiable, units::Document};

let doc = Document::<JsonNode>::new_immutable(r#"{ "foo": 123 }"#);

// By default, the document has an empty name.
assert_eq!(doc.id().name(), "");

doc.id().set_name("Foo Doc");

assert_eq!(doc.id().name(), "Foo Doc");
```
