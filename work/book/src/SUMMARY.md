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

- [Introduction](introduction.md)

- [Overview](overview.md)

- [Lexis](lexis/lexis.md)
    - [Lexical Grammar](lexis/lexical-grammar.md)
    - [Token Buffer](lexis/token-buffer.md)
    - [Source Code](lexis/source-code.md)
    - [Code Inspection](lexis/code-inspection.md)
    - [Token References](lexis/token-references.md)
    - [Site References](lexis/site-references.md)

- [Syntax](syntax/syntax.md)
    - [Syntax Grammar](syntax/syntax-grammar.md)
    - [Error Recovering](syntax/error-recovering.md)
    - [Debugging](syntax/debugging.md)
    - [Syntax Tree](syntax/syntax-tree.md)
    - [Node References](syntax/node-references.md)
    - [Tree Inspection](syntax/tree-inspection.md)
    - [Hand-Written Parsers](syntax/hand-written-parsers.md)
    - [Overriding a Parser](syntax/overriding-a-parser.md)
    - [Syntax Session](syntax/syntax-session.md)
    - [Pratt's Algorithm](syntax/pratts-algorithm.md)

- [Documents](documents.md)

- [Semantics](semantics/semantics.md)
    - [Partition Into Scopes](semantics/partition-into-scopes.md)
    - [Grammar Setup](semantics/grammar-setup.md)
    - [Semantic Graph](semantics/semantic-graph.md)
    - [Incremental Computations](semantics/incremental-computations.md)
    - [Side Effects](semantics/side-effects.md)
    - [Scope Access](semantics/scope-access.md)
    - [Granularity](semantics/granularity.md)
    - [The Analyzer](semantics/the-analyzer.md)
    - [Tasks Management](semantics/tasks-management.md)
    - [Multi-File Analysis](semantics/multi-file-analysis.md)
    - [Language Server Design](semantics/language-server-design.md)
    - [Configuration Issues](semantics/configuration-issues.md)
    - [Code Diagnostics](semantics/code-diagnostics.md)
    - [Tree Index](semantics/tree-index.md)

- [Code Formatters](code-formatters/code-formatters.md)
    - [Pretty Printer](code-formatters/pretty-printer.md)

- [Snippets](snippets.md)
