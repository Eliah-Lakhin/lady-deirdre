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

# Pretty Printer

The pretty printer object operates in a syntax-unaware manner, meaning it
doesn't consider the meaning of the original grammar tokens and nodes. Instead,
it deals with its own system of abstract tokens that shape the output in terms
of string words, blanks between words, and word groups only.

The objective of the printer is to interpret each incoming blank token as either
a whitespace or a line break, depending on the current line length and the
preconfigured maximum line length. If the printer determines to break the line
at the location of the blank token, it will subsequently indent or dedent the
following lines in accordance with the nesting of the groups.

## Printing Words

The [PrettyPrinter::word](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.PrettyPrinter.html#method.word)
function inputs a string word token into the printer, which will be printed on
the current line of the output, thus increasing the line length.

You invoke this function whenever encountering a parse tree token with content,
such as a keyword, identifier, or anything else besides whitespace or a line
break.

Additionally, you can call this function with a whitespace string to instruct
the printer to preserve the whitespace on the line regardless of the current
line length. This is useful, for instance, for comments' inner content or source
code string literals. However, it's not recommended to use this function to
forcibly break lines. In such cases, you should use the *hardbreak* function.

## Blank Tokens

To separate the words in the output of the printer, you utilize one of the
pretty printer's "blank" token functions:

- [PrettyPrinter::blank](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.PrettyPrinter.html#method.blank)
  is the default blank token, interpreted either as a single whitespace or a
  line break. You call this function when the next word should be separated from
  the previous one, possibly with a line break, depending on the printing
  algorithm's decision.

- [PrettyPrinter::hardbreak](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.PrettyPrinter.html#method.hardbreak)
  is a blank token that enforces the printer to always interpret it as a line
  break. You call this function, for instance, when printing the inner content
  of multi-line comments, as the structure of the comment's text typically needs
  to be preserved.

- [PrettyPrinter::softbreak](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.PrettyPrinter.html#method.softbreak)
  is similar to the *blank* function, but if the printer's algorithm decides to
  preserve the next token on the line, it does not insert whitespace. This
  function is useful, for example, to delimit the `.` dot tokens in a
  call-chain (`foo.bar`). In such cases, you can insert a *softbreak* token
  before the dot but omit the delimiter after the dot word

## Word Groups

During the formatting process, when the current content exceeds the maximum line
length, the algorithm attempts to break the content into lines by interpreting
blank tokens as line breaks.

At this point, the algorithm can either interpret all blank tokens as line
breaks, resulting in consistent line splitting, or it can selectively interpret
some blank tokens, maximizing the utilization of line space.

Consistent line splitting is preferable for source code blocks, such as JSON
objects.

```text
{
    "foo": 123,
    "bar": 456,
    "baz": 789
}
```

For enumerations of simple items (e.g., JSON arrays), inconsistent breaking is
more suitable.

```text
[123, 456, 789, 1011, 1213, 1516, 1718, 1920, 2122,
    2324, 2526, 2728]
```

Content breaking occurs within word groups. To initiate a new group, you use
either [PrettyPrinter::cbox](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.PrettyPrinter.html#method.cbox)
or [PrettyPrinter::ibox](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.PrettyPrinter.html#method.ibox).
The former
begins a consistent word group, while the latter starts an inconsistent group.

Each group must be closed by
calling [PrettyPrinter::end](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.PrettyPrinter.html#method.end).

Both *cbox* and *ibox* functions accept indentation level shifting for the
group, represented by a signed integer. Positive values increase the inner
content's indentation, negative values decrease it (with zero having no effect).
When the printer breaks the content inside the group, each new line is indented
with whitespace according to the current indentation level.

## Overriding Indentations

You can manually adjust line indentation by calling
the [PrettyPrinter::indent](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.PrettyPrinter.html#method.indent)
function **immediately after** submitting any of the blank tokens. If the
algorithm interprets the submitted token as a line break, the next line, as well
as all subsequent lines, will be shifted accordingly.

## Keeping Content In Line

In general, the algorithm aims to break lines as early as possible so that
parental word groups are split by lines, while leaf groups remain in line.

```text
{
    "foo": [123, 456, 789, 1011, 1213, 1516, 1718, 1920, 2122]
}
```

This approach is generally suitable for most practical use cases. However, there
are situations where it's preferable to keep the parental content aligned in
line and splitting of the nested groups instead.

In such cases, you can utilize
the [PrettyPrinter::neverbreak](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.PrettyPrinter.html#method.neverbreak)
function, which instructs the printer to reset the current line length counter
to zero. Consequently, the algorithm assumes that the previously submitted text
fits on the line, and begins splitting from the subsequent nested submissions.

```text
{ "foo": [123, 456, 789, 1011, 1213, 1516, 1718,
    1920, 2122] }
```

## Trailing Commas

Depending on the language grammar, some languages allow leaving a trailing comma
at the end of lists (e.g., Rust and JavaScript, but not JSON). This ensures
better readability when the list is split into multiple lines, as the last item
receives a trailing comma, but the comma is omitted if the content remains in a
single line.

This formatting rule depends on whether the algorithm decides to insert a line
break at the blank token. To address this, you can use
the [PrettyPrinter::pre_break](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.PrettyPrinter.html#method.pre_break)
and [PrettyPrinter::pre_space](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/format/struct.PrettyPrinter.html#method.pre_space)
functions to configure the preceding blank token.

Both functions must be called **immediately after** submitting the blank token.
The first function, *pre_break*, specifies a word that will be inserted before
the line break (at the end of the previous line), while the second function,
*pre_space*, inserts the specified word otherwise (the word will appear before
the whitespace when paired with the *blank* function).

When your program formats a comma-separated list, you can insert regular
`,` commas after each intermediary item and a normal blank token after each
comma. At the end of the list, after the last submitted item, you can submit a
*softbreak* token and configure it with `pre_break(',')`, ensuring that if this
trailing blank token receives a line break, the last line of the list will be
appended with a comma.
