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

# Syntax Grammar

You define the syntax grammar using
the [Node derive macro](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/syntax/derive.Node.html)
on an arbitrary enum type that serves as the type for the syntax tree nodes.

Unlike the token enum, the node enum variants are required to have bodies with
fields. These fields allow the parser to store parent-child relationships
between nodes.

The node's parser is described in terms
of [LL(1)](https://en.wikipedia.org/wiki/LL_parser) grammars using
the `#[rule(...)]` macro attributes, which denote individual variant grammar
rules.

Within these regex-like parse expressions, you can refer to other variants,
establishing recursive descent parsing between the parse procedures.

Additionally, you can name any subexpression with the `field:` prefix inside the
expression. This syntax enforces the generated parser to capture the result of
the subexpression matching (whether it be a token or a syntax tree node) and
place the matching result into the variant's field with the same name.

This process is called *capturing*, and it allows the parser to establish the
node-to-children descending relationships between nodes.

The opposite ascending node-to-parent relationships are established
automatically if you declare a variant field with the `#[parent]` macro
attribute.

From the [JSON example](https://github.com/Eliah-Lakhin/lady-deirdre/blob/f350aaed30373a67694c3aba4d2cfd9874c2a656/work/crates/examples/src/json_grammar/syntax.rs):

```rust,noplayground

#[derive(Node)]
#[token(JsonToken)]
#[trivia($Whitespace)]
#[define(ANY = Object | Array | True | False | String | Number | Null)]
#[recovery(
    $BraceClose,
    $BracketClose,
    [$BraceOpen..$BraceClose],
    [$BracketOpen..$BracketClose],
)]
pub enum JsonNode {
    #[root]
    #[rule(object: Object)]
    Root {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        object: NodeRef,
    },

    #[rule(start: $BraceOpen (entries: Entry)*{$Comma} end: $BraceClose)]
    Object {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        entries: Vec<NodeRef>,
        #[child]
        end: TokenRef,
    },

    #[rule(key: String $Colon value: ANY)]
    Entry {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        key: NodeRef,
        #[child]
        value: NodeRef,
    },
    
    // ...

    #[rule(value: $String)]
    #[secondary]
    String {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        value: TokenRef,
    },
}
```

The Node macro generates an optimized and error-resistant syntax parser based on
the provided grammar rules. The macro allows you to replace individual
node-generated parsers with hand-written parsers, where you can implement custom
recursive-descent logic with potentially unlimited lookahead and left recursion.
Hand-written parsers will be discussed in more detail in the next chapters of
this guide.

## Macro API

In this chapter, I will intentionally omit some details, referring you to
the [macro documentation](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/syntax/derive.Node.html)
for a more verbose description of the available features, and to
the [JSON example](https://github.com/Eliah-Lakhin/lady-deirdre/blob/f350aaed30373a67694c3aba4d2cfd9874c2a656/work/crates/examples/src/json_grammar/syntax.rs)
as an example of a node implementation that utilizes most of the macro's
capabilities.

Some general points to note about the macro API are:

1. The `#[token(JsonToken)]` macro attribute specifies the type of the token.
   This attribute is required and denotes the alphabet of the parsing input.
2. The `#[trivia($Whitespace)]` macro attribute describes elements that you want
   to omit automatically in the variant parsers between matching tokens. Trivia
   is a normal parsing expression that will be repeated zero or more times
   between each token. Typically, this expression enumerates whitespace tokens
   and refers to the comment variants of the grammar. The trivia expression can
   be overridden for each parsable variant (e.g., comments and string parsers
   might have different trivia expressions).
3. There is exactly one enum variant annotated with the `#[root]` macro
   attribute. This variant denotes the syntax tree root and serves as the entry
   point of the grammar.
4. A variant field annotated with the `#[node]` attribute is a reference to the
   current node[^noderef].
5. A variant field with the `#[parent]` attribute is a reference to the parent
   node of the current node. This field establishes a node-to-parent relation
   and will be automatically updated by the incremental reparser.
6. A variant field with the `#[child]` attribute establishes a node-to-child
   relation. The name of the field must match one of the capturing operator
   keys, and the type must correspond to the capturing type (node or token) and
   the capturing repetition.

[^noderef]: NodeRef references are similar to TokenRef composite-index
references, as they point to particular syntax tree instances of the compilation
unit. We will discuss them in more detail in the next chapters as well.

## Incremental Reparsing

The parser generated by the macro will be suitable for incremental reparsing.

By default, all node variants are subject to eager caching during incremental
reparsing. These variant nodes are called *Primary*.

If you annotate a variant with the `#[secondary]` macro attribute, you inform
the macro that this node is *Secondary*, and it should not be cached.

## Rule Expressions

The expression syntax of the `#[rule(...)]` macro attribute is similar to the
regular expression syntax of the Token macro, except that inside the parse
expression, we match tokens (prefixed with a dollar sign: `$Colon`) and nodes
(without a dollar sign: `Object`) instead of Unicode characters.

Since the LL(1) parser is a recursive-descent parser that looks at most one
token ahead to make a decision in the choice operator (`A | B`), you should
consider the leftmost set[^leftmost] of the descending rules.

For example, the expression `A | B` would be ambiguous if both A and B variant
rules could start matching with the same token. Similarly, the
expression `A | $Foo` would be ambiguous if A could start with the `$Foo` token.

All variant rules except the `#[root]` variant and the trivia expressions must
parse at least one token. The Root variant is allowed to parse potentially empty
token streams.

The macro will check these and other requirements and yield descriptive error
messages if one of the requirements is violated.

Similar to the Token's regexes, you can use the `dump(...)` operator for
debugging purposes, which prints the state-machine transitions, captures, and
the leftmost set of the surrounding parse expression.

[^leftmost]: The set of tokens from which the parse rule starts matching
directly or indirectly by descending into other rules is called the "leftmost
set".

## Capturing

The expression operator `start: $BraceOpen` means that the parser matches the
token "BraceOpen" and puts its TokenRef reference into the "start" field of the
variant's body.

The operator can capture either a node or a token. If there is something else on
the right-hand side, the capture operator will be spread to the inner operands.
For example, `foo: (Bar | Baz*)` means the same as `(foo: Bar) | (foo: Baz)*`.

The type of the corresponding field depends on what the operator captures (node
or token) and how many times. If the operator could be applied no more than
once, the field type would be NodeRef or TokenRef, respectively. If the operator
could be applied more than once, the type would be a Vec of NodeRef or TokenRef.

Examples:

- In `foo: Bar`, the "foo" field would have the type NodeRef.
- In `foo: $Bar`, the "foo" field would have the type TokenRef.
- In `foo: Bar & foo: Baz`, the "foo" field would have the type Vec<NodeRef>.
- In `foo: Bar*`, the "foo" field would also have the type Vec<NodeRef>.
- In `foo: $Bar?`, the "foo" field would have the type TokenRef because "Bar"
  can be matched no more than one time. If the parser never matches "Bar", the
  "foo" field receives the
  value [TokenRef::nil](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/lexis/struct.TokenRef.html#method.nil).

## Guidelines

1. **Keep the syntax grammar simple**.

   The purpose of the syntax tree is to express the general nesting structure of
   the source code to assist the further semantic analysis stage. If your
   language grammar contains rules that require higher-level lookaheads or
   context-dependent parsing, it might be better to parse a more simplified
   subset of this syntax at the syntax parse stage, leaving the rest of the
   analysis to the semantic stage.

2. **Always capture the start and end bounds of the node**.

   If your parse rules would capture the start and end tokens either directly or
   indirectly by capturing the starting and ending nodes of this node, these
   captures would help Lady Deirdre properly understand the starting and ending
   sites of the node. This is especially important for functions such as
   the [NodeRef::span](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/syntax/struct.NodeRef.html#method.span)
   function.

   In particular, for this reason, in
   the [JSON object and array rules](https://github.com/Eliah-Lakhin/lady-deirdre/blob/f350aaed30373a67694c3aba4d2cfd9874c2a656/work/crates/examples/src/json_grammar/syntax.rs#L96),
   we capture start and end tokens even though they are meaningless in terms of
   syntax tree traversing.

3. **Annotate the captured fields with the `#[child]` attribute**.

   By annotating the captured field with this attribute, you make it clear that
   the corresponding field should be treated as a node's child, even if this
   child is a token.

   For instance,
   the [traverse_tree](https://docs.rs/lady-deirdre/2.2.0/lady_deirdre/syntax/trait.SyntaxTree.html#method.traverse_tree)
   function relies on this metadata when performing the syntax tree depth-first
   traversal.

4. **Keep the `#[child]` fields in order**.

   For the same reasons, the captured fields should appear in the variant's body
   in the same order as they appear in the parse expressions. For example, if
   you have a `#[rule(foo: $Foo & bar: Bar* & baz: $Baz)]` rule, the variant
   fields should come in this order: "foo", "bar", and "baz".

5. **Don't capture semantically meaningless inner tokens**.

   Capturing a comma token in a comma-separated list, for example, is likely
   unnecessary because you wouldn't rely on the list separators when analyzing
   the list.

   Note that for source code formatting purposes, you would use a dedicated API
   where all tokens are intentionally included in the parse tree regardless of
   their omission in the syntax tree.

6. **Prefer wrapping semantically meaningful tokens into dedicated nodes**.

   When you encounter an intermediate token in the rule's expression that
   potentially addresses semantic metadata (e.g., a variable identifier
   in `let x`), you always have a choice: either capture it as is or introduce a
   dedicated node (e.g., "Identifier") that captures the token separately, and
   then capture the introduced node in the rule.

   For semantic analysis purposes, it would be more convenient to always work
   with node captures, so you should prefer wrapping.

   For example, in
   the [JSON object entry rule](https://github.com/Eliah-Lakhin/lady-deirdre/blob/f350aaed30373a67694c3aba4d2cfd9874c2a656/work/crates/examples/src/json_grammar/syntax.rs#L83),
   we capture the entry's key as a node-wrapper (String node) rather than as a
   token for this reason.

7. **Make the leaf nodes the Secondary nodes**.

   By default, all nodes of the syntax tree are subject to eager caching. In
   practice, the incremental reparser probably performs better if you limit the
   cache to the structurally complex nodes only and annotate the rest of the
   node variants with the `#[secondary]` attribute.

   In the JSON example syntax, Root, Object, Entry, and Array are the primary
   nodes and could be cached during incremental reparsing. However, the leaf
   nodes such as String, Number, and others are secondary nodes. The incremental
   reparser will prefer to reparse their values during reparsing, saving cache
   memory.
