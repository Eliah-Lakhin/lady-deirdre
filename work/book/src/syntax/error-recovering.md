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

# Error Recovering

Once the node variant parser takes control flow, it has to parse the input
stream regardless of its content. It must consume at least one token from this
stream (if the stream is non-empty, and unless the node variant is a root node)
and must return a fully initialized instance of the corresponding node variant.

In other words, variant parsers behave eagerly in an attempt to parse the input,
regardless of the context from which they were called.

Given that the input stream is potentially an arbitrary sequence of tokens, the
parser must do its best to recognize the rule on this stream and is subject to
heuristic error recovery.

The generated parser performs error recovery whenever it encounters a token that
is not expected in the current parse state.

For instance, if we are parsing a set of identifiers separated by commas and the
end user forgets to put a comma between two identifiers, the parser might decide
to continue parsing from the next identifier, yielding a parse error at the
position where the comma was absent (a so-called "insert recovery").

The generated parser performs such recoveries based on preliminary compile-time
static analysis of the rule expression. However, it usually prefers a "panic
recovery" strategy by default, which is the most common error recovery approach
in LL grammars.

## Panic Recovery

In the panic recovery mode, if the parser encounters a token that is not
expected in the current parse position, it eagerly consumes this token and the
following tokens until it finds the next token from which it can resume the
normal parsing process from its current parse state.

This approach generally works well in many cases, except that the parser might
consume too many tokens before finding something meaningful to resume. Sometimes
it is better to halt the parsing process earlier and return control flow to the
ascending parser. For example, if we are parsing Rust's `let x` syntax and
encounter another `let` token before reading the variable identifier, it would
be better to halt the parsing of the current let-statement, assuming that the
user started another statement in the ascending context.

In the macro, you can specify a set of panic-recovery halting tokens using
the `#[recovery(...)]` macro attribute.

In the [JSON example](todo), we specify the following recovery configuration:

```rust,noplayground
#[recovery(
    $BraceClose,
    $BracketClose,
    [$BraceOpen..$BraceClose],
    [$BracketOpen..$BracketClose],
)]
pub enum JsonNode {
    // ...
}
```

This configuration will be applied to all parsing rules, but you can override it
for specific rules using the same macro attribute.

In the example above, we configure two halting tokens: "BraceClose" and
"BracketClose". Additionally, we set up so-called recovery
groups (`[$BraceOpen..$BraceClose]`). The group consists of two tokens: the open
token and the close token of the group. Whenever the recoverer encounters an
open token of the group followed consistently by the close token somewhere else,
it omits the entire sequence of tokens surrounded by the open and close tokens,
regardless of whether the surrounded content contains halting tokens. In other
words, the recoverer considers a system of nested groups as a whole to be
skipped during recovery.

In more realistic grammar than JSON, such as Rust syntax, you would probably use
semicolons and the statement starting tokens ("let", "use", etc.) as common
halting tokens, and the open-close braces as groups.

## Mismatched Captures

If during error recovery the recoverer fails to recognize a token or a node that
is a target for capturing, the parser sets enum variant fields to reasonable
defaults:

- TokenRef or NodeRef fields will be set to a nil value.
- Vectors will be left empty or partially completed (if the parser managed to
  successfully pass some of the repetition iterations).
