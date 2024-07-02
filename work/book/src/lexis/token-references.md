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

# Token References

Lexical structures (tokens) are owned by the source code managers, such as
TokenBuffers and Documents, which implement the SourceCode trait through which
you can access the tokens.

The [TokenRef](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.TokenRef.html)
is a convenient interface containing a composite numeric index that uniquely
addresses a token in the source code. As a numeric index, it is a Copy and
lifetime-independent object that you can freely use throughout your program.
However, TokenRef could potentially represent an invalid or obsolete pointer.
Therefore, most TokenRef functions require passing a reference to the source
code manager and may return None if the corresponding token does not exist in
the manager.

For example,
the [TokenRef::deref](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.TokenRef.html#method.deref)
function "dereferences" the token and returns Some if the token exists in the
specified compilation unit, or None otherwise.

```rust,noplayground
use lady_deirdre::lexis::{SourceCode, TokenBuffer, TokenCursor, TokenRef};

let mut buf_1 = TokenBuffer::<JsonToken>::from("123 true null");
let buf_2 = TokenBuffer::<JsonToken>::new();

// Get the reference to the first token in the TokenBuffer.
let first_token: TokenRef = buf_1.cursor(..).token_ref(0);

// Gets an instance of the Token instance.
assert_eq!(first_token.deref(&buf_1), Some(JsonToken::Number));

// Gets a string fragment covered by this token.
assert_eq!(first_token.string(&buf_1), Some("123"));

// Checks validity of the TokenRef for specified compilation unit.
assert!(first_token.is_valid_ref(&buf_1));

// However, this token reference is not valid for buf_2.
assert_eq!(first_token.deref(&buf_2), None);

// Removing all tokens from the TokenBuffer.
buf_1.clear();

// As such, the reference is no longer valid for the buf_1 as well.
assert_eq!(first_token.deref(&buf_1), None);
```

The source of TokenRef objects could be token cursors (as in the example above),
but typically, you will obtain them by inspecting nodes of the syntax trees.

## TokenRef Lifetime

The TokenRef reference is unique in the following ways:

1. It uniquely addresses a particular compilation unit.
2. It uniquely addresses a particular token within this unit.

If the incremental source code manager (such
as [Document](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/units/enum.Document.html))
rescans the source code fragment to which the token belongs, its TokenRef
reference would effectively become obsolete. Every new token in the Document
would receive a new unique instance of the TokenRef object.

The [TokenRef::is_valid_ref](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.TokenRef.html#method.is_valid_ref)
function tests the validity of the reference for a specified compilation unit.

## Nil TokenRef

TokenRefs have one special "nil" value. Nil token references are special
references that intentionally do not address any tokens within any compilation
unit.

These TokenRefs are created with
the [TokenRef::nil](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.TokenRef.html#method.nil)
function and can be tested using
the [is_nil](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.PolyRef.html#tymethod.is_nil)
function. The *is_nil* function returns true only for token references created
this way; otherwise, it returns false, even if the TokenRef is obsolete.

Nil TokenRefs are useful to mimic the `Option::None` discriminant, so you don't
have to wrap a TokenRef type in an Option.

The crate API never wraps TokenRef in an `Option<TokenRef>` type. Instead, if
the value cannot be computed, the API uses a nil TokenRef.
