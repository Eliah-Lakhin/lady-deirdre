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

# Side Effects

Typically, implementations of
attribute's [Computable](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/analysis/trait.Computable.html)
functions should be free of side effects: their results should not rely on the
external environment state, and non-input attributes should be independent from
changes in the syntax and lexical structure of the compilation units.

If the implementation has side effects that cannot be avoided, you have two ways
to overcome the limitations of the validation procedure:

1. You can invalidate any attribute manually using
   the [Attr::invalidate](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/analysis/struct.Attr.html#method.invalidate)
   function if you are aware that the external environment state has changed.

2. Inside the computable function implementation, you can use
   the [Context::subscribe](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/analysis/struct.AttrContext.html#method.subscribe)
   function to subscribe this attribute to the Analyzer-wide event that could be
   triggered independently for bulk invalidation of the semantic graph
   attributes subscribed to a specific event. The event object that you would
   pass to this function is an arbitrary user-defined value of a numeric
   type[^builtinevenets].

Both methods should be used conservatively as they could potentially impact the
incremental capabilities of the framework.

However, one scenario where you might find these mechanisms useful is when your
compiler manages several Analyzers of distinct programming languages that
logically build up a single compilation project. Within this setup, changes in
the state of one Analyzer could be propagated to some attributes of another
Analyzer's setup.

[^builtinevenets]: There are a couple of built-in events as well, such as
the [DOC_UPDATED_EVENT](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/analysis/constant.DOC_UPDATED_EVENT.html),
which denotes document-wide edits within the specified document regardless of
the scopes. However, the majority of the value range is available for
user-defined events.
