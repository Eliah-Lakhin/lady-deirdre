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

# Main Crate API Documentation.

This is the Main API Crate of the Lady Deirdre Technology.

This Crate together with the
[`Macro Crate`](https://docs.rs/lady-deirdre-derive) provide sufficient set of
tools to construct incremental compilation system of a Programming Language.

## Architecture overview.

### Programming Language Grammar.

The Technology deals with syntax and lexis grammar analysis of the language
independently. An API user should define both levels of the grammar separately,
or to define just the lexical grammar bypassing syntax parsing stage. The
[`Macro Crate`](https://docs.rs/lady-deirdre-derive) provides two derive macros
to define lexis and syntax on the custom enum types by specifying parsing rules
on the enum variants directly through the macro attributes. Alternatively, you
can implement syntax and/or lexis parsers manually by implementing corresponding
trait functions.

The [Lexis module](crate::lexis) contains everything related to the Lexical
analysis, and the [Syntax module](crate::syntax), in turn, contains everything
related to the syntax analysis.

### Parsing Process.

The parsing process(of lexis and syntax grammars both) is driven by two loosely
coupled API layers that interact to each other: one layer is responsible for
input read and output write operations, but does not know anything about the
grammar, and another layer performs actual parsing of the local parts
of the input data. An API user can customize any of these layers depending on
the end compilation system design needs. The Crate API provides default
implementations of these layers that would cover most of the practical use
cases.

For example, a [LexisSession](crate::lexis::LexisSession) trait is a cursor
over the source code input data of the lexical parsing stage. It's
opposite object is a [Token](crate::lexis::Token) trait that implements actual
lexical scanner for particular programming language. The
[Token::new](crate::lexis::Token::new) function accepts a reference to the
LexisSession, reads some data from the session moving session's internal cursor
forward, and in the end returns an instance of a single token parsed by the
scanner. An API user normally don't need to implement LexisSession trait
manually, unless you are working on the crate's API extension. For example,
the [Document](crate::Document) object under the hood has its own implementation
of this trait that is properly interacting with the Document internal data.
Usually an API user needs to implement a Token trait
only(on the enum type) to specify PL's lexical grammar. The user is encouraged
to do so using corresponding [Token](::lady_deirdre_derive::Token) derive
macro, or, in some unusual cases, you can implement this trait manually too.

### Incremental Reparsing and Error Recovery.

The Crate provides objects to parse and to store parsed data in incremental and
non-incremental ways. The [Document](crate::Document) object is one of such
objects that provides incremental reparsing capabilities. The Document instance
caches parsed data, and is capable to continue parsing
process from any random point where the end user wants to update the source code
text.

In particular, you can use this object to represent a code editor's opened file.

Parsing infrastructure is resilient to the source code errors. The parsing
process is able to recover from errors efficiently reconstructing and always
keeping syntax tree up to date.

### Data Structures.

Finally, the Technology utilizes a concept of the versioned arena
memory management to provide a framework to organize such data structures as
directional graphs where the nodes of the graph reside in a common arena memory.
The nodes refer each other through the weak type and lifetime independent
references into this arena. In particular this framework used by the Crate to
organize mutable Syntax Tree data structure that, depending on the end
compilation system design, could serve as an Abstract Syntax Tree, and could
serve as a semantic resolution data structure as well. Read more about this
concept in the [Arena module documentation](crate::arena).

## Tutorial.

This Tutorial demonstrates how to implement a parser and an interpreter of a
simple calculator language with
[S-expressions](https://en.wikipedia.org/wiki/S-expression) syntax.

For the sake of simplicity this calculator allows Sum and Mult operations
on integer values and their combinations only.

  - `(123)` resolves to the `123`.
  - `(+ 5, 10, 4)` resolves to `19`.
  - `(* 5, 10)` resolves to `50`.
  - `(* (+ 7, 2), (+ 4, 8))` resolves to `108`.

```rust
// First of all we need to define a programming language lexical grammar, and
// a data structure type to store individual token instances.

// Token is an enum type with variants representing token types.
// Lexis parsing rules specified through the regular expressions on these
// variants. A Token macro derive compiles these regular expressions, and
// implements a Token trait that, in turn, implements a lexical scanner under
// the hood.

use lady_deirdre::lexis::Token;

#[derive(Token, Debug, PartialEq)]
enum CalcToken {
    #[rule("(")]
    Open,

    #[rule(")")]
    Close,

    #[rule("+")]
    Plus,

    #[rule("*")]
    Mult,

    #[rule(",")]
    Comma,

    #[rule(['1'..'9'] & ['0'..'9']* | '0')]
    #[constructor(parse_num)] // This variant contains a custom field,
                              // so we need a dedicated constructor.
    Num(usize),

    // Any `char::is_ascii_whitespace()` character.
    #[rule([' ', '\t', '\n', '\x0c', '\r']+)]
    Whitespace,

    // The lexer sinks all unrecognizable tokens into this special kind of
    // "mismatch" token.
    #[mismatch]
    Mismatch,
}

impl CalcToken {
    fn parse_num(input: &str) -> Self {
        Self::Num(input.parse().unwrap())
    }
}

// Lets try our lexer.

// To test the lexer we need to load the source code into a SourceCode
// storage. We are going to use a Document object which is an incremental
// storage.

// Since we did not define the syntax grammar yet, we are going to use
// a special type of grammar called "NoSyntax" that bypasses syntax parsing
// stage.

use lady_deirdre::{Document, syntax::NoSyntax};

let mut doc = Document::<NoSyntax<CalcToken>>::default();

// Document is an incremental storage with random write access operations.
// Filling the entire document(specified by the `..` span range) with initial
// text.
doc.write(.., "(+ 5, 10)");

// Now lets check our tokens using chunk iterator.

use lady_deirdre::lexis::CodeContent;

assert_eq!(
    doc.chunks(..).map(|chunk| chunk.token).collect::<Vec<_>>(),
    vec![
        &CalcToken::Open,
        &CalcToken::Plus,
        &CalcToken::Whitespace,
        &CalcToken::Num(5),
        &CalcToken::Comma,
        &CalcToken::Whitespace,
        &CalcToken::Num(10),
        &CalcToken::Close,
    ],
);

// Now lets define our syntax parser.

// Similarly to Token, we are going to define a Syntax Tree node type as a Rust
// enum type with LL(1) grammar rules directly on the enum variants.

use lady_deirdre::{
    syntax::{Node, SyntaxError, NodeRef, SyntaxTree},
    lexis::TokenRef,
};

#[derive(Node)]
#[token(CalcToken)] // We need to specify a Token type explicitly.
#[error(SyntaxError)] // An object that will store syntax errors.
                      // SyntaxError is the default implement, but you can use
                      // any custom type that implements From<SyntaxError>.
#[skip($Whitespace)] // Tokens to be ignored in the syntax rule expressions.
enum CalcNode {
    #[root] // The entry-point Rule of the Syntax Tree root node.
    #[rule(expr: Expression)]
    Root {
        // A weak reference to the top captured Expression node.
        expr: NodeRef,
    },

    #[rule(
        $Open
        & operator: ($Plus | $Mult)?
        & (operands: (Number | Expression))+{$Comma}
        & $Close
    )]
    #[synchronization] // "synchronization" directive tells the parse to recover
                       // from syntax errors through balancing of to the "(" and
                       // ")" tokens.
    Expression {
        operator: TokenRef,
        operands: Vec<NodeRef>,
    },

    #[rule(value: $Num)]
    Number { value: TokenRef }
}

// A helper function that prints syntax structure for debugging purposes.
fn show_tree(node_ref: &NodeRef, tree: &Document<CalcNode>) -> String {
    // Turns weak NodeRef reference to Node strong reference using the
    // SyntaxTree instance.
    let node = match node_ref.deref(tree) {
        Some(node) => node,
        // If referred node does not exist in the syntax
        // tree(e.g. due to syntax errors) returning the
        // "?" string.
        None => return String::from("?"),
    };

    match node {
        CalcNode::Root { expr } => format!("Root({})", show_tree(expr, tree)),

        CalcNode::Expression { operator, operands } => {
            let mut result = String::new();

            match operator.deref(tree) {
                Some(CalcToken::Plus) => result.push_str("Plus("),
                Some(CalcToken::Mult) => result.push_str("Mult("),
                Some(_) => unreachable!(),
                None => result.push_str("?("),
            }

            result.push_str(
                operands
                    .iter()
                    .map(|op| show_tree(op, tree))
                    .collect::<Vec<_>>()
                    .join(", ")
                    .as_str(),
            );

            result.push_str(")");

            result
        }

        CalcNode::Number { value } => {
            match value.deref(tree) {
                Some(CalcToken::Num(num)) => num.to_string(),
                Some(_) => unreachable!(),
                None => String::from("?"),
            }
        }
    }
}

// Lets try to run our grammar again. This time with the syntax parser.

let mut doc = Document::<CalcNode>::default();

doc.write(.., "(* (+ 3, 4, 5), 10)");

assert_eq!(show_tree(doc.root(), &doc), "Root(Mult(Plus(3, 4, 5), 10))");

// Now, lets implement an interpreter of our expression language by traversing 
// the Syntax Tree.

fn interpret(node_ref: &NodeRef, doc: &Document<CalcNode>) -> usize {
    let node = match node_ref.deref(doc) {
        Some(node) => node,
        None => return 0,
    };
    
    match node {
        CalcNode::Root { expr } => interpret(expr, doc),
        
        CalcNode::Expression { operator, operands } => {
            match operator.deref(doc) {
                Some(CalcToken::Mult) => {
                    let mut result = 1;

                    for operand in operands {
                        result *= interpret(operand, doc);
                    }

                    result
                }

                Some(CalcToken::Plus) => {
                    let mut result = 0;

                    for operand in operands {
                        result += interpret(operand, doc);
                    }

                    result
                }
                
                Some(_) => unreachable!(),
                
                None => 0,
            }
        }

        CalcNode::Number { value } => {
            match value.deref(doc) {
                Some(CalcToken::Num(num)) => *num,
                Some(_) => unreachable!(),
                None => 0,
            }
        }
    }
}

assert_eq!(interpret(doc.root(), &doc), 120);
```
