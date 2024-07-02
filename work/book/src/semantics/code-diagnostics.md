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

# Code Diagnostics

While most end attributes of the semantic graph aim to infer specific semantic
facts about particular syntax tree nodes, code diagnostics (semantic errors and
warnings) are intended to be collected from the entire syntax tree.

To tackle this issue and improve the incremental nature of code diagnostics, you
can gather local diagnostic messages within scopes by iterating through scope
nodes and their attributes, potentially containing diagnostic issues. These
issues can then be collected into the hash set of the scope's diagnostics
attribute.

Subsequently, in the root node's global diagnostics attribute, you can iterate
through all local diagnostic attributes of scopes and aggregate their values
into a single set, wrapping it into
a [Shared](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/sync/struct.Shared.html)
structure for efficient cloning. Furthermore, you can enhance the final
diagnostics set with syntax errors from the normal compilation unit by directly
reading them from the document[^syntaxerror].

The resulting global diagnostics attribute would indirectly depend on the
majority of the semantic graph. Despite potential optimizations by the validator
due to granularity, querying this attribute could still be computationally
intensive in edge cases. To mitigate this, the language server could
periodically examine this attribute with a low-priority analysis task.

Moreover, when utilizing
the [Attr::snapshot](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/analysis/struct.Attr.html#method.snapshot)
function to retrieve a copy of the current diagnostics sets, you can leverage
the version number of the attribute value to determine whether this set needs to
be republished to the client.

[^syntaxerror]: The [Document::errors](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/trait.SyntaxTree.html#method.errors)
function would provide you with an iterator over all syntax errors within the
compilation unit.
