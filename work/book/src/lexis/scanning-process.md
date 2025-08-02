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

The [Token](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/lexis/trait.Token.html)
trait discussed in the previous chapter defines the scan algorithm of individual
tokens, and specific for the language grammar.

Actual scanning of the source code, and splitting it into tokens happening via
another trait [LexisSession](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/lexis/trait.LexisSession.html).
This trait is independent from particular Token implementations, and it's purpose is
running particular scanning algorithm by feeding it with the source code text
characters as an input, and getting back the bounds of the tokens from the algorithm
implementation.

How these bounds will be managed depends on the LexisSession implementation.
Some implementations may store them in buffers or more complex internal collections,
other yield scanning results directly to the API user.

Unless you want to implement your own extension to the Lady Deirdre API, you
don't need to implement this low-level interface manually, and you encouraged to
use one of the provided solutions.

Specifically, Lady Deirdre offers three kinds of implementations:

1. **Stateless Scanners**. This type of scanners run the Token scan algorithm
   on provided source code string and yields individual tokens and their
   metadata to the API user via the Iterator interface. This is the simplest
   way to scan source code text or a part of it, leaving the scanning results
   solely at the API user discretion. Also, these scanners are useful for quick and dirty
   debugging of the Token implementation.
2. **Token Buffer**. Scans the entire source code, and stores results in the internal
   growable buffer. Token Buffer provides random-access capabilities such that
   you can access and inspect tokens at any given span of the source code.
   It also allows adding more text parts at the end of the buffer, hence opening
   up a possibility for continuous scanning. But it does not provide more
   incremental editing operations (i.e., changing or deleting random source
   spans).
3. **Documents**. Full-featured stateful lexical scanner and syntax parser.
   Provides random-access to the source code tokens, and operations to edit the
   source code text at any give point. We will discuss this object in more details in
   the next chapters.

Each implementation have different performance characteristics and the feature
sets. In most cases, when you work with the lexical layer only of unchangeable
text, [TokenBuffer](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/lexis/struct.TokenBuffer.html)
is recommended choice. It has good balance between performance and feature richness.
But if you need to edit the text, consider using
[Document](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/units/enum.Document.html)
instead. Stateless Scanners are most useful if you want to use generated lexical
scanner only, and you don't need the rest of the Lady Deirdre infrastructure.

## Source Code Manager

Token Buffers and Documents both are stateful objects - they store the state of
the scanned tokens internally.

To provide a way to inspect this state, for example, to iterate through the
tokens in the specified source code span, these objects implement
[SourceCode](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/lexis/trait.SourceCode.html)
trait. We will discuss this trait API in more details in the next chapter too.

Usually, we will refer to these objects as the *source code managers*.
To sum up, source code managers typically implement two traits:

1. The LexisSession trait to provide communication channel between the source
   code text and the Token's scanning algorithm.
2. The SourceCode trait that provides access to the manager's scanned tokens for
   the end user.

To reiterate, both traits are low-level, and usually you don't need to implement
them manually, unless you want to create a new type of the source code manager.

## Stateless Scanners

With the [stateless scanners](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/lexis/trait.Scannable.html)
you can run the scanning algorithm on arbitrary strings without extra overhead.

This is useful to examine Token implementation in action.

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

[TokenBuffer](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/lexis/struct.TokenBuffer.html)
persists scanned tokens, provides a way to inspect their metadata and the text
at random points, and allows appending text continuation at the end of the
source code.

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

This type of the source code managers specifically designed for one-time and
continuation lexical scanning, and it has necessary internal optimizations for
these use case scenarios.

## Documents Without Syntax

The [Document](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/units/enum.Document.html)
object provides full incremental reparsing capabilities and requires specifying
the syntax grammar too, but you can use its incremental scanning features only
using [VoidSyntax](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/syntax/struct.VoidSyntax.html).

```rust,noplayground
use lady_deirdre::{lexis::SourceCode, syntax::VoidSyntax, units::Document};

let mut doc = Document::<VoidSyntax<JsonToken>>::new_mutable(r#"{ "foo": 123 }"#);

doc.write(9..12, "456");

assert_eq!(doc.substring(..), r#"{ "foo": 456 }"#);
```
