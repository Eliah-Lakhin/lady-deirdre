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

# Token Buffer

The simplest data structure for storing the token stream is
the [TokenBuffer](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/lexis/struct.TokenBuffer.html).

It holds both the source code text and the token stream but has limited
incremental rescanning capabilities, only allowing appending to the end of the
source code. Token buffers are useful for loading large files from disk or
network incrementally, particularly when data arrives in parts.

You are encouraged to use token buffer if you don't need general incremental
rescanning capabilities and if you want to store only the source code with
tokens, or if you plan to initially load the source code and later reupload it
to a general-purpose compilation unit storage
like [Document](https://docs.rs/lady-deirdre/2.1.0/lady_deirdre/units/enum.Document.html).

Token buffer offers the fastest scanning implementation among other Lady Deirdre
compilation unit storages, providing high performance when iterating through
token chunks and source code substrings.

Also, this object is useful for examining the results of the lexical scanner
output.

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
