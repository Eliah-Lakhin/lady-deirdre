////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

//todo consider replacing HashMap with AHashMap

//TODO check warnings regularly
#![allow(warnings)]

//! # Lady Deirdre Macros Crate
//!
//! This is a helper crate for the [main crate](https://docs.rs/lady-deirdre/latest/lady_deirdre/)
//! of Lady Deirdre, compiler front-end foundation technology.
//!
//! The derive macros in this crate offer default implementations for
//! the [Token] (lexical scanner), [Node] (syntax parser), and [Feature] (semantic object)
//! traits used by the main crate.
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
//! To copy, use, distribute, and contribute to this work, you must agree to the
//! terms and conditions of the [General License Agreement](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md).
//!
//! Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин). All rights reserved.

extern crate core;
extern crate proc_macro;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use std::str::FromStr;

use proc_macro2::TokenStream;
use quote::ToTokens;

use crate::{feature::FeatureInput, node::NodeInput, token::TokenInput, utils::system_panic};

mod feature;
mod node;
mod token;
mod utils;

/// A canonical implementation of Lady Deirdre's lexical scanner.
///
/// This macro implements a Token trait on the enum type,
/// where the enum variants denote individual variants of the token with
/// the regular expressions defining their parsing rules.
///
/// The generated scanner is a minimal finite state machine that unions the
/// specified regular expression rules.
///
/// ## Macro Application Outline
///
/// ```ignore
/// // Copy and Eq implementations are required by the Token trait.
/// #[derive(Token, Clone, Copy, PartialEq, Eq)]
///
/// // U8 representation is required by the Token macro.
/// #[repr(u8)]
///
/// // An optional instruction that alternates the macro output.
/// //
/// // Possible <mode> values are:
/// //
/// //  - The `output` mode or nothing.
/// //    Prints the full macro output to the terminal using panic.
/// //
/// //  - The `meta` mode.
/// //    Prints the generator's metadata such as the time the generator spent
/// //    to optimize the scanner's state machine.
/// //
/// //  - The `dry` mode.
/// //    Checks correctness of the macro application, but does not produce any
/// //    output.
/// //
/// //  - The `decl` mode.
/// //    Produces the normal output of the macro with all Rust spans erased.
/// //    This is useful when the macro is being applied inside the declarative
/// //    macro.
/// #[dump(<mode>)]
///
/// // An optional instruction that sets the state machine optimization strategy.
/// //
/// // Possible <strategy> values are:
/// //
/// //  - The `flat` strategy. Uses a heuristic approach to optimize
/// //    the state machine. The result is almost always the same as with
/// //    the "deep" strategy, but there is no strong guarantee that
/// //    the finite state machine will be optimized to the minimal form.
/// //
/// //  - The `deep` strategy. Guarantees to optimizes the finite state-machine
/// //    to the canonical form.
/// //
/// // The default value is `flat` for the debug target (`debug_assertions`
/// // feature is enabled); otherwise, the strategy is `deep`.
/// #[opt(<strategy>)]
///
/// // An optional lookback attribute that sets the Token::LOOKBACK value.
/// //
/// // This value denotes the number of Unicode characters the scanner needs
/// // to step back to rescan a fragment of the source code text.
/// //
/// // When omitted, the value set to 1 by default.
/// #[lookback(1)]
///
/// // Optional inline expressions that you can use inside other expressions
/// // by name (specified before the "=" sign): `Foo | 'x' & Bar`.
/// //
/// // You can refer to the inline expression only after the definition
/// // (recursive application is not possible).
/// //
/// // The names must be unique in the namespace of other inline expressions
/// // and the enum variants.
/// #[define(Foo = <reg expr>)]
/// #[define(Bar = <reg expr>)]
/// enum MyToken {
///     // A variant with discriminant 0 is required.
///     // This variant denotes an end-of-input token (`Token::eoi()` value).
///     EOI = 0,
///
///     // A variant with discriminant 1 is required.
///     // This variant denotes the source code text fragments that cannot
///     // be recognized by the scanner (`Token::mismatch()` value).
///     Unknown = 1,
///
///     // Required for the parsable variants.
///     // Specifies the token scanning expression.
///     #[rule(<reg expr>)]
///
///     // Optional.
///     //
///     // Calls the `<rust expr>` expression when the `<reg expr>` matches
///     // the scanned fragment.
///     //
///     // The `<rust expr>` must return a valid token variant that corresponds
///     // to this fragment. The `fragment` variable of type `&str` can be used
///     // inside the constructor expression.
///     #[constructor(<rust expr>)]
///
///     // Optional.
///     //
///     // Specifies the value of the `Token::describe` function that returns
///     // an end-user display description of the token variant.
///     //
///     // When two strings provided, the second string corresponds to
///     // the verbose version of the description.
///     //
///     // When only one string provided, the string specifies the short and
///     // verbose descriptions both.
///     //
///     // If the macro attribute omitted, the `Token::describe` would
///     // return None for this token variant.
///     #[describe("short", "verbose")]
///
///     // Optional.
///     //
///     // Specifies the priority of the rule over other rules.
///     // This helps the scanner to resolves ambiguities between the scanning
///     // rules when several rules could match the same string fragments.
///     //
///     // For example, the general identifier scanning rule could conflict
///     // with the keyword rule. In this case, the keyword rule should have
///     // a higher priority.
///     //
///     // Rules with higher priority values supersede the rules with lower
///     // priority values.
///     //
///     // The default priority is zero.
///     #[priority(<signed integer number>)]
///
///     // The `= <num>` discriminant is optional but if specified,
///     // it will match the `Token::rule()` value.
///     ParsableVariant,
///
///     // Variants without the `#[rule(...)]` macro attribute are allowed.
///     //
///     // They will not be scanned by the generated scanner, but you can
///     // return them from the constructor expressions of the parsable variant.
///     //
///     // The `#[describe(...)]` macro attribute is also allowed for
///     // the variants without rules.
///     UnparseableVariant,
/// }
/// ```
///
/// ## Regular Expressions
///
/// ### Example
///
/// ```ignore
/// | "word"
/// | 'w' & 'o' & 'r' & 'd'
/// | 'x' & ('y' | 'z')
/// | ['1'..'9', 'X', 'a'..'c']
/// | "optional"?`
/// | "zero or more repetition"*
/// | "one or more repetition"+
/// ```
///
/// ### Precedence
///
/// The `&` concatenation operator has a higher priority over
/// the `|` alternation operator.
///
/// The unary operators (`+`, `*`, `?`) have the highest priority.
///
/// The binary operators (`|` and `&`) are left associative.
///
/// Parentheses `(<reg expr>)` group the inner expressions.
///
/// The `&` operator can be omitted: `"foo" & "bar"` means the same as `"foo" "bar"`.
///
/// The alternate expressions (denoted by the `|` operator) can start with
/// the pipe character like in the example above.
///
/// ### Requirements
///
/// The expression specified in the `#[rule(<reg expr>)]` macro attribute
/// must match at least one character.
///
/// The inline expression `#[define(<name> = <reg expr>)]` could match
/// empty strings.
///
/// The parsable variants with the same priority must match distinct string
/// fragments. The priority could be overridden using the `#[priority(<num>)]`
/// macro attribute.
///
/// ### Debugging
///
/// The "dump" operator (e.g., `"foo" | dump("b" & "a" & "r")`) enforces the
/// macro program to print the surrounding regular expression state machine
/// transitions to the terminal using panic.
///
/// ### Operators
///
///  - String fragment: `"foo"`. Matches a sequence of the Unicode characters
///    denoted by the string literal.
///
///  - Single character: `'Y'`. Matches a single Unicode character.
///
///  - Any Unicode character: `.`. Matches any single Unicode character.
///
///  - Any character in the set: `['a', 'c', '1'..'8']`.
///    Matches any Unicode character within the specified set. The character
///    ranges (`'1'..'8'`) denote the Unicode characters in the range starting
///    from the lower bound to the upper bound **inclusive**. The lower bound
///    must be less than or equal to the upper bound.
///
///  - Any character outside of the set: `^['a', 'c', '1'..'8']`.
///    The inverted version of the previous operator that matches any character
///    outside of the specified set.
///
///  - Any Unicode uppercase character: `$upper`.
///
///  - Any Unicode lowercase character: `$lower`.
///
///  - Any Unicode numeric character: `$num`.
///
///  - Any Unicode whitespace character: `$space`.
///
///  - Any Unicode alphabetic character: `$alpha`.
///
///  - Any Unicode identifier's start character: `$xid_start`.
///
///  - Any Unicode identifier's continuation character: `$xid_continue`.
///
///  - A class of the character property combinations: `${alpha | num | space}`.
///    The property names are any combinations of the names listed above above.
///
///  - A concatenation of the rules: `<expr1> & <expr2>` or just `<expr1> <expr2>`.
///    Matches `<expr1>`, then matches `<expr2>`. The concatenation expression
///    matches the string fragment if and only if both operands match
///    the substrings of the fragment string.
///
///  - A union of the rules: `<expr1> | <expr2>` or `| <expr1> | <expr2>`.
///    Matches either `<expr1>` or `<expr2>`. The union expression matches
///    the string fragment if and only if at least one operand matches this
///    fragment.
///
///  - Non-empty repetition: `<expr>+`. Applies the `<expr>` rule one or more
///    times. The repetition expression matches the string fragment if and only
///    if at least one application of the `<expr>` rule is satisfied.
///
///  - Possibly empty repetition: `<expr>*`. Applies the `<expr>` rule zero or
///    more times. If the `<expr>` cannot be applied one or more times,
///    the operator matches an empty string.
///
///  - Optional application: `<expr>?`. Attempts to apply the `<expr>` rule.
///    If the `<expr>` cannot be applied, the operator matches an empty string.
///
///  - Inline expression: `FOO`. Inlines the expression defined previously using
///    the `#[define(FOO = <expr>)]` macro attribute.
///
///  - Debug dump: `dump(<expr>)`. Enforces the macro program to print the state
///    machine transitions of the `<expr>` rule to the terminal.
#[proc_macro_derive(
    Token,
    attributes(define, lookback, rule, priority, constructor, describe, opt, dump)
)]
pub fn token(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as TokenInput);

    let declarative = input.dump.is_declarative();

    output_stream(declarative, input.into_token_stream())
}

