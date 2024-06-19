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

# Overriding a Parser

To recap,
the [Node derive macro](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/derive.Node.html)
automatically implements parse procedures for each enum variant annotated with
the `#[rule(...)]` macro attribute. Inside the rule, you write a regex-like
parse expression in terms of the LL(1) grammars used by the macro to generate
the parse procedure. This determines the leftmost set of tokens from which
the procedure starts parsing. The leftmost set is used when you descend into
this variant in another variant's rule.

There is a possibility to replace the generated parse procedure with a manually
written Rust function using the `#[parser(...)]` macro attribute.

This attribute accepts a Rust expression that must return an instance of the
enum that represents the parsing result product. As an input, you would use
the `session` variable, which is a mutable reference to
the [SyntaxSession](https://docs.rs/lady-deirdre/2.0.0/lady_deirdre/syntax/trait.SyntaxSession.html)
that represents the current state of the parsing environment.

Usually, inside this expression, you would call your parsing function passing
the `session` variable as an argument.

From the [Expr Parser](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/work/crates/examples/src/expr_parser/syntax.rs#L57) example:

```rust,noplayground

#[derive(Node)]
#[token(BoolToken)]
#[trivia($Whitespace)]
pub enum BoolNode {
    #[root]
    #[rule(expr: Expr)]
    Root {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        expr: NodeRef,
    },

    #[rule($ParenOpen | $True | $False)] // Leftmost set.
    #[denote(EXPR)]
    #[describe("expression", "<expr>")]
    #[parser(parse_expr(session))] // Overridden parser.
    Expr {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        content: NodeRef,
    },
    
    //...
    
    #[denote(AND)]
    #[describe("operator", "<and op>")]
    And {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        left: NodeRef,
        #[child]
        right: NodeRef,
    },
    
    //...
}
```

## Leftmost Set is Required

Note that even though we are overriding the parse procedure for the
*BoolNode::Expr* enum variant via the `#[parser(parse_expr(session))]` macro
attribute, we still have to specify the `#[rule($ParenOpen | $True | $False)]`
attribute too.

The macro requires this attribute because it needs to know the leftmost set of
the parser. Specifically, when we refer to the *Expr* variant inside the
*Root*' s `#[rule(expr: Expr)]` parse expression, the macro knows that the Expr
parser would start parsing from the "ParenOpen", "True", or "False" tokens as
described in its rule.

Certainly, you don't need to reimplement the entire grammar of the overridden
parse function inside the `#[rule(...)]` attribute (the macro will ignore it
anyway). Instead, it would be enough just to enumerate the leftmost tokens via
the `|` choice operator.

## Variants Denotation

Another thing to notice in this snippet is that the *BoolNode::And* variant does
not have a rule attribute, but instead, it has a pair of `#[denote(AND)]`
and `#[describe("operator", "<and op>")]` macro attributes.

We don't specify the "rule" attribute here because we are going to parse this
variant manually inside the "parse_expr" function too.

The **denote** attribute informs the macro that this variant is subject to
parsing (even if it does not have an explicitly expressed grammar rule) and
therefore is a legitimate part of the syntax tree.

The macro allows us to specify the `#[child]`, `#[parent]`, and other similar
fields in the denoted variants, assuming that their values will be assigned
manually. But more importantly, the macro reserves a parse rule number for the
denoted variant that we will use inside the manually written parser to address
this variant. The number can be accessed through the type's constant with the
name that we specify inside the attribute (`BoolNode::AND` in this case).

If the variant is denoted but does not have a rule, the macro additionally
requires specifying the **describe** attribute, which provides the end-user
facing description of this syntax tree node variant. The first parameter is a
string that describes the general class of this node variant (`"operator"`), and
the second one is a more specific description of this particular
variant (`"<and op>"`). Lady Deirdre will use this metadata to format error
messages for the syntax errors.

Finally, the variants with the rule attribute are assumed to be denoted
implicitly. We don't need to denote them manually, but as a rule of thumb, it is
recommended denoting and describing all enum variants regardless.
