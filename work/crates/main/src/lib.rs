////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

//TODO check warnings regularly
#![allow(warnings)]
#![allow(unused_unsafe)]
#![deny(missing_docs)]

//! # Lady Deirdre API Documentation
//!
//! Lady Deirdre is a framework for incremental programming language compilers,
//! interpreters, and source code analyzers.
//!
//! This documentation provides formal API descriptions. For a general
//! exploration of Lady Deirdre's usage, please refer to the [User Guide](https://lady-deirdre.lakhin.com/).
//!
//! ## Getting Started
//!
//! To start using Lady Deirdre, add this crate to the dependencies in your
//! project's Cargo.toml file:
//!
//! ```toml
//! [dependencies.lady-deirdre]
//! version = "2.0"
//! ```
//!
//! This crate does not have any configuration features or third-party
//! dependencies, except for the Rust standard library and the accompanying
//! macro derive crate. Therefore, no additional preparations are needed.
//!
//! ## Web Assembly Builds
//!
//! The crate can compile and run under WebAssembly targets (including the
//! `wasm32-unknown-unknown` target) without any extra preparations or setups.
//!
//! ## Links
//!
//! - [Source Code](https://github.com/Eliah-Lakhin/lady-deirdre)
//! - [Main Crate](https://crates.io/crates/lady-deirdre)
//! - [API Documentation](https://docs.rs/lady-deirdre)
//! - [User Guide](https://lady-deirdre.lakhin.com/)
//! - [Examples](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples)
//! - [License Agreement](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md)
//!
//! ## Copyright
//!
//! This work is proprietary software with source-available code.
//!
//! To copy, use, distribute, or contribute to this work, you must agree to the
//! terms and conditions of the [General License Agreement](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md).
//!
//! For an explanation of the licensing terms, see the
//! [F.A.Q.](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/FAQ.md)
//!
//! Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин). All rights reserved.

/// Semantic analysis framework.
///
/// This framework enables arbitrary on-demand incremental computations over
/// the set of documents that build up a single compilation project.
///
/// The API design of this module mimics Reference Attributed Grammars but uses
/// the incremental computation algorithm similar to the one used in
/// [Salsa](https://github.com/salsa-rs/salsa).
///
/// You can find the detailed specification of the framework features under the
/// [Analyzer](analysis::Analyzer) object documentation. This object is an
/// entry point of the framework.
pub mod analysis;

/// Memory management utilities.
///
/// The primary object of interest is the [Repo](arena::Repo) ("repository").
///
/// Repository is a storage of values in the memory allocation. Each value
/// is associated with a unique key called [Entry](arena::Entry). Keys management
/// is on the repository side. Insertion, deletion, and borrowing a value from
/// the Repo by key is a fast O(1) operation.
///
/// Repositories are of particular interest for the compilation units, which use
/// them to store individual syntax tree nodes and the source code tokens
/// metadata.
///
/// You can use the [Repo](arena::Repo) object for various purposes depending
/// on the needs.
///
/// Additionally, the arena module has an [Id](arena::Id) object, instances of
/// which are globally unique (within the current process) and identify
/// individual compilation units.
pub mod arena;

/// Tools for source code formatting and printing.
///
/// The [PrettyPrinter](format::PrettyPrinter) objects provides a set
/// of features to implement source code formatters.
///
/// The [Snippet](format::Snippet) is a configurable interface for printing
/// source code snippets with syntax highlighting and annotated fragments into
/// the terminal. This interface is useful for printing compiler's
/// syntax errors.
///
/// Finally, this module provides a set of features to stylize terminal strings
/// within the [TerminalString](format::TerminalString) trait and related
/// components.
pub mod format;

/// Building blocks of the lexical structure of compilation units.
///
/// This module provides interfaces to describe the lexical component of
/// your programming language grammar and to access the source code tokens.
///
/// - The [Token](lexis::Token) trait implements a lexical scanner of
///   a particular programming language, and describes individual lexical
///   tokens.
/// - The [SourceCode](lexis::SourceCode) trait provides access to individual
///   tokens, source code text, and other metadata related to the lexical
///   structure of the compilation unit.
/// - The [TokenBuffer](lexis::TokenBuffer) object is the default implementation
///   of the SourceCode. It is capable to scan an ongoing text stream using
///   specified Token scanner.
///
/// A detailed specification of the lexical scanning process can be found
/// under the [LexisSession](lexis::LexisSession) trait.
pub mod lexis;

/// Synchronization primitives useful for compilers.
///
/// These primitives enrich the set of  [std::sync] objects, which you may find
/// useful when implementing a compiler or a language analyzer intended to work
/// in a multi-thread environment:
///
///  - [Shared](sync::Shared) is like an [Arc](std::sync::Arc), but without
///    the weak counterpart.
///  - [Lazy](sync::Lazy) is like a Lazy interface from the once_cell crate,
///    but is built on the recently stabilized [std::sync::OnceLock].
///  - [Table](sync::Table) is a sharded read-write lock of the hash map.
///    This object is similar to the DashMap from the dashmap crate.
///  - [Trigger](sync::Trigger) is an atomic bool flag that you can set only
///    once to signalize a task to gracefully finish its job.
pub mod sync;

/// Building blocks of the syntax structure of compilation units.
///
/// This module provides interfaces to describe the syntax component of
/// your programming language grammar and to traverse the syntax trees.
///
/// - The [Node](syntax::Node) trait describes the syntax tree's node structure
///   and provides a parser function for individual nodes.
/// - The [SyntaxTree](syntax::SyntaxTree) trait provides access to individual
///   nodes and parse errors, and has functions to traverse the syntax tree.
/// - The [ImmutableSyntaxTree](syntax::ImmutableSyntaxTree) object is
///   the default implementation of the SyntaxTree trait. It is capable of
///   parsing the syntax tree only once during creation and does not provide
///   incremental reparsing features.
///
/// A detailed specification of the syntax parsing process can be found
/// under the [SyntaxSession](syntax::SyntaxSession) trait.
pub mod syntax;

/// A set of objects to manage individual compilation units in memory.
///
/// The primary object of interest is the [Document](units::Document).
///
/// This object contains the content of a single file within your compilation
/// project and offers methods to read the lexical and syntax structure of
/// the content, as well as methods to apply user-input changes to the file.
/// The Document automatically reparses all incoming changes and ensures that
/// the lexical and syntax structure remains up to date with its text content.
///
/// Additionally, the Document provides interfaces to track changes in the
/// content structure.
///
/// The remaining module interfaces serve as the building blocks of
/// the Document, which you can utilize independently depending on your needs.
pub mod units;

mod mem;
mod report;

extern crate self as lady_deirdre;
