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

# Scanning Process

The [Token](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/lexis/trait.Token.html)
trait discussed in the previous chapter defines the scanning algorithm for
individual tokens, specific to the language grammar.

Actual scanning of source code and splitting it into tokens happens via another
trait, [LexisSession](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/lexis/trait.LexisSession.html).
This trait is independent of particular Token implementations; its purpose is
to run a scanning algorithm by feeding it the source code text characters as
input and registering the bounds of the tokens produced by that algorithm.

How these bounds are managed depends on the LexisSession implementation. Some
implementations store them in buffers or more complex internal collections;
others yield scanning results directly to the API user.

Unless you are implementing your own extension to the Lady Deirdre API, you do
not need to implement this low-level interface manually, and are encouraged to
use one of the provided solutions.

Specifically, Lady Deirdre offers three kinds of implementations:

1. **Stateless Scanners**. These scanners run the Token scanning algorithm on a
   provided source code string and yield individual tokens with their metadata
   to the API user via the Iterator interface. This is the simplest way to scan
   source code (or a portion of it), leaving the scanning results entirely to the
   API user. These scanners are also useful for quick-and-dirty debugging of a
   Token implementation.
2. **Token Buffer**. Scans the entire source code and stores the results in an
   internal growable buffer. TokenBuffer provides random-access capabilities,
   allowing inspection of tokens at any span of the source code. It also allows
   appending additional text at the end of the buffer, enabling continuous
   scanning, but it does not support finer-grained incremental edits (e.g.,
   modifying or deleting arbitrary spans).
3. **Document**. A full-featured stateful lexical scanner and syntax parser.
   Provides random access to source code tokens and allows editing the source
   text at any given point. We will discuss this object in more detail in the
   next chapters.

Each implementation has different performance characteristics and feature sets.
In most cases, when working only with the lexical layer of immutable text,
[TokenBuffer](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/lexis/struct.TokenBuffer.html)
is the recommended choice: it balances performance and features well. If you
need to edit the text, consider using
[Document](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/units/enum.Document.html)
instead. Stateless scanners are most useful when you only need the generated
lexical stream and do not require the rest of the Lady Deirdre infrastructure.

## Source Code Manager

TokenBuffer and Document are both stateful objects: they store the state of
scanned tokens internally.

To allow inspection of this state — for example, iterating through tokens within
a specified source code span — these objects implement the
[SourceCode](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/lexis/trait.SourceCode.html)
trait. We will discuss this trait's API in more detail in the next chapter as
well.

We usually refer to these objects as *source code managers*. In summary, source
code managers typically implement two traits:

1. The LexisSession trait, which provides a communication channel between the
   source code text and the Token scanning algorithm.
2. The SourceCode trait, which gives end users access to the manager's scanned
   tokens.

To reiterate, both traits are low-level, and you typically do not need to
implement them manually unless you are creating a new type of source code
manager.

## Stateless Scanners

With the
[stateless scanners](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/lexis/trait.Scannable.html),
you can run the scanning algorithm on arbitrary strings with minimal overhead.
This is useful for examining a Token implementation in action.

```rust,noplayground
use lady_deirdre::lexis::Scannable;

let text = r#"{ "foo": ["bar", 1, false] }"#;

let mut token_vector = Vec::new();

// `Scannable::tokens()` creates a stateless lexical scanner that iterates
// through the scanned tokens.
for token in text.tokens::<JsonToken>() {
    token_vector.push(token);
}

println!("{:?}", token_vector);

// Or using just one line when debugging:

println!("{:?}", text.tokens::<JsonToken>());
```

## Token Buffer

[TokenBuffer](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/lexis/struct.TokenBuffer.html)
persists scanned tokens, allows inspection of their metadata and the underlying
text at arbitrary points, and supports appending additional text to the end of
the source code.

```rust,noplayground
use lady_deirdre::lexis::{TokenBuffer, SourceCode};

let mut buffer = TokenBuffer::<JsonToken>::from("[1, 2, 3");

assert_eq!(buffer.substring(..), "[1, 2, 3");

buffer.append(", 4, 5, 6]");

assert_eq!(buffer.substring(..), "[1, 2, 3, 4, 5, 6]");

// Prints all tokens in the token buffer to the terminal.
for chunk in buffer.chunks(..) {
    println!("{:?}", chunk.token);
}
```

This type of source code manager is specifically designed for one-time and
continued lexical scanning and includes internal optimizations for these use
cases.

## Documents Without Syntax

The [Document](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/units/enum.Document.html)
object provides full incremental reparsing capabilities and normally requires
specifying the syntax grammar, but you can use only its incremental scanning
features by using [VoidSyntax](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/syntax/struct.VoidSyntax.html).

```rust,noplayground
use lady_deirdre::{lexis::SourceCode, syntax::VoidSyntax, units::Document};

let mut doc = Document::<VoidSyntax<JsonToken>>::new_mutable(r#"{ "foo": 123 }"#);

doc.write(9..12, "456");

assert_eq!(doc.substring(..), r#"{ "foo": 456 }"#);
```
