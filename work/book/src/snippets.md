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

# Snippets

When a compilation project has errors or warnings, it is usually more beneficial
for the end user to print source code snippets in the terminal, annotating the
fragments where the issues occur.

While there are several similar tools in the Rust ecosystem that you can use
with this crate, Lady Deirdre provides its own solution as well.

The [Snippet](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.Snippet.html)
is a configurable builder object that prints the source code text of a
compilation unit, or a part of it, with emphasized fragments annotated with
custom messages.

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

You create the builder in the Display or Debug context, providing the Document
(or any similar object with lexis, such as TokenBuffer) that needs to be
printed, and annotate arbitrary code spans with string messages.

Once building is finished, the Snippet prints the annotated snippet into the
Formatter's output.

The [Json Highlight](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/json_highlight/highlighter.rs#L45)
example demonstrates how to set up this builder on a custom object that wraps a
compilation unit document.

```rust,noplayground
pub struct JsonSnippet<'a> {
    pub doc: &'a Document<JsonNode>,
    pub annotation: Vec<(PositionSpan, AnnotationPriority, &'static str)>,
}

impl<'a> Display for JsonSnippet<'a> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        // You create the Snippet builder by calling the snippet function on
        // the Formatter. The specified parameter is a reference to the document
        // that contains the source code text.
        let mut snippet = formatter.snippet(self.doc);

        snippet
            // Configure the Snippet's header and footer text.
            // If omitted, the header and footer decorations will also be
            // omitted accordingly.
            .set_caption("Header text")
            .set_summary("Footer text.")
            // Specifies the highlighter that instructs the Snippet on how to
            // stylize individual tokens of the printed text.
            
            // This configuration is optional. When omitted, the Snippet will
            // print all tokens uniformly using the default color scheme.
            .set_highlighter(JsonHighlighter);

        for (span, priority, message) in &self.annotation {
            // Adds an annotated span to the builder.
            //
            // The Snippet will print the specified fragment with an inverted
            // foreground (using the foreground color specified by
            // the annotation priority).
            //
            // If a message is present (the message string is not empty),
            // the Snippet will print this message near the spanned fragment.
            //
            // If the Snippet has specified annotations, it will print only
            // the source code lines that contain annotated fragments
            // (regardless of whether they have a message), plus some lines that
            // surround these lines before and after.
            //
            // If the Snippet does not have any annotations, it will print
            // the entire source code text.
            snippet.annotate(span, *priority, *message);
        }

        // Finishes the builder and prints the snippet to the Formatter's output.
        snippet.finish()
    }
}
```

The Snippet has several drawing configuration options that you can specify using
the [Snippet::set_config](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.Snippet.html#method.set_config)
function. Here are a few:

- You can show or hide line numbers, header and footer, and the outer frame.
- You can enforce the Snippet to use ASCII-only drawing.
- You can disable all terminal styles so that the Snippet will be monochrome.

By default (if you don't provide the drawing config manually), the builder draws
the snippet with all drawing options turned off if the format is not
alternated (`format!("{}")`). Otherwise, all drawing options are
enabled (`format!("{:#}")`).

In the example above, we specify the JSON syntax highlighter using
the [Snippet::set_highlighter](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.Snippet.html#method.set_highlighter)
function.

The highlighter is a stateful object that implements
the [Highlighter](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/trait.Highlighter.html)
trait and instructs the Snippet on how to stylize the source code tokens. The
Snippet builder
calls [Highlighter::token_style](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/trait.Highlighter.html#tymethod.token_style)
for each token in the source code sequentially, and the function returns
the [Style](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.Style.html)
of the token.

```rust,noplayground
pub struct JsonHighlighter;

impl Highlighter<JsonToken> for JsonHighlighter {
    // The `dim` argument is set to true if this token is meant to have lesser
    // contrast than usual.
    //
    // The Snippet prefers to print the tokens outside of the annotated
    // fragments with lesser contrast to focus the user's attention on
    // the annotated spans.
    //
    // If the function returns None, it means that the token will be printed
    // without additional styles.
    fn token_style(&mut self, dim: bool, token: JsonToken) -> Option<Style> {
        match token {
            JsonToken::True | JsonToken::False | JsonToken::Null => Some(match dim {
                false => Style::new().blue(),
                true => Style::new().bright_blue(),
            }),

            JsonToken::String => Some(match dim {
                false => Style::new().green(),
                true => Style::new().bright_green(),
            }),

            JsonToken::BraceOpen | JsonToken::BraceClose => Some(Style::new().bold()),

            _ => None,
        }
    }
}
```

Since the highlighter is a stateful object, it can rely on previous tokens to
make a decision about the next token style. For example, if the highlighter
discovers that the token is part of a comment or a string literal context, it
can stylize this token accordingly.
