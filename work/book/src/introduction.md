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

# Introduction

<img align="right" style="width: 160px" alt="Lady Deirdre Logo" src="https://raw.githubusercontent.com/Eliah-Lakhin/lady-deirdre/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/logo.jpg" />

Lady Deirdre is a framework that helps you develop front-end code analysis
tools, such as code editor language extensions, programming language compilers
and interpreters, and even new code editors.

This guide will explain the main concepts of the API and walk you through the
steps of developing an analysis tool.

The book assumes that you already have some experience with the Rust programming
language and that you understand the core concepts of classical compilers:
lexical scanners, syntax parsers, regular expressions, context-free grammars,
etc.

If you have prior experience with code editor plugin development and
the [LSP](https://microsoft.github.io/language-server-protocol/) protocol in
particular, it will certainly help you understand the material but is not
strictly required. The book will provide you with a brief overview of
the core concepts behind these tools.

## Links

- [Source Code](https://github.com/Eliah-Lakhin/lady-deirdre)
- [Main Crate](https://crates.io/crates/lady-deirdre)
- [API Documentation](https://docs.rs/lady-deirdre)
- [User Guide](https://lady-deirdre.lakhin.com/)
- [Examples](https://github.com/Eliah-Lakhin/lady-deirdre/tree/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples)
- [License Agreement](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md)

## Copyright

This work is proprietary software with source-available code.

To copy, use, distribute, or contribute to this work, you must agree to the
terms and conditions of the [General License Agreement](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md).

For an explanation of the licensing terms, see the
[F.A.Q.](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/FAQ.md)

Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин). All rights reserved.
