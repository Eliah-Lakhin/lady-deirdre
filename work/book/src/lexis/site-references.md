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

# Site References

The [Site](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/type.Site.html)
index, which represents the absolute offset of a Unicode character in the source
code text, cannot reliably address a token's absolute offset after source code
edits. This is because the token could be shifted left or right, or it could
disappear during incremental rescanning, depending on the bounds of the edit.

In contrast,
[TokenRef::site](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.TokenRef.html#method.site)
returns the absolute offset of the beginning of the token's string fragment at
the time of the call. In other words, this function returns an updated absolute
offset of the token after an edit operation, provided the incremental rescanner
did not remove the token during rescanning.

This allows for addressing a token's character bounds relative to changes in the
source code.

The [SiteRef](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.SiteRef.html)
helper object (backed by the TokenRef under the hood) addresses token bounds.
Specifically, this object addresses either the beginning of the token or the end
of the source code.

[ToSite](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.ToSite.html)
implements the ToSite trait, so it can be used as a valid bound of a range span.

```rust,noplayground
use lady_deirdre::{
    lexis::{SiteRef, SourceCode, TokenCursor},
    syntax::VoidSyntax,
    units::Document,
};

let mut doc = Document::<VoidSyntax<JsonToken>>::new_mutable("foo [bar] baz");

let brackets_start: SiteRef = doc.cursor(..).site_ref(2);
let brackets_end: SiteRef = doc.cursor(..).site_ref(5);

assert_eq!(doc.substring(brackets_start..brackets_end), "[bar]");

// Rewriting "bar" to "12345".
doc.write(5..8, "12345");

assert_eq!(doc.substring(brackets_start..brackets_end), "[12345]");
```

Similar to TokenRef, the SiteRef interface has a
special [nil](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.SiteRef.html#method.nil)
value and
the [is_nil](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/struct.SiteRef.html#method.is_nil)
test function.