/// A canonical implementation of Lady Deirdre's syntax parser.
///
/// This macro implements the syntax component (Node and AbstractNode traits)
/// and the semantic entry points (Grammar and AbstractFeature traits)
/// of the programming language grammar on enum types.
///
/// The enum variants denote individual syntax tree node variants, and their
/// parsing rules. The enum variant fields represent the state of the node.
/// In particular, through the variant fields, the syntax tree establishes
/// parent-child relations between the tree nodes.
///
/// The parsing rules are described in terms of the LL(1) grammar expressions,
/// but the macro enables the possibility to implement the individual node
/// parsing procedure using the user-defined function where you can implement
/// custom  recursive-descend parsing logic with potentially unlimited lookahead
/// and the left recursion.
///
/// The generated parser is capable of automatically recovering from
/// the syntax errors.
///
/// ## Macro Application Outline
///
/// ```ignore
/// #[derive(Node)]
///
/// // Required.
/// //
/// // Denotes the lexical component of the grammar (`Node::Token` type).
/// #[token(MyToken)]
///
/// // Optional.
/// //
/// // Sets the syntax tree classifier type (`Grammar::Classifier` type).
/// //
/// // When omitted, the classifier is set to the VoidClassifier.
/// #[classifier(<classifier type>)]
///
/// // Optional.
/// //
/// // Defines the expression that will be automatically parsed zero or more
/// // times between every consumed token in the node's parse rules.
/// //
/// // The trivia expression usually enumerates whitespace tokens, comment
/// // rules and similar syntactically useless things
/// // (e.g., `#[trivia($Whitespace | InlineComment)`).
/// //
/// // The <parse expr> expression is allowed to parse an empty sequence of
/// // tokens.
/// //
/// // When omitted, the default trivia expression is an empty expression.
/// //
/// // You can manually override trivia expression of each parsable rule.
/// #[trivia(<parse expr>)]
///
/// // Optional.
/// //
/// // Defines panic error recovery configuration of the parsable rules.
/// //
/// // By default, the panic recovery is unlimited (`Recovery::unlimited()`).
/// //
/// // Using this macro attribute you can specify the recovery halting
/// // tokens (the `Recovery::unexpected` tokens), and the token groups
/// // (the `Recovery::group` pairs).
/// //
/// // The <config> is a sequence of elements delimited by `,` comma, where each
/// // element is either a halting `$Token`, or a group pair `[$Start, $End]`.
/// //
/// // Example: `#[recovery($Semicolon, [$OpenBrace, $CloseBrace])]`.
/// //
/// // You can manually override the recovery configuration of each parsable rule.
/// #[recovery(<config>)]
///
/// // An optional instruction that alternates the macro output.
/// //
/// // Possible <mode> values are:
/// //
/// //  - The `output` mode or nothing.
/// //    Prints the full macro output to the terminal using panic.
/// //
/// //  - The `trivia` mode.
/// //    Prints the parser's common trivia parsing function (the function
/// //    generated from the `#[trivia(...)]` expression of the enum type).
/// //
/// //  - The `meta` mode.
/// //    Prints the generator's metadata such as the time the generator spent
/// //    to statically optimize the syntax parser.
/// //
/// //  - The `dry` mode.
/// //    Checks correctness of the macro application, but does not produce any
/// //    output.
/// //
/// //  - The `decl` mode.
/// //    Produces the normal output of the macro with all Rust spans erased.
/// //    This is useful when the macro is being applied inside the declarative
/// //    macro.
/// #[dump(<mode>)]
///
/// // Optional inline expressions that you can use inside other expressions
/// // by name (specified before the "=" sign): `Foo | $Token & Bar`.
/// //
/// // You can refer to the inline expression only after the definition
/// // (recursive application is not possible).
/// //
/// // The names must be unique in the namespace of other inline expressions
/// // and the enum variants.
/// #[define(Foo = <parse expr>)]
/// #[define(Bar = <parse expr>)]
///
/// enum MyNode {
///     // Must be applied to exactly one parseable variant that represents
///     // the root node of the syntax tree.
///     #[root]
///
///     // Optional if the variant has a #[denote(...)] attribute.
///     //
///     // Specifies the parsing rule of the variant.
///     //
///     // The macro uses this expression to generate the parser, and
///     // to reveal the leftmost set of tokens of the parsing rule.
///     #[rule(<parse expr>)]
///
///     // Optional.
///     //
///     // Overrides the parser generated by the macro with the user-defined
///     // parser. Only applicable when the variant has a #[rule(...)] attribute
///     // that would define the leftmost token set of the parser.
///     //
///     // The <rust expr> must return an instance of the Node. Inside the
///     // <rust expr> you can use the "session" variable which is a mutable
///     // reference to the SyntaxSession of the current parsing state from
///     // which you should parse the node.
///     //
///     // Typically, inside the <rust expr> you would call a user-defined
///     // parser function with the "session" argument.
///     #[parser(<rust expr>)]
///
///     // Optional if the variant has a #[rule(...)] attribute.
///     //
///     // Specifies the NodeRule number of this variant.
///     //
///     // Possible syntax is `<const_name>`, `<const_name> = <int_value>`,
///     // or `<int_value>`.
///     //
///     // When the <const name> specified, the macro will generate type's
///     // constant with the value: `MyNode::Foo == 10`.
///     //
///     // When the <int_value> specified, the number must be unique across
///     // all denoted variants with the #[denote(...)] attribute.
///     //
///     // If the <int_value> omitted, the macro will assign the unique value
///     // automatically.
///     #[denote(FOO = 10)]
///
///     // Optional if the variant has a #[rule(...)] attribute.
///     //
///     // Specifies the value of the `AbstractNode::describe` function that
///     // returns an end-user display description of the node variant.
///     //
///     // When two strings provided, the second string corresponds to
///     // the verbose version of the description.
///     //
///     // When only one string provided, the string specifies the short and
///     // verbose descriptions both.
///     //
///     // If the macro attribute omitted, the `AbstractNode::describe` would
///     // return None for this node variant.
///     #[describe("short", "verbose")]
///
///     // Optional. Only applicable when the variant has a #[rule(..)]
///     // attribute, and does not have overridden parser.
///     //
///     // Overrides default variant's constructor. The <rust expr> must return
///     // an instance of the Node. Inside the <rust expr> you can use variables
///     // of the rule expression capture keys and the "session" variable of
///     // the SyntaxSession type from which you can read the current parsing
///     // state.
///     #[constructor(some_constructor(session, foo, bar, baz))]
///
///     // Optional. Only applicable when the variant has a #[rule(..)]
///     // attribute, and does not have overridden parser.
///     //
///     // Overrides trivia parsing expression of this node. See #[trivia(...)]
///     // type attribute description above for details.
///     #[trivia(<parse expr>)]
///
///     // Optional. Only applicable when the variant has a #[rule(..)]
///     // attribute, and does not have overridden parser.
///     //
///     // Overrides recovery configuration of the generated parser of this
///     // node. See #[recovery(...)] type attribute description above
///     // for details.
///     #[recovery(<config>)]
///
///     // Optional.
///     //
///     // Instructs the macro that the generated parsers should bypass
///     // caching of this node when descending into the parser of this rule.
///     //
///     // When omitted, the macro will consider that the variant's parser is
///     // "primary", and it will enforce the parsing environment to cache the
///     // node whenever possible.
///     #[secondary]
///
///     // Optional.
///     //
///     // Tells the macro that this Node variant is the root of the semantics
///     // scope branch (the `Grammar::is_scope()` function would return true
///     // for such variants).
///     #[scope]
///
///
///     // An optional instruction that enforce the macro to print debug
///     // metadata for this node.
///     //
///     // Possible <mode> values are:
///     //
///     //  - The `output` mode or nothing.
///     //    Prints the generated parsing function to the terminal using panic.
///     //
///     //  - The `trivia` mode.
///     //    Prints the parser's overridden trivia parsing function
///     //    (the function generated from the `#[trivia(...)]` expression of
///     //    the variant).
///     #[dump(<mode>)]
///
///     Variant {
///         // Optional. Sets the reference of this node in the syntax tree.
///         #[node]
///         node: NodeRef,
///
///         // Optional. Sets the reference to the parent node of this node
///         // in the syntax tree
///         #[parent]
///         parent: NodeRef,
///
///         // Optional.
///         //
///         // A child of this node.
///         //
///         // The field name must match one of the capturing operator's keys
///         // from the rule's expression (e.g., "foo: FooNode") if the variant
///         // has a #[rule(...)] attribute which was not overridden by
///         // the #[parser(...)] attribute.
///         //
///         // The type of the field is NodeRef, TokenRef, Vec<NodeRef>,
///         // or Vec<TokenRef> depending on the capture type and repetition.
///         //
///         // Note that all captures specified in the #[rule(...)] attribute
///         // must be covered by the variant fields unless the variant has
///         // an overridden constructor (via the #[constructor(...)]
///         // attribute). The overridden constructor allows to change
///         // the logic of the variant fields initialization.
///         #[child]
///         foo: NodeRef,
///
///         // Optional unless any other denoted variant already has
///         // a #[semantics] field.
///         //
///         // Specifies the semantic entry-point of this node variant.
///         //
///         // The type of the field must be Semantics type parametrized with
///         // the Feature implementation describing the semantics of the node.
///         //
///         // If the variant does not have semantics you can use
///         // `Semantics<VoidFeature<MyNode>>` as the field type.
///         #[semantics]
///         semantics: Semantic<VariantSemantics>,
///
///         // Required for arbitrary fields of the node variants.
///         //
///         // The <rust expr> specifies a constructor of the field value,
///         // that the generated parser will use to initialize the field.
///         //
///         // The <rust expr> could be omitted (`#[default]`). In this case
///         // the parser will use the Default implementation of the field's type.
///         //
///         // This macro attribute is only applicable when the variant
///         // has a #[rule(...)] attribute, and it does not have overridden
///         // #[constructor(...)] or overridden #[parser(..)] attributes
///         // because the overridden the Constructor or Parser are specifying
///         // the initialization logic explicitly.
///         #[default(<rust expr>)]
///         custom_field: CustomType,
///     },
/// }
/// ```
///
/// ### Variants Denotation
///
/// Denoted enum variants are variants that either have
/// a `#[rule(...)]` macro attribute or at least a `#[denote(...)]` attribute.
///
/// Denoted variants are eligible variants of the nodes of the syntax tree.
///
/// The macro allows you to describe their field structure (i.e., to annotate
/// the fields with the `#[parent]`, `#[child]`, and other macro attributes),
/// and it will generate the traits functions in accordance with these
/// descriptions.
///
/// The denotation of the variant is important because in this case, the variant
/// at least receives a NodeRule number through which it could be referred to.
///
/// Non-denoted variants are just normal enum variants that don't relate
/// to the syntax. The macro ignores them.
///
/// ### Parsable variants
///
/// The macro variants considered to be "parsable" if they have a
/// `#[rule(...)]` attribute. These variants become implicitly "denoted".
///
/// Parseability is only makes sense for the Node macro logic because
/// by specifying the Rule attribute, you are exposing the leftmost set of
/// the variant's parser. The macro needs to know this set to properly implement
/// descend transitions between the parsing procedures when you refer this
/// variant in another parsing expression.
///
/// However, if you do not intend to refer to the variant in the parsing
/// expression, and you want to parse the variant manually in one of the custom
/// parsers, you only need to denote the variant (explicitly) using
/// the `#[denote(...)]` macro attribute.
///
/// ```ignore
/// #[derive(Node)]
/// enum MyNode {
///     // You can refer to this variant in another rule.
///     #[rule(...)]
///     DenotedVariant1 { ... },
///
///     // You can refer to this variant in another rule,
///     // and in the custom parsers using the `MyNode::VARIANT_2` rule number.
///     #[rule(...)]
///     #[denote(VARIANT_2)]
///     #[describe(...)]
///     DenotedVariant2 { ... },
///
///     // You cannot refer to this variant in another rule,
///     // but you can refer to it in the custom parsers using
///     // the `MyNode::VARIANT_3` rule number.
///     #[denote(VARIANT_3)]
///     #[describe(...)]
///     UnparsableDenotedVariant { ... },
/// }
/// ```
///
/// ### Custom Parsers
///
/// By default, the macro generates a parsing function for each parsable
/// variant (a variant with the `#[rule(...)]` macro attribute) based on the
/// parsing expression specified in the rule.
///
/// You can manually override the function with your own custom parsing
/// function using the `#[parser(...)]` macro attribute.
///
/// ```ignore
/// #[derive(Node)]
/// enum MyNode {
///     #[rule(...)]
///     #[parser(custom_parser(session))]
///     ParsableVariant { ... },
/// }
///
/// fn custom_parser<'a>(session: &mut impl SyntaxSession<'a, MyNode>) -> MyNode {
///     // custom parser implementation of the MyNode::ParsableVariant variant
/// }
/// ```
///
/// Inside the parser's expression, you can use the "session" variable, which is
/// a reference to the current state of the SyntaxSession from which the node
/// should be parsed. The expression must return an instance of the parsable
/// node variant. Usually, this expression is a custom parsing function call.
///
/// Note that even though the parser is customized, the `#[rule(...)]` expression
/// is still required. You don't need to implement the full parser in the rule
/// expression, but you should enumerate the leftmost tokens so that the macro
/// will be aware of how to descend into the node's parser. Usually, you can just
/// enumerate the tokens with the union operator: `#[rule($TokenA | $TokenB | $TokenC)]`.
///
/// ### Ascending Relations
///
/// It is recommended that each denoted variant would have `#[node]` and
/// `#[parent]` variant fields.
///
/// The `#[node]` field is the NodeRef that will point to this node instance.
///
/// The `#[parent]` field is the NodeRef that will point to the parent node
/// instance of this node.
///
/// The macro-generated parser of the variant will automatically set these
/// values. Inside the custom parser, you should set them manually (you can use
/// the `SyntaxSession::node()` and `SyntaxSession::parent` functions for this
/// purpose).
///
/// These field values could then be fetched using
/// the `AbstractNode::node_ref()` and `AbstractNode::parent_ref()` functions
/// accordingly. The parsing environment would use
/// the `AbstractNode::set_parent_ref()` function to update the parent reference
/// during incremental reparsing when the reparser attempts to "transplant"
/// the branch of the syntax tree.
///
/// The macro requires that either all denoted variants have `#[node]` and
/// `#[parent]` fields, or none of them.
///
/// ```ignore
/// #[derive(Node)]
/// enum MyNode {
///     #[rule(...)]
///     Variant1 {
///         #[node]
///         node: NodeRef,
///         #[parent]
///         parent: NodeRef,
///     },
///
///     #[rule(...)]
///     Variant2 {
///         #[node]
///         node: NodeRef,
///         #[parent]
///         parent: NodeRef,
///     },
/// }
/// ```
///
/// ### Descending Relations
///
/// The parent-child relations are established through the system of captures.
///
/// When the parsing expression reads a token or descends into another rule,
/// you can capture the TokenRef / NodeRef reference of the result and
/// put it into the variant fields annotated with the `#[child]` macro
/// attribute.
///
/// ```ignore
/// #[derive(Node)]
/// enum MyNode {
///     #[rule(foo: $FooToken & bar: BarNode? & baz: BazNode*)]
///     Variant {
///         #[child]
///         foo: TokenRef,
///         #[child]
///         bar: NodeRef,
///         #[child]
///         baz: Vec<NodeRef>,
///     },
/// }
/// ```
///
/// When the expression captures a token, it should be put into the TokenRef
/// field. When it captures a node, it should be put into the NodeRef field.
///
/// If the expression could be captured more than once (like a `baz: BazNode*`
/// in the example above), it should be put into Vec.
///
/// If the expression capturing is optional (`bar: BarNode?` captures no more
/// than once), the field type should still be the TokenRef / NodeRef. When
/// the parser didn't capture the value, it will set the field to
/// the `TokenRef::nil()` / `NodeRef::nil()` accordingly.
///
/// The custom user-defined parser should follow this convention too.
///
/// The `#[child]` macro attribute informs the macro that the field is subject
/// to capturing. In particular, the implementation of `AbstractNode::capture`
/// and related functions uses these fields to represent the AbstractNode’s
/// captures.
///
/// Properly denoting children is particularly important for a number of
/// built-in features, including syntax tree traversal
/// (SyntaxTree::traverse_tree()), which rely on the children metadata.
///
/// ### Semantics
///
/// The macro automatically implements the Grammar and AbstractFeature traits
/// for the derived enum type, making the type eligible for the Analyzer.
///
/// To bind the semantic entry points, you should specify the `#[semantics]`
/// variant fields in all denoted variants.
///
/// Even if the variant does not have semantic features, you have to implement
/// the field for semantic consistency. In this case, you can use
/// the VoidFeature helper type.
///
/// ```ignore
/// #[derive(Node)]
/// enum MyNode {
///     #[rule(...)]
///     Variant1 {
///         #[semantics]
///         semantics: Semantics<Variant1Semantics>,
///     },
///
///     #[rule(...)]
///     Variant2 {
///         #[semantics]
///         semantics: Semantics<VoidFeature<MyNode>>,
///     }
/// }
/// ```
///
/// The macro requires that either all denoted variants have semantics, or none
/// of them.
///
/// You may have at most one variant field annotated with the `#[semantics]`
/// macro attribute.
///
/// ### Error Recovery
///
/// The parsers generated by the macro are subject for error recovery, which
/// is a heuristic process based on the static analysis of the specified
/// syntax rules.
///
/// The exact procedure is not specified and could change over time in
/// the minor versions of the crate to improve the recovery logic.
///
/// The recovery mechanism uses at least the "panic" recovery approach when
/// appropriate. As the author of the syntax grammar, you can specify
/// the panic recovery configurations for the entire grammar or
/// per individual variants using the `#[recovery(<config>)]` attribute.
///
/// ## Parsing Expressions
///
/// Parsing expressions are regex-like expressions that describe the parsing
/// rules in terms of LL(1) grammars.
///
/// ### Example
///
/// ```ignore
/// | $TokenA & ($TokenB & VariantX) & VariantY
/// | $OptionalToken?
/// | ZeroOrMoreRepetition*
/// | ZeroOrMoreRepetition*{$WithDelimiter}
/// | OneOrMoreRepetition+
/// | OneOrMoreRepetition+{$WithDelimiter}
/// | token_capture: $SomeToken
/// | many_nodes_capture: SomeVariant*
/// ```
///
/// ### Precedence
///
/// The `&` concatenation operator has a higher priority over
/// the `|` alternation operator.
///
/// The unary operators (`+`, `*`, `?`) have higher priority than
/// the `&` concatenation operator.
///
/// The capturing operator (`a: Foo`) has the highest priority.
///
/// The binary operators (`|` and `&`) are left associative.
///
/// Parentheses `(<parse expr>)` group the inner expressions.
///
/// The `&` operator can be omitted: `Foo & Bar` means the same as `Foo Bar`.
///
/// The alternate expressions (denoted by the `|` operator) can start with
/// the pipe character like in the example above.
///
/// ### Requirements
///
/// The expression specified in the `#[rule(<parse expr>)]` macro attribute
/// must match at least one token (possibly implicitly via descending into
/// other parse rules).
///
/// The inline expression `#[define(<name> = <parse expr>)]` could match
/// empty token sequences.
///
/// Left recursion is forbidden: the `#[rule(...)]` expression cannot
/// descend into itself as the first step of the parsing, neither directly nor
/// indirectly through other rules.
///
/// Each parsing expression has an associated _leftmost set_, a set of tokens
/// through which the parser starts to parse the expression (possibly indirectly
/// by descending into another parse rule).
///
/// The generated parser makes a decision to descend into the subrules based
/// on the leftmost set of the subrules. Therefore, rules that descend must be
/// unambiguous in the parsing step.
///
/// For example, the expression `$Foo | Bar` would be ambiguous if the Bar's
/// variant has a `$Foo` token in its leftmost set.
///
/// ### Debugging
///
/// The "dump" operator (e.g., `$Foo | dump($A & x: B & Y: $C)`) enforces the
/// macro program to print the surrounding expression's inner state machine
/// transitions, leftmost set, and the captures to the terminal using panic.
///
/// ### Operators
///
///  - Single token match: `$SomeToken`.
///
///  - Any token match: `.`. Matches any single token from the alphabet of
///    available tokens.
///
///  - Any token except the tokens in the set: `^[$TokenA | $TokenB | $TokenC]`.
///    Matches any single token from the alphabet of available tokens, except
///    the enumerated tokens.
///
///  - Descending or inline: `Foo`. If "Foo" is a parsable variant, descends
///    into this variant's parsing rule. If "Foo" is an inline expression
///    defined with the `#[define(Foo = <expr>)]` macro attribute, copies
///    this `<expr>` expression in place as it is.
///
///  - A concatenation of the rules: `<expr1> & <expr2>` or just `<expr1> <expr2>`.
///    Matches `<expr1>`, then matches `<expr2>`. The concatenation expression
///    matches the token sequence if and only if both operands match
///    the subsequences of the sequence.
///
///  - A union of the rules: `<expr1> | <expr2>` or `| <expr1> | <expr2>`.
///    Matches either `<expr1>` or `<expr2>`. The union expression matches
///    the token sequence if and only if at least one operand matches this
///    sequence.
///
///  - Non-empty repetition: `<expr>+`. Applies the `<expr>` rule one or more
///    times. The repetition expression matches the token sequence if and only
///    if at least one application of the `<expr>` rule is satisfied.
///
///  - Non-empty repetition with delimiter: `<expr>+{<del_expr>}`. Same as
///    the normal non-empty repetition, but requires the `<del_expr>`
///    expression to be present between each match of the `<expr>` expression.
///
///  - Possibly empty repetition: `<expr>*`. Applies the `<expr>` rule zero or
///    more times. If the `<expr>` cannot be applied one or more times,
///    the operator matches an empty token sequence.
///
///  - Possibly empty repetition with delimiter: `<expr>*{<del_expr>}`. Same as
///    the normal possibly empty repetition, but requires the `<del_expr>`
///    expression to be present between each match of the `<expr>` expression.
///
///  - Optional application: `<expr>?`. Attempts to apply the `<expr>` rule.
///    If the `<expr>` cannot be applied, the operator matches an token sequence.
///
///  - Result capture: `<key>: <expr>`. If `<expr>` is a token match, matches
///    and captures the TokenRef of the token. If `<expr>` is a subrule
///    descending, descends into the rule, and captures the NodeRef result of
///    this rule. Otherwise, if `<expr>` is a complex expression, spreads
///    the capturing operator to all inner token matches and rule descendings.
///    The `<key>` is an identifier of the capture (basically, variant field's
///    name) to which the captured value should be assigned or pushed.
///
///  - Debug dump: `dump(<expr>)`. Enforces the macro program to print
///    the inner state machine transitions, leftmost set, and the captures
///    of the surrounding `<expr>` expression to the terminal.
#[proc_macro_derive(
    Node,
    attributes(
        token,
        classifier,
        define,
        trivia,
        recovery,
        rule,
        root,
        denote,
        constructor,
        secondary,
        parser,
        default,
        node,
        parent,
        child,
        semantics,
        describe,
        scope,
        dump,
    )
)]
pub fn node(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as NodeInput);

    let declarative = input.dump.is_declarative();

    output_stream(declarative, input.into_token_stream())
}

