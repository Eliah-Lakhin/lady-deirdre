# Lady Deirdre.

[![Lady Deirdre Main Crate API Docs](https://img.shields.io/docsrs/lady-deirdre?label=Main%20Docs)](https://docs.rs/lady-deirdre)
[![Lady Deirdre Macro Crate API Docs](https://img.shields.io/docsrs/lady-deirdre-derive?label=Macro%20Docs)](https://docs.rs/lady-deirdre-derive)
[![Lady Deirdre Main Crate](https://img.shields.io/crates/v/lady-deirdre?label=Main%20Crate)](https://crates.io/crates/lady-deirdre)
[![Lady Deirdre Macro Crate](https://img.shields.io/crates/v/lady-deirdre-derive?label=Macro%20Crate)](https://crates.io/crates/lady-deirdre-derive)

Compiler front-end foundation technology.

If you want to create your own programming language with IDE support from
day one, or if you are going to develop new IDE from scratch, or a programming
language LSP plugin, this Technology is for you!

Lady Deirdre provides a framework to develop Lexical Scanner, Syntax Parser and
Semantic Analyser that could work in live coding environment applying
user-input changes incrementally to all underlying data structures.

This Technology represents a set of essential instruments to develop modern
programming language compilers with seamless IDE integration.

**Features**:

 - Written in Rust entirely.
 - Derive-macros to define PL Grammar directly on Enum types.
 - Smart error recovery system out of the box.
 - Dependency-free no-std ready API.
 - Works faster than Tree Sitter.

**Links:**
 - [Main Crate API Documentation](https://docs.rs/lady-deirdre).
 - [Macro Crate API Documentation](https://docs.rs/lady-deirdre-derive).
 - [Repository](https://github.com/Eliah-Lakhin/lady-deirdre).
 - [Examples, Tests, Benchmarks](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples).
 - [End User License Agreement](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md).

**This Work is a proprietary software with source available code.**

To copy, use, distribute, and contribute into this Work you must agree to
the terms of the
[End User License Agreement](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md).

The Agreement let you use this Work in commercial and non-commercial purposes.
Commercial use of the Work is free of charge to start, but the Agreement
obligates you to pay me royalties under certain conditions.

If you want to contribute into the source code of this Work, the Agreement
obligates you to assign me all exclusive rights to the Derivative Work made by
you (this includes GitHub forks and pull requests to my repository).

The Agreement does not limit rights of the third party software developers as
long as the third party software uses public API of this Work only, and the
third party software does not incorporate or distribute this Work directly.

If you do not or cannot agree to the terms of this Agreement, do not use
this Work.

Copyright (c) 2022 Ilya Lakhin (Илья Александрович Лахин). All rights reserved.

# Macro Crate API Documentation.

This Crate provides two optional companion macros to the
[`Main Crate`](https://docs.rs/lady-deirdre) to construct
Lexis Scanner and Syntax Parser using derive Rust syntax on enum types.

The
[Token](https://docs.rs/lady-deirdre-derive/latest/lady_deirdre_derive/derive.Token.html)
macro constructs a Lexical Scanner through the set of user-defined regular
expressions specified directly on enum variants using macro-attributes.
And the
[Node](https://docs.rs/lady-deirdre-derive/latest/lady_deirdre_derive/derive.Node.html)
macro, in turn, constructs a Syntax Parser through the set of
user-defined LL(1) grammar rules over the Token variants.

Both macros implement
[Token](https://docs.rs/lady-deirdre/latest/lady_deirdre/lexis/trait.Token.html)
and
[Node](https://docs.rs/lady-deirdre/latest/lady_deirdre/syntax/trait.Node.html)
traits accordingly, and considered to be the primary recommended way to define
Programming Language grammar.
