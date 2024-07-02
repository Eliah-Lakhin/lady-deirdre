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

# Code Inspection

## Text Addressing

In Lady Deirdre, the minimal unit for indexing the source code text is the
Unicode character.

A [Site](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/type.Site.html)
is a numeric type (an alias of `usize`) representing the absolute Unicode
character index in a string.

[SiteSpan](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/type.SiteSpan.html)
is an alias type of `Range<Site>` that denotes a fragment (or *span*) of the
source code.

Most API functions within the crate conveniently
accept [impl ToSite](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.ToSite.html)
or [impl ToSpan](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.ToSpan.html)
objects when users need to address specific source code
characters or spans of characters. These traits facilitate automatic conversion
between different representations of source code indices.

One example of a source code index type is
the [Position](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.Position.html)
object, which references code in terms of the line and column within the
line[^position]. It implements the *ToSite* trait.

For any type implementing the *ToSite* trait, *ToSpan* is automatically
implemented for all standard Rust range types with bound of this type.
For instance, both *Site* and *Position* types implement the *ToSite* trait,
making `10..20`, `10..=20`, and `10..`,
and `Position::new(10, 40)..Position::new(14, 2)` valid span types.

However, a particular span instance could be invalid; for instance, `20..10` is
invalid because its lower bound is greater than its upper bound.

Certain API functions in the crate (e.g.,
[Document::write](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/units/enum.Document.html#method.write))
require that the specified span must be valid; otherwise, the function would
panic. This behavior aligns with Rust's behavior when indexing arrays with
invalid ranges.

You can check the validity of a range upfront using
the [ToSpan::is_valid_span](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.ToSpan.html#tymethod.is_valid_span)
function.

The RangeFull `..` object always represents the entire content and is always
valid.

```rust,noplayground
use lady_deirdre::lexis::{Position, ToSpan, TokenBuffer};

let mut buf = TokenBuffer::<JsonToken>::from("foo\nbar\nbaz");

assert!((2..7).is_valid_span(&buf));
assert!((2..).is_valid_span(&buf));
assert!((..).is_valid_span(&buf));
assert!(!(7..2).is_valid_span(&buf));
assert!((Position::new(1, 2)..Position::new(3, 1)).is_valid_span(&buf));
```

[^position]: Please note that the line and column numbers in
the [Position](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.Position.html)
object are one-based: 1 denotes the first line, 2 denotes the second line, and
so forth.

## Text Inspection

The
following [SourceCode](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.SourceCode.html)'
s functions enable you to query various metadata of the compilation unit's text.

```rust,noplayground
use lady_deirdre::lexis::{SourceCode, TokenBuffer};

let mut buf = TokenBuffer::<JsonToken>::from("foo, bar, baz");

// The `substring` function returns a `Cow<str>` representing the substring
// within the specified span.
// The underlying implementation attempts to return a borrowed string whenever
// possible.
assert_eq!(buf.substring(2..7), "o, ba");

// Iterates through the Unicode characters in the span.
for ch in buf.chars(2..7) {
    println!("{ch}");
}

// A total number of Unicode characters.
assert_eq!(buf.length(), 13);

// Returns true if the code is empty (contains no text or tokens).
assert!(!buf.is_empty());

// A total number of lines (delimited by `\n`).
assert_eq!(buf.lines().lines_count(), 1);
```

From `buf.lines()`, you receive
a [LineIndex](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.LineIndex.html)
object that provides additional functions for querying metadata about the source
code lines. For example, you can fetch the length of a particular line using
this object.

## Tokens Iteration

The [SourceCode::cursor](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.SourceCode.html#tymethod.cursor)
and its simplified
version [chunks](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.SourceCode.html#method.chunks)
allow you to iterate through the tokens of the source code.

Both functions accept a span of the source code text and yield tokens that
"touch" the specified span. Touching means that the token's string is fully
covered by, intersects with, or at least contacts the span within its bounds.

For example, if the text "FooBarBazWaz" consists of the tokens "Foo", "Bar",
"Baz", and "Waz", the span `3..7` would contact the "Foo" token (3 is the end of
the token's span), fully cover the "Bar" token, and intersect with the "Baz"
token (by the "B" character). However, the "Waz" token is outside of this span
and will not be yielded.

In other words, these functions attempt to yield the widest set of tokens that
are in any way related to the specified span.

The *chunks* function simply returns a standard iterator over the token
metadata. Each
[Chunk](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.Chunk.html)
object contains the token instance, a reference to its string, the absolute Site
of the beginning of the token, and the substring length in Unicode
characters[^chunk].

```rust,noplayground
use lady_deirdre::lexis::{Chunk, SourceCode, TokenBuffer};

let buf = TokenBuffer::<JsonToken>::from("123 true null");

for Chunk {
    token,
    string,
    site,
    length,
} in buf.chunks(..)
{
    println!("---");
    println!("Token: {token:?}");
    println!("String: {string:?}");
    println!("Site: {site}");
    println!("Length: {length}");
}
```

The *cursor* function returns a more
complex [TokenCursor](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.TokenCursor.html)
object that implements a cursor-like API with built-in lookahead capabilities
and manual control over the iteration process. This object is particularly
useful for syntax parsing and will be discussed in more detail in the subsequent
chapters of this guide.

To give you a brief overview of the token cursor, the above code could be
rewritten with the token cursor as follows:

```rust,noplayground
use lady_deirdre::lexis::{SourceCode, TokenBuffer, TokenCursor};

let buf = TokenBuffer::<JsonToken>::from("123 true null");

let mut cursor = buf.cursor(..);

loop {
    // 0 means zero lookahead -- we are looking at the point of where the cursor
    // is currently pointing.
    let token = cursor.token(0);

    // If the cursor reached the end of input, we are breaking the loop.
    if token == JsonToken::EOI {
        break;
    }

    println!("---");
    println!("Token: {:?}", cursor.token(0));
    println!("String: {:?}", cursor.string(0));
    println!("Site: {:?}", cursor.site(0));
    println!("Length: {:?}", cursor.length(0));

    // Manually moves token cursor to the next token.
    cursor.advance();
}
```

[^chunk]: Note that the *Chunk* object represents a valid span and implements
the *ToSpan* trait.