/// A canonical implementation of Lady Deirdre's syntax tree's node semantic
/// object.
///
/// This macro implements the Feature and AbstractFeature traits on the struct
/// types, making these types eligible for use as the generic parameter of
/// the Semantics object and as field types of other Feature objects.
///
/// ## Macro Application Outline
///
/// ```ignore
/// #[derive(Feature)]
///
/// // Specifies the grammar to which this Feature belongs (`Feature::Node` type).
/// #[node(MyNode)]
///
/// // An optional instruction that alternates the macro output.
/// //
/// // Possible <mode> values are:
/// //
/// //  - The `output` mode or nothing.
/// //    Prints the full macro output to the terminal using panic.
/// //
/// //  - The `dry` mode.
/// //    Checks correctness of the macro application, but does not produce any
/// //    output.
/// //
/// //  - The `decl` mode.
/// //    Produces the normal output of the macro with all Rust spans erased.
/// //    This is useful when the macro is being applied inside the declarative
/// //    macro.
/// #[dump(<mode>)]
///
/// // The macro exposes the inner fields of the struct through
/// // the `AbstractFeature::feature()` and `AbstractFeature::feature_keys()`
/// // function that have the same visibility as the type's visibility.
/// pub(super) struct SomeFeature {
///     // Will be exposed.
///     pub(super) foo: Attr<FooFn>,
///
///     // Will not be exposed, visibility is different from the type's visibility.
///     bar: Attr<BarFn>,
///
///     // BazFeature must also implement the Feature and AbstractFeature traits too.
///     pub(super) baz: BazFeature,
///
///     // This macro attribute is optional, and denotes that the semantics
///     // Attribute or a Feature is subject to invalidation when
///     // the `Feature::invalidate()` function is called for the "SomeFeature"
///     // type.
///     //
///     // This attribute is only makes sense to use if the "SomeFeature"
///     // will be used as a part of the semantics of the syntax tree node
///     // denoted as a scope; otherwise, the #[scoped] marker will be ignored.
///     //
///     // In practice, you should only annotate the fields of the struct with
///     // the #[scoped] macro attribute that supposed to be entry points of
///     // the semantic model.
///     #[scoped]
///     pub(super) scoped_attr: Attr<InputFn>,
/// }
/// ```
///
/// Structs with anonymous fields are also derivable:
///
/// ```ignore
/// #[derive(Feature)]
/// #[node(MyNode)]
/// pub(super) struct SomeFeature(
///     pub(super) Attr<FooFn>,
///     Attr<BarFn>,
///     pub(super) BazFeature,
///     #[scoped] pub(super) Attr<InputFn>,
/// );
/// ```
#[proc_macro_derive(Feature, attributes(node, scoped, dump))]
pub fn feature(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as FeatureInput);

    let declarative = input.dump.is_declarative();

    output_stream(declarative, input.into_token_stream())
}

fn output_stream(declarative: bool, stream: TokenStream) -> proc_macro::TokenStream {
    match declarative {
        true => match TokenStream::from_str(&stream.to_string()) {
            Ok(stream) => stream.into(),
            Err(error) => system_panic!("Spans erasure failure. {error}",),
        },
        false => stream.into(),
    }
}
