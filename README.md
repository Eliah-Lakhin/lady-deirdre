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

# Lady Deirdre

[![Crate](https://img.shields.io/crates/v/lady-deirdre?label=Crate)](https://crates.io/crates/lady-deirdre)
[![API Docs](https://img.shields.io/docsrs/lady-deirdre?label=API%20Docs)](https://docs.rs/lady-deirdre)
[![User Guide](https://img.shields.io/badge/User_Guide-616161)](https://lady-deirdre.lakhin.com/)
[![Examples](https://img.shields.io/badge/Examples-616161)](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples)

<img align="right" height="220" style="float: right; margin-left: 10px; width: 220px" alt="Lady Deirdre Logo" src="https://raw.githubusercontent.com/Eliah-Lakhin/lady-deirdre/master/work/logo.jpg" />

Lady Deirdre is a framework for incremental programming language compilers,
interpreters, and source code analyzers.

This framework helps you develop a hybrid program that acts as a language
compiler or interpreter and as a language server for a code editor's language
extension. It offers the necessary components to create an in-memory
representation of language files, including the source code, their lexis and
syntax, and the semantic model of the entire codebase. These components are
specifically designed to keep the in-memory representation in sync with file
changes as the codebase continuously evolves in real time.

Lady Deirdre is the perfect tool if you want to start a new programming language
project.

## Key Features

- **Parser Generator Using Macros**.

  Lexical and syntax grammar of the language are specified using derive macros
  on enum types, where the enum variants represent individual tokens and nodes
  with parsing rules.

- **Hand-Written Parsers**.

  The API enables the development of hand-written recursive-descent parsers with
  unlimited lookahead and their seamless integration with macro-generated parsers.

- **Error Resilience**.

  The resulting parser will be error-resistant and capable of building a syntax
  tree from incomplete source code.

- **Semantics Analysis Framework**.

  Lady Deirdre includes a built-in semantic analyzer that manages arbitrary
  on-demand computations on the syntax trees in terms of referential attributes.

- **Incremental Compilation**.

  These components continuously patch the in-memory representation of the
  codebase structures in response to the end user's incremental edits of the
  file texts. Processing of the changes is typically fast, even in large
  codebases.

- **Parallel Computations**.

  The API is specifically designed for both multi-threaded and single-threaded
  programs, according to your discretion.

- **Web-Assembly Compatibility**.

  This crate is compatible with wasm-targets and the browser environment in
  particular.

- **Source Code Formatters**.

  Lady Deirdre includes tools to develop code formatter programs that take into
  account code comments and blank lines.

- **Annotated Snippets**.

  The framework provides a configurable API to print source code snippets with
  syntax highlighting and annotations to the terminal, intended to display
  syntax and semantic errors in the codebase.

- **Self-Sufficient API**.

  The crate offers a self-sufficient, extendable, and highly configurable API
  for developing the front-end part of programming language compilers and code
  editor language extensions. As a foundation technology, Lady Deirdre does not
  have any third-party dependencies except for the Rust standard library and
  the macros crate.

## Performance

Lady Deirdre aims to provide development infrastructure with acceptable
computational performance suitable for production use.

The crate's API demonstrates solid benchmark test results, comparing individual
features of the framework with specialized solutions from the Rust ecosystem.

For detailed information, refer to the [Benchmarks page](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/tests).

## Links

- [Source Code](https://github.com/Eliah-Lakhin/lady-deirdre)
- [Main Crate](https://crates.io/crates/lady-deirdre)
- [API Documentation](https://docs.rs/lady-deirdre)
- [User Guide](https://lady-deirdre.lakhin.com/)
- [Examples](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples)
- [License Agreement](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md)

## Copyright

This work is proprietary software with source-available code.

To copy, use, distribute, or contribute to this work, you must agree to the
terms and conditions of the [General License Agreement](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md).

For an explanation of the licensing terms, see the
[F.A.Q.](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/FAQ.md)

Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин). All rights reserved.
