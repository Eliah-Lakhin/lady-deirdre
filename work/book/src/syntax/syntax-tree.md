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

# Syntax Tree

The syntax API shares many similarities with the lexis API architecture:

1. The syntax grammar, implemented by
   the [Node](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/trait.Node.html)
   type, is distinct from the syntax tree manager responsible for actual parsing
   and storage of the syntax tree.
2. The syntax tree manager implements
   the [SyntaxTree](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/trait.SyntaxTree.html)
   trait, providing access to the parsed syntax tree through its functions.
3. There are several syntax manager implementations with distinct sets of
   features and performance characteristics.
4. Individual nodes within the syntax tree are addressed using
   the [NodeRef](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/struct.NodeRef.html)
   referential object, which points to concrete node instances owned by the
   syntax tree manager.

The simplest implementation of the syntax tree manager is
the [ImmutableSyntaxTree](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/struct.ImmutableSyntaxTree.html),
which performs one-time parsing without incremental reparsing capabilities but
has the fastest computation performance.

This object accepts a token cursor providing access to the input stream. For
example, you can obtain this cursor from the TokenBuffer.

```rust,noplayground
use lady_deirdre::{
    lexis::{SourceCode, TokenBuffer},
    syntax::{ImmutableSyntaxTree, SyntaxTree},
};

let tokens = TokenBuffer::<JsonToken>::from(r#"{
    "foo": true,
    "bar": [123, null]
}"#);

// Parsing the entire set of tokens in the token buffer.
let tree = ImmutableSyntaxTree::<JsonNode>::parse(tokens.cursor(..));

// Ensuring that the ImmutableSyntaxTree successfully parsed
// the input stream without syntax errors.
assert!(tree.errors().next().is_none());
```

The above code is verbose because it requires manual setup of the TokenBuffer
and its token cursor.

More commonly, we can utilize the
immutable [Document](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/units/enum.Document.html),
which is backed by the TokenBuffer and ImmutableSyntaxTree under the hood.
Through this object, we can directly scan and parse the source code text. This
object implements both the SourceCode and SyntaxTree traits, allowing us to
access the lexical structure of the compilation unit as well.

```rust,noplayground
use lady_deirdre::{syntax::SyntaxTree, units::Document};

let doc = Document::<JsonNode>::new_immutable(r#"{
   "foo": true,
   "bar": [123, null]
}"#);

assert!(doc.errors().next().is_none());
```
