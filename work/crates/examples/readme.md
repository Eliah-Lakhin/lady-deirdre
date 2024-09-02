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

# Lady Deirdre Examples

This crate contains examples showcasing the core features of Lady Deirdre.

The source code of each example is accompanied by detailed explanations and
comments in the [User Guide](https://lady-deirdre.lakhin.com/). Therefore, it is
recommended to explore them alongside the corresponding chapters of the guide.

Each example is located in its own crate module within the "src" directory.
The root "mod.rs" file of each module includes runnable tests that demonstrate
specific features of the example.

- [Json Grammar](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples/src/json_grammar).

  Demonstrates how to use the Token and Node derive macros on arbitrary
  user-defined enums to establish programming language lexical and syntax
  grammar using a simple JSON language example.

  Relevant User Guide chapters: [Lexical Grammar](https://lady-deirdre.lakhin.com/lexis/lexical-grammar.html)
  and [Syntax Grammar](https://lady-deirdre.lakhin.com/syntax/syntax.html)

- [Expr Parser](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples/src/expr_parser).

  Shows how to implement a hand-written and error-resistant recursive descendant
  syntax parser and how to use this parser together with the Node
  macro-generated parsers.

  This example parses boolean expressions (e.g., `(true | false) & true`) using
  the Pratt algorithm.

  Relevant User Guide chapter: [Hand-Written Parsers](https://lady-deirdre.lakhin.com/syntax/hand-written-parsers.html).

- [Chain Analysis](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples/src/chain_analysis).

  Illustrates how to set up and use the semantic analysis framework of Lady
  Deirdre with simple variable introduction statements and nested code blocks,
  such as: `{ let x = 10; { let y = x; } }`.

  This example incrementally infers the actual numeric values of introduced
  variables by analyzing the chains of variable references in dynamically
  evolving source code.

  Relevant User Guide chapter: [Semantics](https://lady-deirdre.lakhin.com/semantics/semantics.html).

- [Shared Semantics](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples/src/shared_semantics).

  Illustrates how to organize cross-file semantic connections.

  The source code of the files contains a set of key-value pairs, where the key
  is any identifier, and the value is either a numeric value or a reference to
  another identifier within the same file or a different one. This example
  demonstrates resolving key values through a system of references down to their
  numeric values.

  Relevant User Guide chapter: [Multi-File Analysis](https://lady-deirdre.lakhin.com/semantics/multi-file-analysis.html).

- [JSON Formatter](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples/src/json_formatter).

  Shows how to use the source code formatter tools of Lady Deirdre to implement
  a source code reformatting program based on the already defined syntax grammar
  of the JSON language.

  Relevant User Guide chapter: [Code Formatters](https://lady-deirdre.lakhin.com/code-formatters/code-formatters.html).

- [JSON Highlight](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples/src/json_highlight).

  Demonstrates how to print source code snippets with syntax highlighting and
  annotated code fragments with user-defined messages to the terminal.
  This feature is particularly useful for displaying compiler syntax errors to
  the end user in the terminal.

  Relevant User Guide chapter: [Snippets](https://lady-deirdre.lakhin.com/snippets.html).

## Links

- [Source Code](https://github.com/Eliah-Lakhin/lady-deirdre)
- [Main Crate](https://crates.io/crates/lady-deirdre)
- [API Documentation](https://docs.rs/lady-deirdre)
- [User Guide](https://lady-deirdre.lakhin.com/)
- [Examples](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples)
- [License Agreement](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md)
