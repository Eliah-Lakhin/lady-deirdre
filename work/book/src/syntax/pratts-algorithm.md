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

# Pratt's Algorithm

In this chapter, I will explain how the algorithm implemented in the
hand-written parser in the [Expr Parser](todo) example works in general. You may
find this approach useful for programming languages with infix expressions (math
expressions with binary operators).

To recall, the example parses expressions of simple boolean logic: `true`
and `false` are the atomic operands of the expression, `_ & _` and `_ | _` are
conjunction and disjunction binary operators respectively, where the conjunction
has a priority over disjunction (`true & false | false & true`
means `(true & false) | (false & true)`). Finally, the language has a
parenthesis grouping operator (e.g., `(true | false)`).

In theory, we could describe such a language in terms of the ordinary LL(1)
grammar, for example, by parsing lists of operands separated by the operator
tokens and disregarding the operators' precedence, assuming that the operator
precedence will be established in the semantic analysis stage manually and based
on the lists' content. However, such an approach is generally acceptable, but it
is usually more convenient to work with an already prepared binary tree that
properly reflects operands nesting.

Parsing binary trees with left and right recursion is generally impossible in
LL-parsers because these parsers' grammar cannot express left recursion.
However, inside the hand-written recursive descending parser, we can bypass this
limitation.

The approach used behind the example
utilizes [Pratt's Parsing Algorithm](https://en.wikipedia.org/wiki/Operator-precedence_parser#Pratt_parsing).

The idea is that we associate each operator with a numeric priority, usually
called a *binding power*: 0 for unbound precedence, 1 for the `|` operator, and
2 for the `&` operator[^binding]. There are two mutually recursive functions:
the `parse_operator` function that parses a sequence of operators from the token
stream in accordance with the specified binding power, and the `parse_operand`
function that parses the atomic operand ("true" or "false"), or a parenthesis
operator (which we treat as an operand too).

The parsing procedure starts by entering into the *parse_operator* function with
zero binding power (which means that the function should attempt to parse all
input tokens).

First, this function parses an operand by calling the *parse_operand* function
and stores the result in the `accumulator` variable. The first operand that we
parsed is going to be the left-hand operand.

Then the function enters a loop where it parses the next incoming pairs of
operator tokens and the right-hand operands and reduces them to the left-rotated
binary tree using the *accumulator*:

1. Whenever the loop encounters the next operator token, it checks if this
   operator has the equal or higher binding power than the current one. If not,
   it breaks the loop.
2. Otherwise, the loop consumes the token and parses the right-hand side operand
   by recursively calling the *parse_operator* function **with the binding power
   of this operator**.
3. Finally, the loop folds the current *accumulator* as the left-hand side of
   the operation and the result of the right-hand side parsing product into a
   binary node representing this operation and stores it in the accumulator
   again, resuming the loop.
4. The loop finishes when it encounters the end of the token stream input or
   the `)` token that denotes that the function reached the end of the
   expression inside the `(...)` grouping expression.

```rust,noplayground
fn parse_operator<'a>(
    session: &mut impl SyntaxSession<'a, Node = BoolNode>,
    context: NodeRule,
    binding: u8, // Current Binding Power
) -> NodeRef {
    let mut accumulator = parse_operand(session, context);

    loop {
        // Skipping the whitespaces between operands and operators.
        skip_trivia(session);

        // Looking ahead at the next token.
        let token = session.token(0);

        match token {
            // `&` operator encountered.
            BoolToken::And => {
                // Check the current binding power with the operator's binding
                // power.
                if binding >= 2 {
                    return accumulator;
                }
                
                // Folds the current accumulator as the left-hand operand and
                // the next right-hand operand into a single binary node.

                let left = accumulator;

                let node = session.enter(BoolNode::AND);

                if !left.is_nil() {
                    session.lift(&left);
                }

                let parent = session.parent_ref();

                session.advance(); // Consumes the operator token.
                skip_trivia(session);

                // Parses the right-hand side with the operator's binding power (2).
                let right = parse_operator(session, BoolNode::AND, 2);

                // Finishes folding and stores the result in the accumulator.
                accumulator = session.leave(BoolNode::And {
                    node,
                    parent,
                    left,
                    right,
                });
            }

            BoolToken::Or => {
                if binding >= 1 {
                    return accumulator;
                }

                // The same procedure, but uses the binding power 1 when parsing
                // the right-hand side.

                // ...
            }

            // The end of the input has been reached.
            // Breaking the loop and returning the accumulated result.
            BoolToken::ParenClose | BoolToken::EOI => return accumulator,

            _ => {
                // Syntax error handler
            }
        }
    }
}
```

The *parse_operand* function, in turn, parses just a single operand ("true" or
"false") or a parenthesis operator, which is treated as an operand too.

```rust,noplayground
fn parse_operand<'a>(
    session: &mut impl SyntaxSession<'a, Node = BoolNode>,
    context: NodeRule,
) -> NodeRef {
    loop {
        let token = session.token(0);

        match token {
            BoolToken::True => return parse_true_operand(session),
            BoolToken::False => return parse_false_operand(session),
 
            // Recursively descends into the `parse_operator` function again
            // with binding power 0 when parsing the inner expression
            // inside `(...)`.
            BoolToken::ParenOpen => return parse_group(session),

            _ => {
                // Syntax error handler.
            }
        }
    }
}
```

The above algorithm effectively constructs a left-rotated binary tree. However,
the algorithm could be easily extended to cover more cases:

- If you assign even binding powers to the operators (`&` power is 20, `|` power
  is 10), you can easily turn any operator into the right-recursive by passing
  the one binding power less to the right-hand side parsers
  (e.g., `parse_operator(session, BoolNode::AND, 19)` turns the conjunction
  operator into the right recursive operator).

- The unary operators without the right-hand side could be parsed the same way,
  except that in the *parse_operator* function, you don't need to parse the
  right-hand side.

- The unary operators without the left-hand side could be parsed in the
  *parse_operand* function that would recursively call the *parse_operator*
  function with the corresponding operator's binding power to parse the
  right-hand side.

[^binding]: Some operators obviously could share the same binding power. For
example, the "+" and "-" operators in arithmetic expressions would have the same
priority, and therefore the same binding power.
