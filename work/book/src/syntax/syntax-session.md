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

# Syntax Session

Inside the hand-written parse function, you will use the `session` variable
provided by the macro-generated code when it invokes this function.

The variable is of
type [SyntaxSession](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.SyntaxSession.html),
which provides an interface to read input tokens and manage the output syntax
tree.

The final goal of the parse function is to read some tokens from the syntax
session to recognize the target grammar rule and initialize and return an
instance of the syntax tree node as a result of the parsing procedure.

## Input Tokens Stream

The SyntaxSession trait is at first place a supertrait
of [TokenCursor](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.TokenCursor.html),
representing an input stream for the parser.

From this interface, you can read tokens' metadata ahead of the current stream
position. For example, the `session.token(0)` function returns the first token
that has not been consumed yet, `session.token(1)` reads the next token, and so
on. Other similar lookahead functions allow you to observe more metadata about
the tokens[^string]. However, typically, the syntax parser should only rely on
the token instances when making a decision to change its own inner parse
state[^lookahead].

None of these lookahead functions move the input stream forward. Once your parse
algorithm has observed a few tokens ahead, analyzed them, and made a decision to
actually "consume" these tokens, the algorithm calls
the [TokenCursor::advance](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.TokenCursor.html#tymethod.advance)
function, which consumes one token and moves the stream position to the next
token,
or [TokenCursor::skip](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/lexis/trait.TokenCursor.html#tymethod.skip),
which allows you to consume several tokens.

For instance, in
the [Expr Parser](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/expr_parser/parser.rs#L271)
example, we are parsing a sequence of whitespaces iteratively by reading
the tokens one by one:

```rust,noplayground
fn skip_trivia<'a>(session: &mut impl SyntaxSession<'a, Node = BoolNode>) {
    loop {
        // Looking ahead at the next token.
        let token = session.token(0);

        // If the token is not a whitespace token, finish the parse procedure.
        if token != BoolToken::Whitespace {
            break;
        }

        // Otherwise, if the token is a whitespace, consume it, and resume
        // the loop from the next token. 
        session.advance();
    }
}
```

Note that the above function, as a helper function in the overall procedure,
could potentially parse zero tokens. However, the general algorithm is required
to consume at least one token from the non-empty input stream.

[^string]: For instance, `session.string(0)` would return a substring of the
source code text covered by the first token.

[^lookahead]: Furthermore, ideally, the parser should look ahead at no more than
a single token ahead of the stream position (`session.token(0)`). The fewer
tokens you look ahead, the better incremental reparsing performance you would
gain. However, this is not a strict requirement. Looking at a few tokens ahead
is generally acceptable.

## Error Recovering

If the parsing algorithm hasn't finished yet but encounters an unexpected token
in the middle of the parsing procedure — a token that normally shouldn't exist
in the input stream based on the current parse state — this is a syntax error.

Conventionally, in a hand-written parser, you should follow the same error
recovery approaches common for parsers generated by the macro. More likely, you
would use the [panic recovery](error-recovering.md#panic-recovery) procedure.

To avoid manually reimplementing the panic recovery algorithm and to be
consistent with the auto-generated parsers, Lady Deirdre exposes
the [Recovery](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/struct.Recovery.html)
configurable object that implements this algorithm, which is also used inside
the macro-generated code.

The Recovery object has the same configuration options that you would use inside
the `#[recovery(...)]` macro attribute:
the [Recovery::unexpected](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/struct.Recovery.html#method.unexpected)
function adds a halting token, and
the [Recovery::group](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/struct.Recovery.html#method.group)
function adds a group of tokens that should be treated as a whole.

It is assumed that this object will be constructed upfront in the const context
and stored in a static for fast reuse.

Depending on the parsing procedure complexity, you may want to prepare several
Recovery objects for various types of syntax errors. For instance, in
the [Expr Parser](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/expr_parser/parser.rs#L54)
example, there are three prepared Recovery objects: one to recover from syntax
errors in the operators, one for operands, and one for errors inside
the parentheses.

```rust,noplayground
static OPERAND_RECOVERY: Recovery =
    // Creates an unlimited recovery configuration without groups
    // and halting tokens.
    Recovery::unlimited() 
        // Adds a group of parenthesis tokens.
        // The sequence of tokens like `(...)` will be consumed as a whole
        // during the panic recovery.
        .group(BoolToken::ParenOpen as u8, BoolToken::ParenClose as u8)
        // If the recoverer encounters a `)` token somewhere outside of any
        // group, it halts the recovery procedure (the halting token will
        // not be consumed).
        .unexpected(BoolToken::ParenClose as u8);
```

To apply the panic recovery procedure, you call
the [Recovery::recover](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/struct.Recovery.html#method.recover)
function, passing it the `session` variable and the set of tokens the recoverer
should look for. The function will consume as many tokens as needed according to
the configured rules and will return an object describing whether the procedure
managed to find the required token or failed to do so due to a specific
reason (e.g., a halting token has been reached).

Regardless of the recovery result, you should report the error using
the [SyntaxSession::failure](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.SyntaxSession.html#tymethod.failure)
function.

```rust,noplayground
// A set of tokens that we expect as the leftmost token of an operand.
static OPERAND_TOKENS: TokenSet = TokenSet::inclusive(&[
    BoolToken::True as u8,
    BoolToken::False as u8,
    BoolToken::ParenOpen as u8,
]);

// ...

fn parse_operand<'a>(
    session: &mut impl SyntaxSession<'a, Node = BoolNode>,
    context: NodeRule,
) -> NodeRef {
    loop {
        let token = session.token(0);

        match token {
            // Handling expected tokens.
            BoolToken::True => return parse_true_operand(session),
            BoolToken::False => return parse_false_operand(session),
            BoolToken::ParenOpen => return parse_group(session),

            // Otherwise, try to recover using the panic recovery algorithm.
            _ => {
                // A SiteRef of where the unexpected token was encountered.
                let start_site_ref = session.site_ref(0);

                // Runs the recovery procedure. This function possibly consumes
                // some tokens from the input stream (using `session`).
                let result = OPERAND_RECOVERY.recover(session, &OPERAND_TOKENS);

                // A SiteRef of where the recoverer finishes.
                let end_site_ref = session.site_ref(0);

                // Regardless of the recovery result, the syntax error has
                // to be reported.
                session.failure(SyntaxError {
                    span: start_site_ref..end_site_ref,
                    context,
                    recovery: result,
                    expected_tokens: &OPERAND_TOKENS,
                    expected_nodes: &EMPTY_NODE_SET,
                });

                // If the recoverer failed to recover, finish the parse loop;
                // otherwise, resume parsing from the recovered token stream
                // position.
                if !result.recovered() {
                    return NodeRef::nil();
                }
            }
        }
    }
}
```

## Rules Descending

Whenever your parser needs to descend into other rules, you basically have two
options:

1. Call
   the [SyntaxSession::descend](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.SyntaxSession.html#tymethod.descend)
   function, which gives control flow back to the parsing environment.
2. Create and parse the node manually using a pair of
   functions: [SyntaxSession::enter](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.SyntaxSession.html#tymethod.enter)
   and [SyntaxSession::leave](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.SyntaxSession.html#tymethod.leave).

The result of the *descend* function would be similar to if the parsing
environment parsed the requested node: it will advance the token cursor of
the `session` to as many tokens as needed to cover the parsing rule, it will add
a branch of nodes to the syntax tree as a result of parsing, and it will return
you a NodeRef reference of the top node of the branch. Basically, the *descend*
function performs a normal parsing procedure, except that in practice, during
incremental reparsing, this function could potentially utilize the parsing
environment's inner cache to bypass real parsing steps.

You should prefer to use the *descend* function on
the [primary nodes](syntax-grammar.md#incremental-reparsing) whenever possible.

In
the [Expr Parser](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/expr_parser/parser.rs#L230)
example, we are using this method to descend into the subexpression when parsing
the expression group surrounded by the `(...)` parentheses.

```rust,noplayground
fn parse_group<'a>(
   session: &mut impl SyntaxSession<'a, Node = BoolNode>,
) -> NodeRef {
    // Consumes the opening "(" token.
    session.advance();

    // Skips whitespaces in between.
    skip_trivia(session);

    // Parses the inner expression.
    let inner = session.descend(BoolNode::EXPR);

    // In the rest of the code, we are parsing the closing ")" token and return
    // the `inner` NodeRef to the parsed subexpression.
}
```

Calling the *descend* function requires you to follow the same requirements as
if you were descending into the rule from
the [Node macro expression](syntax-grammar.md#rule-expressions):

1. Left recursion is forbidden. You should not call this function at the
   beginning of the parse procedure if descending into this rule could directly
   or indirectly lead to recursive calling of the current parsing procedure.
   Such a call is likely to result in infinite recursion. However, descending
   into the same rule in the middle of the parsing is perfectly fine. In
   particular, the `parse_group` function recursively descends into the
   same `parse_expr` procedure because we forcefully consume the `(` token
   before the call.
2. The variant you descend to must have a parser. The variant should have
   a `#[rule(...)]`.

The second method allows you to parse the subnode manually and is generally not
restricted to the above limitations.

Calling the *enter* function starts node parsing. Calling the *leave* function
finishes the subparser and returns the syntax session to parsing of the parent
node. The *enter* and *leave* functions must be properly balanced: entering into
the subparse context must always be enclosed by leaving the context.

In the *leave* function, you specify the instance of the node that is the
product of the subparser. This function returns a NodeRef of the product
deployed to the syntax tree (similarly to the *descend* function).

```rust,noplayground
fn parse_true_operand<'a>(
   session: &mut impl SyntaxSession<'a, Node = BoolNode>,
) -> NodeRef {
    // Starts "true" node parsing.
    session.enter(BoolNode::TRUE);

    // Consumes the "true" token.
    session.advance();

    // A NodeRef of the syntax tree node currently being parsed.
    let node = session.node_ref();
    
    // A NodeRef of the parent node that we parsed before entering into
    // the "true" node subparser.
    let parent = session.parent_ref();

    // Finishes "true" node subparser, and returns its NodeRef.
    return session.leave(BoolNode::True { node, parent });
}
```

Note that both *descend* and *enter* functions require the rule number as an
argument. Having this number, the parsing environment reveals nesting between
the parsing procedures, which is specifically important for building the parser
tree.

These numbers are the constants that were specified in the `#[denote(TRUE)]`
macro attributes when we set up the derive macro.

## Left Recursion

Lady Deirdre follows an approach to handling left recursion by lifting syntax
tree nodes to their siblings, such that the node becomes a child of its former
sibling.

When parsing code such as `true & false`, first, you parse the `true` operand.
If the input stream finishes at this step, you return this node as the result
product of parsing. Otherwise, when the parser encounters an `&` token, it
starts a new binary operator parser immediately lifting the previously created
operand to the current operator's context, then parses the second operand and
finishes the operator parser. You can repeat this procedure iteratively to
create a left-rotated binary tree.

The [SyntaxSession::lift](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.SyntaxSession.html#tymethod.lift)
function "transplants" the syntax tree branch created just before we enter the
new node subparser to the context of this subparser. In particular, this
function automatically changes the parent NodeRef of the former sibling to the
node that we start parsing.

From the operator parser of
the [Expr Parser](https://github.com/Eliah-Lakhin/lady-deirdre/blob/1f4ecdac2a1d8c73e6d94909fb0c7fcd04d31fc0/work/crates/examples/src/expr_parser/parser.rs#L90)
example:

```rust,noplayground
BoolToken::And => {
    if binding >= 2 {
        return accumulator;
    }

    // The `accumulator` is the result product of the overall parse procedure.
    let left = accumulator;

    // Entering into the `&` operator subparser.
    let node = session.enter(BoolNode::AND);
 
    // The accumulated product could be Nil due to syntax errors.
    // In this case, we should not and cannot lift it.
    if !left.is_nil() {
        // Makes the former accumulated node the child (the left-hand operand)
        // of the currently parsed operator.
        session.lift(&left);
    }

    let parent = session.parent_ref();

    // Consumes the `&` token.
    session.advance();
    skip_trivia(session);

    // Parses the right-hand side of the operator.
    let right = parse_operator(session, BoolNode::AND, 2);

    // Finishes operator subparser, and sets the result to the `accumulator`
    // for reuse on the next loop step.
    accumulator = session.leave(BoolNode::And {
        node,
        parent,
        left,
        right,
    });
}
```

## Nodes Relations

In contrast to the macro-generated parsers, in the hand-written parser, you have
to instantiate and properly initialize the instance of the node manually: when
you return the final result from the overall parse procedure, and when you
finish the inner subparsers via the *leave* function.

To set up the node-to-parent relation, you can use
the [SyntaxSession::parent_ref](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.SyntaxSession.html#tymethod.parent_ref)
function that returns a NodeRef reference to the parent node in the syntax tree
of the currently parsed node.

The [SyntaxSession::node_ref](https://docs.rs/lady-deirdre/2.0.1/lady_deirdre/syntax/trait.SyntaxSession.html#tymethod.node_ref)
returns a NodeRef reference of the currently parsed node that will be deployed
into the syntax tree when the parser finishes parsing (or subparsing) process.

To set up the child NodeRefs, you can use the result of the *descend* and
*leave* functions.

Whenever the parse procedure encounters syntax errors that cannot be recovered,
the parser function should set the child references to the most reasonable
defaults following the same [approach](error-recovering.md#mismatched-captures)
as in the macro-generated parsers.
