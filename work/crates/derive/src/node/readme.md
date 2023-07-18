A derive macro of the Node trait to construct Syntax Parser using a set of
context-free LL(1) grammar rules.

This macro implements a [Node](::lady_deirdre::syntax::Node) trait for the
Rust enum type.

An API user specifies Node parse rules directly on enum variants through the
macro attributes. The macro analyses these rules validity in compile-time and
constructs run-time optimized and error-resistant
[`LL(1) parser`](https://en.wikipedia.org/wiki/LL_parser)
of the [Token](::lady_deirdre::lexis::Token) sequences as a Node trait
implementation.

In case of invalid definitions or misuse the macro throws descriptive
compile-time errors to the macro programmer.

## Grammar Specification.

Derive macro application outline:

```rust ignore
#[derive(Node)]
#[token(MyToken)] // Specifies a Token type.
#[error(MyError)] // Specifies an Error type.
#[skip($Whitespace | $LineBreak)] // Tokens to be auto-skipped during parsing.
enum MyNode {
    #[root] // An entry-point Rule.
    #[rule(foos: Foo*)]
    MyRoot {
        foos: Vec<NodeRef>
    },

    // A Regular Rule.
    #[rule($Foo & field_1: $Bar & field_2: AnotherRule+)]
    Foo {
        field_1: TokenRef,
        field_2: Vec<NodeRef>,
    },

    // A Comment Rule.
    #[comment]
    #[rule($CommentStart & ($Whitespace | $Foo)* & $LineBreak?)]
    Comment,

    // ...
}
```

Enum variants labeled with `#[rule(<expression>)]` attributes specify the set
of LL(1) grammar rules. And the instances of these variants are products of
corresponding rule execution. The expression language is a regular-expression
alike language that could refer(possibly in recursive way) other rules.

During the expression execution the parser is capable to track selected
Tokens and other referred rule Nodes, and to store their weak references
inside the Variant's product fields. This process called Capturing. A system
of Node Variants with captured weak references builds up an Abstract Syntax Tree
of the source code.

As the LL(1) grammar cannot express
[left recursion](https://en.wikipedia.org/wiki/Left_recursion), an API user
cannot express common infix expressions("mathematical" expressions) with
operator precedence directly using this grammar language. To work around this
problem, it is assumed that the parser would perform a raw parsing of a sequence
of operands and operators ignoring operators' priority to be post processed
later on using
[Shunting Yard](https://en.wikipedia.org/wiki/Shunting_yard_algorithm) or
[Pratt](https://en.wikipedia.org/wiki/Operator-precedence_parser#Pratt_parsing)
parsing algorithms.

### Terms.

  - __Parsable Rule__. Any Enum variant labeled with `#[rule(...)]` attribute.
    Such variants represent LL(1) Grammar rules and the Syntax Tree Nodes
    produced by these rules.
  - __Regular Rule__. A Parsable Rule. Defines a regular programming language syntax
    component(e.g. a function, a class definition, a statement block, etc).
  - __Root Rule__. A Parsable Rule. A grammar entry-point rule.
  - __Comment Rule__. A Parsable Rule. Defines a syntax of a comment that could
    implicitly appear at any site of the source code(except other Comments).
  - __Skip Tokens__. A set of tokens to be ignored in the Regular and the Root
    rules during parsing. Such tokens are not ignoring in the Comment Rules.
  - __Expression__. A matcher of a sequence of Tokens. This is a body of
    a Parsable Rule or an Inline Expression.
  - __Inline Expression__. A named Expression to be inline by name directly
    into any other Expression.
  - __Reference__. A named reference of a Parsable Rule inside Expression.
  - __Capturing__. An identifiable part of the expression that either matches
    a Token or another Rule. The weak references into such Tokens or Rule's
    Nodes will be stored in the Variant's named fields.
  - __Leftmost Tokens__. A set of first Tokens of sequences of tokens
    that could be parsed by specified Expression.
  - __Rightmost Tokens__. A set of last Tokens of sequences of tokens
    that could be parsed by specified Expression.

### Expressions.

Expression language is any combination of the following sentences that
fully recognizes a sequence of Tokens.

| Sentence                 | Example          | Description                                                                                 |
|:-------------------------|:-----------------|:--------------------------------------------------------------------------------------------|
| Token Match.             | $Foo             | Matches a single token.                                                                     |
| Inline.                  | Foo              | If referred Identifier is an Inline Expression, matches this Expression.                    |
| Reference.               | Foo              | If referred Identifier is a Parable Rule, descends into this Rule.                          |
| Group.                   | (Foo &amp; $Bar) | In a pattern "(A)", sentence A matches.                                                     |
| Sequence Match.          | Foo &amp; $Bar   | In a pattern "A &amp; B", sentence A matches, and then sentence B matches.                  |
| Choice Match.            | Foo &#124; $Bar  | In a pattern "A &#124; B", either sentence A matches, or sentence B matches.                |
| Zero or More Repetition. | $Foo&ast;        | In a pattern "A&ast;", sentence A matches zero or more times.                               |
| Zero or More Repetition. | $Foo&ast;{$Bar}  | In a pattern "A&ast;{B}", sentences A delimitered by B matches zero or more times.          |
| One or More Repetition.  | $Foo+            | In a pattern "A+", sentence A matches one or more times.                                    |
| One or More Repetition.  | $Foo+{$Bar}      | In a pattern "A+{B}", sentences A delimitered by B matches one or more times.               |
| Optional Match.          | $Foo?            | In a pattern "A?", sentence A fully matches or does not match at all.                       |
| Capture.                 | field_1: $Foo    | In a pattern "id: A", matches a pattern of A, and stores matching result in the field "id". |

For Binary operators such as Sequence Match(&amp;) and Choice Match(&#124;)
the Sequence Match have priority over the Choice Match.
Unary operators(&ast;, +, ?, capturing) have priorities over the binary
operators. And the Group operator prioritizes anything inside the parenthesis.

### Restrictions.

  1. There is one and only one Root rule in the Grammar that is a Parser entry
     point.
  2. The Root Rule cannot be referred by any other Rule. As such the Root Rule
     is not recurrent.
  3. Any Regular Rule must be directly or indirectly referred by the Root Rule.
     In other words, any Regular Rule must be distinguished.
  4. Referred Parsable Rule's Leftmost Tokens cannot conflict with another
     Tokens in Expression in the same matching position.

     ```rust ignore
     enum MyNode {
         #[rule($A)]
         Foo1,

         #[rule($A | $B)]
         Foo2,

         // Conflicts, because Foo1's leftmost token is $A.
         #[rule($A | Foo1)]
         Conflict1,

         // Conflicts, because both Foo1 and Foo2 could start with $A.
         #[rule(Foo1 | Foo2)]
         Conflict2,
     
         // This is fine, because Foo1 and Foo2 are in the different matching
         // positions.
         #[rule(Foo1 & Foo2)]
         Ok,
     }
     ```
  5. All Inline Expression names and Parsable Rules' variant names must be
     unique.
  6. The Capturing variable inside a Rule expression cannot capture values of
     different kinds. For example, Capturing Variable cannot capture Token
     and Node at the same time.

     ```rust ignore
     enum MyNode {
         #[rule($Foo)]
         SomeNode,

         // Conflicts, because `capt_1` tries to capture Token and Node at
         // the same time.
         #[rule((capt_1: $SomeToken) & (capt_1: SomeNode))]
         Conflict { capt_1: TokenRef },

         // No conflict, `capt_1` and `capt_2` are two distinct variables.
         #[rule((capt_1: $SomeToken) & (capt_2: SomeNode))]
         Ok { capt_1: TokenRef, capt_2: NodeRef },
     }
     ```
  7. Capturing variable type must match variant field's type.

     ```rust ignore
     enum MyNode {
         // `capt_1` captures a Token, not a Node.
         #[rule(capt_1: $Foo)]
         Error1 { capt_1: NodeRef },

         #[rule(capt_1: $Foo)]
         Ok1 { capt_1: TokenRef },

         // `capt_1` captures a token multiple times.
         #[rule(capt_1: $Foo+)]
         Error2 { capt_1: TokenRef },

         #[rule(capt_1: $Foo+)]
         Ok2 { capt_1: Vec<TokenRef> },

         // Even though $Foo could be matched zero times this is still fine,
         // because TokenRef could be a `TokenRef::nil()` reference.
         #[rule($Bar & capt_1: $Foo?)]
         Ok3 { capt_1: TokenRef },
     }
     ```
  8. Comment Rules cannot refer Parsable Rules. And Parsable Rules cannot refer
     Comments directly.
  9. Skip Tokens cannot be matched in the Root Rule and Regular Rules
     explicitly. But they can(and should) be matched inside Comment Rule
     Expression.
  10. Regular and Comment Rules cannot match empty sequences of Tokens. 

## Error Recovery.

The Macro constructs Syntax Parser with syntax errors recovery capabilities.

Error Recovery is a heuristic process. There are two error recovery strategies:
Insert Mode and Panic Mode. The choice between two of them determined by
the Macro in every possible parsing situation preliminary during compile-time
static analysis of specified grammar.

A particular Parsable Rule enters recovery mode when the next reading Token
does not fit any possibility specified by the rule's Expression in particular
parsing state.

For example, if the rule with Expression `$Foo & $Bar & $Baz` tries to parse a
`[$Foo, $Baz]` sequence, it would successfully match the first Token, but then
fail on the second Token($Bar was expected, but $Baz found) entering error
recovery mode to fulfill this rule requirements.

In the error recovery mode, if the Parser did not match required Capturing
variable, the corresponding Variant field will be set to Nil(`TokenRef::nil()`
or `NodeRef::nil()`).

### Insert Mode.

If the Rule has mismatched one specific Token, and this Token expected to be the
only possibility in particular parsing situation, and the next reading Token
is the Token expected to be matched after that missing one, the Parser
will ignore this mismatch as if the missing Token would be in place(virtually
"inserting" this Token).

For example, for `$Foo & $Bar & $Baz` Expression the Parser would "insert" a
Token in the middle of the `[$Foo, $Baz]` sequence to fulfill Expression's
matching requirements.

When the Insert Mode applicable it has a priority over the Panic Mode.

### Panic Mode.

If the syntax error cannot be recovered by the Insert Mode, the Panic Recovery
Mode takes place instead.

In Panic Mode the Parser eagerly skips all incoming mismatched Tokens until 
the possible expected Token found.

For example, for the `$Foo & ($Bar | &Baz) & $Aaa` Expression and the
`[$Foo, $Bbb, $Ccc, $Bar, $Aaa]` input sequence, the Parser will skip the second
and the third Tokens, and then continues parsing process normally. 

In Panic Mode token skipping process is usually limited by a set of heuristic
contextual assumptions to prevent overuse of recovery strategy. If the Panic
Mode cannot fulfill Expression requirements the Parser leaves error recovery
mode earlier finishing corresponding Syntax Tree Node as it is, and normally
returns control flow to the ancestor's rule.

There are three possibilities when the Parser could early finish Panic Mode:
  1. Rule's Expression unambiguously ends with the delimiter token(e.g.
     a semicolon token in the end of the statement). If the parser encounters
     such Token, the Panic Mode will be finished earlier.
  2. An API user has explicitly specified a set of Synchronization Rules(using
     `#[synchronization]` attribute) to define a global set of synchronization
     Token pairs. For example, in Rust code block tokens `{` and `}` would be
     a good candidate of such global synchronization. In this case during the
     Panic Recovery the Parser will count of nesting of such Tokens, and
     it will early finish Panic Mode when the outer synchronization context
     termination detected.
  3. If no more Tokens are left in the input sequence.


## Type-level attributes.

These attributes meant to be bound with the Enum type.

```rust ignore
#[derive(Node)]
// Type-level attributes go here.
enum MyNode {
    // ..
}
```

  - ### Token Type.

    **Format:** `#[token(<token type>)]`.

    Specifies a type of the Source Code tokens. `<token type>` must be an enum
    type accessed from the current context, and it must implement a Token
    trait. It is assumed that the `<token type>` would be derived by the
    [Token](crate::Token) macro, but this is not a strict requirement.

    This attribute is **required**.

  - ### Syntax Error Type.

    **Format:** `#[error(<error type>)]`.

    Specifies a type of the syntax error. This type must be accessed from the
    current context, and it must implement a `From<ParseError>` trait.
    In particular the `ParseError` itself fits this requirement.

    This attribute is **required**.

  - ### Skip Tokens.

    **Format:** `#[skip(<expression>)]`.

    Specifies a set of tokens to be auto-ignored in the Root and Regular
    parsable rules.

    A Whitespace or a Line-break tokens are good candidates for Skip Tokens. 

    This attribute is optional.

  - ### Inline Expression.

    **Format:** `#[define(<name> = <expression>)]`.
    
    Defines a named inline expression. These expressions could be further
    referred inside other regular expressions by `<name>` (including Parsable
    Rules and other Inline Expressions).
    
    The macro interprets such references as direct inlines of the
    `<expression>` value.
    
    Inline expression must be defined before use. As such, inline expression
    cannot define direct or indirect recursion.
    
    Inline expression is a mechanism to reuse of frequently repeated fragments
    of expressions by name.
 
    Expression's name must be unique in the entire set of all Inline Expressions
    and the enum variant names.
    
    This attribute is optional.

## Variant-level attributes.

These attributes meant to be bound with the Enum Variants.

```rust ignore
#[derive(Node)]
enum MyNode {
    // Variant attributes go here.
    Variant1,

    // Variant attributes go here.
    Variant2,
    
    // ...
}
```

  - ### Rule.

    **Format:** `#[rule(<expression>)]`.

    Defines a Parsable Rule of the enum variant.

    This is an optional attribute, but an API user must define at least one
    Parsable rule per Grammar, and to also label one Parsable Rule as a Root
    Rule.

  - ### Root Rule.

    **Format:** `#[root]`.

    Specializes a Parsable Rule to be the Grammar entry-point rule.

    This attribute must be bound to the Enum Variant already labeled as a
    [Parsable Rule](#rule), but which does not have any other specializations.

  - ### Comment.

    **Format:** `#[comment]`.

    Specializes a Parsable Rule to be the Comment Rule.

    This attribute must be bound to the Enum Variant already labeled as a
    [Parsable Rule](#rule), but which does not have any other specializations.

    Similarly to [`Skip Tokens`](#skip-tokens), Comments could appear at any
    place of any Root or Regular Rule. An API user doesn't have to refer them
    explicitly. In contrast to Skip Tokens, the Comment Rule produces a
    Syntax Tree Node instance.

  - ### Constructor.

    **Format:** `#[constructor(<enum type constructor>(variable_1, variable_2, ...))]`.

    Specifies Parsable Rule node's explicit construction function.
    
    The Parser will call provided `<enum type constructor>` function to
    construct enum's instance when rule's expression matches.

    The function must be defined on the enum type as a static function
    accessible from the current Rust scope, it must accept provided set of
    Capturing variables, and it must return an instance of this enum type.
  
    An API user specifies this attribute when the enum's Variant has
    non-standard construction mechanism. For example, if the Variant has some
    non-capturing fields with complex initialization strategy, or if the Variant
    has anonymous fields.

    This attribute must be bound to the Enum Variant already labeled as a
    [Parsable Rule](#rule).

    ```rust ignore
    #[derive(Node)]
    //...
    enum MyNode {
        // ...
    
        #[rule($Foo & bar: $Bar)]
        #[constructor(new_some_variant(bar))]
        SomeVariant(TokenRef, usize),
    
        // ...
    }

    impl MyNode {
        fn new_some_variant(bar: TokenRef) -> Self {
            Self::SomeVariant(bar, 10)
        }
    }
    ```

  - ### Synchronization.

    **Format:** `#[synchronization]`.

    Specifies a globally unique nested context for the error recovery
    synchronization.
  
    To improve error recovery mechanism it is recommended to some label Regular
    Rules that represent nested contexts that could frequently appear around the
    code. For example, in Rust a system of nested code blocks is a good
    candidate of "synchronization", because the code blocks could be nested,
    they frequently appear everywhere in the code, and they have simple pair
    of enter and leave contextual tokens("{" and "}").

    See [Panic Mode](#panic-mode) for details.

    Synchronization Rule must fit the following two requirements:
      1. Expression's leftmost token and the rightmost token are explicitly
         and unambiguously defined and distinct to each other.
      2. There are no any other Synchronization rules with the same leftmost
         and the rightmost tokens.


## Field-level attributes.

These attributes meant to be bound with the Enum Variants' Named Fields.

```rust ignore
#[derive(Node)]
enum MyNode {
    // ...

    Variant1 {
        // Field attributes go here.
        field_1: usize,
    },
    
    // ...
}
```

  - ### Default.

    **Format:** `#[default(<value>)]`.

    Specifies default value of the Variant's custom field.

    When an API user relies on default Node constructor(a constructor that is
    overloaded by the [Constructor](#constructor) attribute), it is assumed
    that Variant fields must exactly correspond to the Capturing variables.

    However, an API user can specify custom fields too by labeling them with
    this attribute. Their values will be set to the `<value>` expression during
    the Node constructing.

    ```rust ignore
    #[derive(Node)]
    //...
    enum MyNode {
        // ...
    
        #[rule($Foo & bar: $Bar)]
        SomeVariant {
            bar: TokenRef, // Will be set to the "bar" Capturing variable.

            #[default(100)]
            baz: usize, // Will be set to "100" as defined in the attribute Value.
        },
    
        // ...
    }
    ```

## Json Syntax Example.

```rust ignore
#[derive(Node, Clone)]
#[token(JsonToken)]
#[error(ParseError)]
#[skip($Whitespace)]
#[define(ANY = Object | Array | True | False | String | Number | Null)]
pub enum JsonNode {
    #[root]
    #[rule(object: Object)]
    Root { object: NodeRef },

    #[rule($BraceOpen & (entries: Entry)*{$Comma} & $BraceClose)]
    #[synchronization]
    Object { entries: Vec<NodeRef> },

    #[rule(key: $String & $Colon & value: ANY)]
    Entry { key: TokenRef, value: NodeRef },

    #[rule($BracketOpen & (items: ANY)*{$Comma} & $BracketClose)]
    #[synchronization]
    Array { items: Vec<NodeRef> },

    #[rule(value: $String)]
    String { value: TokenRef },

    #[rule(value: $Number)]
    Number { value: TokenRef },

    #[rule($True)]
    True,

    #[rule($False)]
    False,

    #[rule($Null)]
    Null,
}
```
