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

# Debugging

When designing a syntax parser, it can be useful to perform quick and
straightforward observations of the parser's step-by-step actions.

The built-in
[Node::debug](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/trait.Node.html#method.debug)
function accepts a string of the source code text and prints to the terminal the
hierarchical structure that shows how the parser descends into the node variant
parsing procedures and what tokens these procedures consumed. Additionally, it
will include the points where the parser detected syntax errors.

For example, the following code:

```rust,noplayground
use lady_deirdre::syntax::Node;
    
JsonNode::debug(r#"{
    "foo": true,
    "bar": [123 "baz"]
}"# );
```

will print something like this:

```text
 Root {
     Object {
         $BraceOpen
         $Whitespace
         Entry {
             String {
                 $String
             } String
             $Colon
             $Whitespace
             True {
                 $True
             } True
         } Entry
         $Comma
         $Whitespace
         Entry {
             String {
                 $String
             } String
             $Colon
             $Whitespace
             Array {
                 $BracketOpen
                 Number {
                     $Number
                 } Number
                 $Whitespace
                 --- error ---
                 String {
                     $String
                 } String
                 $BracketClose
             } Array
         } Entry
         $Whitespace
         $BraceClose
     } Object
 } Root
```

## Errors Printing

Note that in the above example, the parser encountered a syntax error when
parsing the JSON array (missing a comma between `123` and `"baz"`).

You can generically iterate and print syntax errors using
the [SyntaxError::display](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/struct.SyntaxError.html#method.display)
function.

```rust,noplayground
use lady_deirdre::{syntax::SyntaxTree, units::Document};

// Parsing the syntax and lexis of the source code into the immutable Document.
let doc = Document::<JsonNode>::new_immutable(r#"{
    "foo": true,
    "bar": [123 "baz"]
}"#);

for error in doc.errors() {
    println!("{:#}", error.display(&doc));
}
```

This code will print annotated snippets of the source code, pointing to the
fragments where the errors occur, along with the default generated error
messages.

```text
   ╭──╢ Unit(1) ╟──────────────────────────────────────────────────────────────╮
 1 │ {                                                                         │
 2 │     "foo": true,                                                          │
 3 │     "bar": [123 "baz"]                                                    │
   │                ╰╴ missing ',' in Array                                    │
 4 │ }                                                                         │
   ├───────────────────────────────────────────────────────────────────────────┤
   │ Array syntax error.                                                       │
   ╰───────────────────────────────────────────────────────────────────────────╯
```

## Syntax Tree Printing

Finally, using
the [CompilationUnit::display](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/units/trait.CompilationUnit.html#method.display)[^treedisplay]
method, you can print the output syntax tree to the terminal.

```rust,noplayground
use lady_deirdre::{
    syntax::SyntaxTree,
    units::{CompilationUnit, Document},
};

let doc = Document::<JsonNode>::new_immutable(r#"{
    "foo": true,
    "bar": [123 "baz"]
}"#);

println!("{:#}", doc.display(&doc.root_node_ref()));
```

Outputs:

```text
Root(entry: 0) {
    object: Object(entry: 1) {
        start: $BraceOpen(chunk_entry: 0) {
            string: "{",
            length: 1,
            site_span: 0..1,
            position_span: 1:1 (1 char),
        },
        entries: [
            Entry(entry: 2) {
                key: String(entry: 3) {
                    value: $String(chunk_entry: 2) {
                        string: "\"foo\"",
                        length: 5,
                        site_span: 6..11,
                        position_span: 2:5 (5 chars),
                    },
                },
                value: True(entry: 4) {
                    token: $True(chunk_entry: 5) {
                        string: "true",
                        length: 4,
                        site_span: 13..17,
                        position_span: 2:12 (4 chars),
                    },
                },
            },
            Entry(entry: 5) {
                key: String(entry: 6) {
                    value: $String(chunk_entry: 8) {
                        string: "\"bar\"",
                        length: 5,
                        site_span: 23..28,
                        position_span: 3:5 (5 chars),
                    },
                },
                value: Array(entry: 7) {
                    start: $BracketOpen(chunk_entry: 11) {
                        string: "[",
                        length: 1,
                        site_span: 30..31,
                        position_span: 3:12 (1 char),
                    },
                    items: [
                        Number(entry: 8) {
                            value: $Number(chunk_entry: 12) {
                                string: "123",
                                length: 3,
                                site_span: 31..34,
                                position_span: 3:13 (3 chars),
                            },
                        },
                        String(entry: 9) {
                            value: $String(chunk_entry: 14) {
                                string: "\"baz\"",
                                length: 5,
                                site_span: 35..40,
                                position_span: 3:17 (5 chars),
                            },
                        },
                    ],
                    end: $BracketClose(chunk_entry: 15) {
                        string: "]",
                        length: 1,
                        site_span: 40..41,
                        position_span: 3:22 (1 char),
                    },
                },
            },
        ],
        end: $BraceClose(chunk_entry: 17) {
            string: "}",
            length: 1,
            site_span: 42..43,
            position_span: 4:1 (1 char),
        },
    },
}
```

[^treedisplay]: Keep in mind that this function accepts either a TokenRef or a
NodeRef. Supplying a NodeRef of a syntax tree branch allows you to print only
the subtree of this branch, while providing a TokenRef enables you to print
detailed metadata about the referred token.
