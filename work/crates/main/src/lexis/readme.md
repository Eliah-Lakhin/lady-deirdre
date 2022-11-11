# Lexis analysis features.

This module provides functionality to organize the source code lexis analysis
system.

The source code is a string of UTF-8 characters. This text builds up a sequence
of substrings of usually small sizes called Tokens. For example, "foo bar" is a
text consists of three tokens: an Identifier "foo", a Whitespace " ", and an
Identifier "bar". Splitting the source code text into token substrings and
associating them with lexical metadata is a Lexical analysis stage of
Compilation System.

An API user encouraged to implement a [Token](crate::lexis::Token) trait
on the Rust enum types. The variants of this enum would represent token types.
And the [`Token::new`](crate::lexis::Token::new) function would define
a Programming Language lexical grammar parser, an algorithm that divides the
source text into tokens. Under the hood this function performs lexical
parsing of the UTF-8 text by interacting with the low-level
[LexisSession](crate::lexis::LexisSession) interface.

Normally an API user does not need to implement Token interface manually. You
can utilize [Token](::lady_deirdre_derive::Token) derive macro instead to
specify lexical grammar of the PL directly on the enum variants through the
macro attributes.

Individual Token metadata called [Chunk](crate::lexis::Chunk) consists of four
fields:
 - An instance of the Token type that represents a group of tokens this
   particular "token" belongs too. This field, in particular, is supposed to be
   used on the further lexical and syntax analysis stages. The Token type could
   also contain additional semantic metadata.
 - An absolute UTF-8 character index of the first character of the token string
   inside the original source code text. This "index" called
   [Site](crate::lexis::Site).
 - A [Length](crate::lexis::Length) of the token's string. This is the number
   of the UTF-8 characters in the string.
 - A UTF-8 token [String](std::string::String). This is a substring of the
   original source text that was scanned by the lexical parser to recognize
   corresponding token.

Objects that store a source code lexical structure implement a
[SourceCode](crate::lexis::SourceCode) trait. This trait provides functions to
access and to inspect lexical structure such as individual Tokens, token Chunks,
and to dereference weak references of the Tokens and the token Chunk fields.
Unless you work on a Crate extension, you don't need to implement this trait
manually.

The default implementation of the SourceCode is a
[TokenBuffer](crate::lexis::TokenBuffer) object. This object provides an
efficient way to load and lexically parse of the text loaded from file, and
is supposed to be either used directly for non-incremental compilation mode, or
to be further turned into a [Document](crate::Document) incremental storage.

To traverse individual tokens of the source code, the SourceCode trait provides
a [`SourceCode::cursor`](crate::lexis::SourceCode::cursor) function that returns
a low-level iterator-alike interface over the token metadata called
[TokenCursor](crate::lexis::TokenCursor).

To inspect particular features of the source code content such as arbitrary
substrings or to iterate token Chunks in a more convenient way, an API user
encouraged to use a higher-level [CodeContent](crate::lexis::CodeContent)
extension interface. This interface is auto-implemented for all SourceCode
implementations such as TokenBuffer or Document.

To index into arbitrary characters of the source code text characters, the
module provides a low-level [ToSite](crate::lexis::ToSite) trait. This trait
was designed to transform custom index objects to the source code character
Sites. ToSite is implemented for the [Site](crate::lexis::Site) type(a [usize]
UTF-8 character absolute index) itself, but is also implemented for
[Position](crate::lexis::Position) object that holds a text index in terms of
the source code lines and columns, and is implemented for
the [SiteRef](crate::lexis::SiteRef) source code changes history independent
weak reference.

Depending on the end compilation system design needs you can implement this
trait manually for custom indexing objects.

To specify arbitrary spans of the source code text to be indexed to, the module
provides a low-level [ToSpan](crate::lexis::ToSpan) trait. This interface is
auto-implemented for all types of the Rust standard range types(such as
[Range](::std::ops::Range) or [RangeTo](::std::ops::RangeTo)) over the ToSite
objects. As such an API user can specify, for example, a span in form of the
Site range `8..12`, or using Position objects
`Position::new(3, 5)..=Position::new(12, 1)`.

The end incremental compilation system is supposed to resolve semantic
information in lazy changes-independent fashion. For this purpose an API user
encouraged to utilize weak references into the source code chunks metadata.
This module provides two high-level API interfaces for such references:
the [TokenRef](crate::lexis::TokenRef) reference object to index particular
tokens and its chunk fields, and the [SiteRef](crate::lexis::SiteRef)
reference object to index particular Sites inside the source code text.

See [Arena](crate::arena) module documentation to read more about the weak
reference system. 
