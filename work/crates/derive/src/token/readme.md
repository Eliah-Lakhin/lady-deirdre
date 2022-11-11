A derive macro of the Token trait to construct Lexical Scanner using a set of
regular expression.

This macro implements a [Token](::lady_deirdre::lexis::Token) trait for the
Rust enum type.

An API user specifies Token parse rules directly on enum variants through the
macro attributes. The macro analyses these rules validity in compile-time and
constructs run-time optimized
[`Finite State Automaton`](https://en.wikipedia.org/wiki/Finite-state_machine)
scanner of arbitrary Unicode strings as a Token trait implementation.

In case of invalid definitions or misuse the macro throws descriptive
compile-time errors to the macro programmer.

## Regular Expressions Specification

Regular Expression language is any combination of the following sentences that
fully recognizes a sequence of Unicode characters.

| Sentence                 | Example                    | Description                                                                  |
|:-------------------------|:---------------------------|:-----------------------------------------------------------------------------|
| Character Match.         | 'a'                        | Matches a single character.                                                  |
| Character Set Match.     | ['A'..'Z', '0'..'9', '_']  | Matches any single character within specified range(s).                      |
| Inverse Match.           | ^['A'..'Z', '0'..'9', '_'] | Matches any[^1] single character except the one from specified range(s).     |
| Inline.                  | FOO                        | Matches [Inline Expression](#inline-expression).                             |
| String Match.            | "foo"                      | Matches specified set of characters in specified order.                      |
| Group.                   | ('a' &amp; 'b')            | In a pattern "(A)", sentence A matches.                                      |
| Sequence Match.          | "foo" &amp; "bar"          | In a pattern "A &amp; B", sentence A matches, and then sentence B matches.   |
| Choice Match.            | "foo" &#124; "bar"         | In a pattern "A &#124; B", either sentence A matches, or sentence B matches. |
| Zero or More Repetition. | ['A'..'Z']&ast;            | In a pattern "A&ast;", sentence A matches zero or more times.                |
| One or More Repetition.  | ['A'..'Z']+                | In a pattern "A+", sentence A matches one or more times.                     |
| Optional Match.          | "foo"?                     | In a pattern "A?", sentence A fully matches or does not match at all.        |

For Binary operators such as Sequence Match(&amp;) and Choice Match(&#124;)
the Sequence Match have priority over the Choice Match.
Unary operators(&ast;, +, ?) have priorities over the binary operators. And the
Group operator prioritizes anything inside the parenthesis.

E.g. `'a' & 'b' | ('c' | 'd')+ & 'e'` expression matches either a string "ab",
or a string that starts with repetitions of the 'c' and 'd' characters and that
ends with character 'e'. 

[^1]: Note that the Inverse Match sentence matches through the set of characters
explicitly mentioned in the entire set of the Parsable rules expressions that
define the entire scanning "Alphabet". In principle the macro cannot set
"Alphabet" to be the full Unicode set.

## Type-level attributes.

These attributes meant to be bound with the Enum type.

```rust
#[derive(Token)]
// Type-level attributes go here.
enum MyToken {
    // ..
}
```

  - ### Inline Expression.

    **Format:** `#[define(<name> = <regular expression>)]`.
    
    Defines a named inline expression. These expressions could be further
    referred inside other regular expressions by `<name>` (including Variant
    Rules and other Inline Expressions).
    
    The macro interprets such references as direct inlines of the
    `<regular expression>`.
    
    Inline expression must be defined before use. As such, inline expression
    cannot define direct or indirect recursion.
    
    Inline expression is a mechanism to reuse of frequently repeated fragments
    of regular expression by name.
    
    This attribute is optional.
    
    ```rust
    #[derive(Token)]
    #[define(POSITIVE_DIGIT = ['1'..'9'])]
    #[define(DIGIT = '0' | POSITIVE_DIGIT)] // Referring POSITIVE_DIGIT.
    enum MyToken {
        #[rule(POSITIVE_DIGIT & DIGIT*)] // Referring POSITIVE_DIGIT and DIGIT.
        Number,
        
        // ...
    
        #[mismatch]
        Mismatch,
    }
    ```

## Variant-level attributes

These attributes meant to be bound with the Enum Variants.

```rust
#[derive(Token)]
enum MyToken {
    // Variant attributes go here.
    Variant1,

    // Variant attributes go here.
    Variant2,
    
    // ...
}
```

  - ### Rule.

    **Format:** `#[rule(<regular expression>)]`.

    Defines Parsable token variant.

    This attribute must be bound to all Parsable variants of underlying enum
    type except the [Mismatch](#mismatch) variant.

    An API user must define at least one Parable variant per enum type.

    All Parsable variants must not conflict to each other. Two variants
    considered to be conflicting if one of them could parse a string that
    would be a substring of another variant's parsable string. See
    [Precedence](#precedence) attribute for conflict resolution details.
   
    The `<regular expression>` must not parse empty strings.

    ```rust
    #[derive(Token)]
    enum MyToken {
        #[rule(['a'..'z']+)]
        Identifier,
    
        #[mismatch]
        Mismatch,
    }
    ```

  - ### Precedence.

    **Format:** `#[precedence(<numeric precendence>)]`.

    Establishes execution priority between two conflicting Parsable token
    variant.

    Two Token variants considered to be conflicting if one of them could parse a
    string that would be a substring of another variant's parsable string. Such
    conflicts required to be resolved explicitly using this attribute.

    If one Parsable token has higher `numeric precedence` over another one,
    the first one would always shadow the second one.

    The default precedence is 1. This attribute is optional, and is not
    applicable to non-parsable Variants(the Variant must be labeled with the
    `#[rule(...)]` attribute too).

    For example, an arbitrary alphabetical identifier would conflict with the
    programming language's alphabetical reserved words.

    ```rust
    #[derive(Token)]
    enum MyToken {
        #[rule(['a'..'z']+)]
        Identifier,
    
        // "keyword" string could be recognized as an Identifier too, so we have
        // to raise it's precedence explicitly.
        //
        // Note, however, that raising an Identifier's precedence instead would
        // lead to a compile-time error, because in this case the "keyword"
        // string will never match as a Keyword. It would always be recognizable
        // as an Identifier.
        #[rule(["keyword"])]
        #[precedence(2)]
        Keyword,
    
        #[mismatch]
        Mismatch,
    }
    ```
    
  - ### Mismatch.

    **Format:** `#[mismatch]`.

    One and only one non-Parsable enum variant must be labeled with this
    attribute. This attribute is required.

    All strings that cannot be recognized by any other Parsable variants will be
    sinked into this token variant.

    Such tokens considered to be lexically valid tokens, however they could be
    recognized as syntactically incorrect on the Syntax Parsing stage.

  - ### Constructor.

    **Format:** `#[constructor(<enum type constructor>)]`.

    Specifies Parsable enum variant construction function.
    
    The Scanner will call provided `<enum type constructor>` function to
    construct enum's instance when the variant's rule matches.

    The function must be defined on the enum type as a static function
    accessible from the current Rust scope, it must accept `&str` string type of
    recognized string, and it must return an instance of this enum type.

    If a Parsable enum variant has a body, this attribute must be specified
    explicitly, otherwise this attribute is optional. The attribute is
    applicable to Parsable variants only(the Variant must be labeled with the
    `#[rule(...)]` attribute too).

    ```rust
    #[derive(Token)]
    enum MyToken {
        #[rule(['1'..'9'] & ['0'..'9']* | '0')]
        #[constructor(parse_num)]
        Num(usize),

        #[mismatch]
        Mismatch,
    }

    impl MyToken {
        fn parse_num(input: &str) -> Self {
            Self::Num(input.parse().unwrap())
        }
    }
    ```

## Json Lexis Example.

```rust
#[derive(Token)]
#[define(DEC = ['0'..'9'])]
#[define(HEX = DEC | ['A'..'F'])]
#[define(POSITIVE = ['1'..'9'] & DEC*)]
#[define(ESCAPE = '\\' & (
      ['"', '\\', '/', 'b', 'f', 'n', 'r', 't']
    | ('u' & HEX & HEX & HEX & HEX)
))]
enum JsonToken {
    #[rule("true")]
    True,

    #[rule("false")]
    False,

    #[rule("null")]
    Null,

    #[rule('{')]
    BraceOpen,

    #[rule('}')]
    BraceClose,

    #[rule('[')]
    BracketOpen,

    #[rule(']')]
    BracketClose,

    #[rule(',')]
    Comma,

    #[rule(':')]
    Colon,

    #[rule('"' & (ESCAPE | ^['"', '\\'])* & '"')]
    String,

    #[rule('-'? & ('0' | POSITIVE) & ('.' & DEC+)? & (['e', 'E'] & ['-', '+']? & DEC+)?)]
    Number,

    #[rule([' ', '\t', '\n', '\x0c', '\r']+)]
    Whitespace,

    #[mismatch]
    Mismatch,
}
```
