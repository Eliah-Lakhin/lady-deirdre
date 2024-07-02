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

# Lexical Grammar

The lexical grammar is defined using
the [Token derive macro](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/derive.Token.html)
on an arbitrary enum type, which represents the type of the token.

Each enum variant represents a token variant. To specify the scanning rule for
an individual variant, you annotate that variant with the `#[rule(...)]` macro
attribute and provide a regular expression to match the corresponding token.

The macro uses these expressions to build an optimized finite-state automaton,
from which it generates the scanning program..

From the [JSON example](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/json_grammar/lexis.rs#L47):

```rust,noplayground
use lady_deirdre::lexis::Token;

#[derive(Token, Clone, Copy, PartialEq, Eq)]
#[define(DEC = ['0'..'9'])]
#[define(HEX = DEC | ['A'..'F'])]
#[define(POSITIVE = ['1'..'9'] DEC*)]
#[define(ESCAPE = '\\' (
    | ['"', '\\', '/', 'b', 'f', 'n', 'r', 't']
    | ('u' HEX HEX HEX HEX)
))]
#[lookback(2)]
#[repr(u8)]
pub enum JsonToken {
    EOI = 0,

    Mismatch = 1,

    #[rule("true")]
    True,
    
    // ...

    #[rule('}')]
    BraceClose,
    
    // ...

    #[rule('"' (ESCAPE | ^['"', '\\'])* '"')]
    String,

    #[rule('-'? ('0' | POSITIVE) ('.' DEC+)? (['e', 'E'] ['-', '+']? DEC+)?)]
    Number,

    #[rule([' ', '\t', '\n', '\x0c', '\r']+)]
    Whitespace,
}
```

The type must be Copy, Eq, and `#[repr(u8)]` enum type with variants without a
body.

The macro will implement
the [Token](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.Token.html)
trait on the applicable object, providing not only the scan function itself but
also additional metadata about the lexical grammar.

## Special Tokens

The `EOI` and `Mismatch` variants must be present in the enum[^discriminant].

The EOI ("end-of-input") variant denotes the end of the source code. While
the scanner does not scan this token explicitly, it uses this variant in certain
API functions that return this token instance to indicate the end of the input
stream.

The Mismatch tokens are utilized by the scanning algorithm to represent source
code fragments that do not adhere to any lexical grammar rules. In essence, this
serves as a fallback token, indicating that the source code contains
unrecognized elements ("abracadabra").

In Lady Deirdre, scanning is, in principle, an infallible process. If the
scanning algorithm encounters a portion of the source code it cannot identify,
it emits the Mismatch token into the output stream. Depending on the parser,
this token may be recognized as a syntax parsing error.

[^discriminant]: They are determined by discriminant rather than their names.

## Regular Expressions

Inside the `#[rule(...)]` macro attribute, you specify the regular expression
that matches this token kind.

The expression language consists of a set of operators commonly found in typical
regular expression languages. These include character range match
operators (`['a'..'z', '0'..'9']`), repetition operators (`+`, `*`, `?`), and
character classes (`$upper`, `$alpha`, etc).

The macro supports predefined expressions (via `#[define(Name = Expr)]` as
shown in the example above) that could be inlined as-is an any other expression
by name, without recursion.

## Grammar Ambiguity

Every token scanning expression must match at least one character, as Lady
Deirdre does not allow empty tokens.

Additionally, the rules must be mutually exclusive. For instance, if you have a
scanning rule for identifiers `['a'..'z']+` and a dedicated keyword rule
`"package"`, both could potentially match the same text fragment "package".

To resolve this, you should prioritize the keyword variant over the identifier
by annotating it with a higher priority number `#[priority(1)]` (the default
priority being zero). Prioritizing the identifier instead would render the
keyword rule inapplicable, which is also considered an error.

The macro checks for these issues during compile time and yields corresponding
error messages when ambiguity errors are detected.

Lastly, in the above example, there is a `#[lookback(2)]` macro attribute that
specifies how many characters the rescanning algorithm should step back to
restart the incremental rescanning process. By default, this value is 1,
indicating that the rescanner must start the process from at least one character
back before the edited fragment. In the case of JSON, we need to ensure that
there are at least two characters available so that the rescanner can continue
rescanning of incomplete floating-point number literals ending with the dot
character.

## Debugging

You can debug the regular expressions by surrounding them with the `dump(...)`
operator: `'"' dump((ESCAPE | ^['"', '\'])*) '"'`. This will prompt the macro to
print the contents within the "dump" argument, displaying the state machine
transitions of this specific expression as interpreted by the macro. This
feature can be particularly useful when crafting complex lexical rules.

Additionally, you can annotate the enum type with the `#[dump]` macro attribute.
This will instruct the macro to print its generated output to the terminal,
allowing you to inspect the generated code. This is similar to using the
macro-expand tool, but the output will be properly formatted for readability.

## Guidelines

It is advisable to keep the lexical grammar as simple and granular as possible,
leaving the finer details to the syntax parsing stage.

In particular, I do not recommend scanning entire code comments and string
literals during lexical analysis. While in the example provided above, for the
sake of simplicity and demonstration, we scan string literals, in actual
applications, it would be preferable to scan just a `"` character as a single
token and define syntax parsing rules for strings in the syntax parser.

Informally speaking, you should only scan text portions between which you would
navigate when using the "ctrl ←" and "ctrl →" keyboard shortcuts in a code
editor.
