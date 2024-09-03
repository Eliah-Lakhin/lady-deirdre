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

# Semantics

Semantic analysis is the final stage of the compilation project processing.

The Semantic Model is a set of user-defined data objects that collectively form
an abstraction over the syntax trees of the compilation project.

These data objects are constructed by associated user-defined *computable*
functions. Together, the model's data object and its associated function are
called an *attribute*.

Attributes are objects owned by the syntax tree nodes. By traversing the syntax
tree and querying their attribute values (the data objects of the semantic
model), you discover the semantics of the compilation project.

Some attributes are the inputs of the semantic model; they perform a direct
initial mapping of the syntax and lexical structures of the compilation units to
a subset of the semantic model. Other attributes infer derived information from
other attribute values.

Dependencies between attributes form a *semantic graph*. This graph is a
lazy-evolving, demand-driven structure and is subject to incremental
recomputations. Subsets of the graph are computed or recomputed whenever you
query attributes from these subsets. The rest of the graph remains in an
uninitialized or outdated state.

Lady Deirdre's semantic analysis framework helps you organize these data
structures and compute the semantic graph efficiently, possibly from multiple
concurrent threads, while keeping it in sync with changes in the source code of
the compilation project.

## Chain Analysis Example

The subchapters of this book section refer to
the [Chain Analysis](https://github.com/Eliah-Lakhin/lady-deirdre/tree/f350aaed30373a67694c3aba4d2cfd9874c2a656/work/crates/examples/src/chain_analysis)
example, which illustrates the basic concepts of the framework.

The example program attempts to analyze a simple programming language consisting
of nested code blocks, where each block contains variable assignment expressions
and sub-blocks.

```text
{
    x = 100;

    {
        y = x;

        {
            z = y;
            w = 200;
            u = w;
        }
    }
}
```

On the left-hand side of the assignment is the name of the variable (a "Key")
being introduced. On the right-hand side is either a numeric value or a
reference to a variable introduced previously. Variables can shadow each other.

The compiler computes the numeric values of the variables based on the system of
references between them. For instance, the variable `z` in the above snippet has
a value of `100` because it refers to the variable `y`, which in turn refers to
the variable `x`, having a numeric value of `100`.

The non-incremental approach to this problem is straightforward. We can create a
hashmap ("namespace") with the keys representing variable names and the values
representing the currently inferred numbers for these variables. Then, we
perform a depth-first traversal through the entire graph. Whenever we encounter
an assignment expression, we insert an entry into the map with the key from the
left-hand side of the expression and a value that is either a number from the
right-hand side or, if the right-hand side is a reference, we retrieve the
numeric value associated with that reference from the same map. After processing
each assignment expression, we associate the key of that expression with the
inferred number.

The above approach is inefficient in practice for two reasons:

1. In a code editor, the end-user typically views just one screen of the source
   code text at a time. Therefore, computing the entire tree is unnecessary most
   of the time.
2. Rerunning this procedure on every end-user's keystroke is necessary to keep
   the assignment expressions in sync with the changes.

To make this procedure more incremental and lazily computable, instead of
computing the entire procedure at once, we would split it into independent
sub-procedures localized for each code block.

For each code block, we will create its own namespace hashmap and traverse the
block's statements similarly to the previous approach:

- Whenever we encounter an assignment expression, we will try to resolve it
  based on the current hashmap state as before. However, if the assignment
  refers to a variable that does not exist in the map, we assume that the
  referenced variable is external. In this case, we will use a string with this
  reference name in the map's entry as a marker that this variable is external.
- If we encounter a nested block, we will not descend into this block. Instead,
  we will associate this block with the current copy of the hashmap.

```text
{
    x = 100; // x = 100

    // namespace copy: x = 100
    {
        y = x; // y = "x"

        // namespace copy: y = "x"
        {
            z = y; // x = "y"
            w = 200; // w = 200
            u = w; // u = 200
        }
    }
}
```

Note that the above block procedures are independent from each other. Each
block's procedure can be run in any order, and the running could be postponed
until needed.

To query a particular variable's resolution, first, we run the block procedure
into which it is nested. Then, we look at the local variable resolution: if the
variable was already resolved to a numeric value by its block procedure (such
as `x`, `w`, or `u` variables), we are done.

Otherwise, we run the procedure of the parent block and look at the copy of the
namespace that the parent's procedure assigns to our block. If the namespace
contains a numeric value for the referred token, we are done. Otherwise, we
repeat this iteration with the grandparent block, and so on, until we climb up
to the ancestor where the number is found.

This incremental approach offers two advantages:

1. Whenever we need to run the block-resolution procedure, we can cache its
   results as well as all intermediate resolutions. This means that the next
   time we resolve this or another variable that directly or indirectly depends
   on this block's local resolutions, we can retrieve their values from the
   cache.
2. If the end-user types something in the block, we can erase only this block's
   caches. As a result, the previously computed resolutions can still utilize
   the caches that were not erased by these changes.

This example illustrates the core concept of the incremental semantic analysis
framework. The source codes of the compilation units should be split into
independent scopes, so that the local semantic information can be inferred from
the states of the scopes. Then, higher-level procedures will infer higher-level
semantics from the local semantics of the scopes by seamlessly connecting their
bounds. Finally, all of these procedures would cache their results for reuse,
and these caches are subject to invalidation depending on the changes in the
source code.
