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

# Hand-Written Parsers

The following chapters cover more advanced topics, providing an in-depth
exploration of the anatomy of Lady Deirdre's parsers. They will guide you on how
to override parsers generated by the Node macro with manually implemented parse
functions.

One common case where you might want to implement the parse procedure manually
is infix expression parsing. Infix expressions usually require left recursion,
which cannot be directly expressed in terms of LL(1) grammars.

These chapters will guide you through
the [Expr Parser](https://github.com/Eliah-Lakhin/lady-deirdre/tree/f350aaed30373a67694c3aba4d2cfd9874c2a656/work/crates/examples/src/expr_parser)
example. This example demonstrates how to parse boolean expressions
(e.g., `(true | false) & true`) using the Pratt algorithm.
