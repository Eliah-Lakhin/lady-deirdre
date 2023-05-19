////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" Work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This Work is a proprietary software with source available code.            //
//                                                                            //
// To copy, use, distribute, and contribute into this Work you must agree to  //
// the terms of the End User License Agreement:                               //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The Agreement let you use this Work in commercial and non-commercial       //
// purposes. Commercial use of the Work is free of charge to start,           //
// but the Agreement obligates you to pay me royalties                        //
// under certain conditions.                                                  //
//                                                                            //
// If you want to contribute into the source code of this Work,               //
// the Agreement obligates you to assign me all exclusive rights to           //
// the Derivative Work or contribution made by you                            //
// (this includes GitHub forks and pull requests to my repository).           //
//                                                                            //
// The Agreement does not limit rights of the third party software developers //
// as long as the third party software uses public API of this Work only,     //
// and the third party software does not incorporate or distribute            //
// this Work directly.                                                        //
//                                                                            //
// AS FAR AS THE LAW ALLOWS, THIS SOFTWARE COMES AS IS, WITHOUT ANY WARRANTY  //
// OR CONDITION, AND I WILL NOT BE LIABLE TO ANYONE FOR ANY DAMAGES           //
// RELATED TO THIS SOFTWARE, UNDER ANY KIND OF LEGAL CLAIM.                   //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this Work.                                                      //
//                                                                            //
// Copyright (c) 2022 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::fmt::{Display, Formatter};

use proc_macro2::{Ident, Span};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute,
    Error,
    Result,
};

use crate::{
    node::{
        builder::Builder,
        regex::{operand::RegexOperand, operator::RegexOperator, Regex},
    },
    utils::{debug_panic, PredictableCollection, Set, SetImpl},
};

#[derive(Clone)]
pub(in crate::node) struct Leftmost {
    span: Span,
    optional: bool,
    tokens: Set<Ident>,
    nodes: Set<Ident>,
}

impl Default for Leftmost {
    #[inline(always)]
    fn default() -> Self {
        Self {
            span: Span::call_site(),
            optional: false,
            tokens: Set::empty(),
            nodes: Set::empty(),
        }
    }
}

impl Spanned for Leftmost {
    fn span(&self) -> Span {
        self.span
    }
}

impl Display for Leftmost {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let mut tokens = self.tokens.iter().cloned().collect::<Vec<_>>();

        tokens.sort();

        for name in &tokens {
            writeln!(formatter, "    ${}", name)?;
        }

        let mut nodes = self.nodes.iter().cloned().collect::<Vec<_>>();

        nodes.sort();

        for name in &nodes {
            writeln!(formatter, "    {}", name)?;
        }

        Ok(())
    }
}

impl<'a> TryFrom<&'a Attribute> for Leftmost {
    type Error = Error;

    fn try_from(attribute: &'a Attribute) -> Result<Self> {
        enum TokenOrNode {
            Token(Ident),
            Node(Ident),
        }

        impl Parse for TokenOrNode {
            fn parse(input: ParseStream) -> Result<Self> {
                let lookahead = input.lookahead1();

                if input.peek(Token![$]) {
                    let _ = input.parse::<Token![$]>()?;

                    return Ok(Self::Token(input.parse::<Ident>()?));
                }

                if lookahead.peek(syn::Ident) {
                    return Ok(Self::Node(input.parse::<Ident>()?));
                }

                return Err(lookahead.error());
            }
        }

        let span = attribute.span();

        attribute.parse_args_with(|input: ParseStream| {
            let set = Punctuated::<TokenOrNode, Token![|]>::parse_terminated(input)?;

            if set.is_empty() || !input.is_empty() {
                return Err(
                    input.error("Expected $<Token> or <Node> sequence separated with pipe ('|').")
                );
            }

            let mut leftmost = Self {
                span,
                optional: false,
                tokens: Set::empty(),
                nodes: Set::empty(),
            };

            for entry in set {
                match entry {
                    TokenOrNode::Token(ident) => {
                        if leftmost.tokens.contains(&ident) {
                            return Err(Error::new(ident.span(), "Duplicate token."));
                        }

                        let _ = leftmost.tokens.insert(ident);
                    }

                    TokenOrNode::Node(ident) => {
                        if leftmost.nodes.contains(&ident) {
                            return Err(Error::new(ident.span(), "Duplicate node."));
                        }

                        let _ = leftmost.nodes.insert(ident);
                    }
                }
            }

            Ok(leftmost)
        })
    }
}

impl Leftmost {
    pub(in crate::node) fn append(&mut self, other: Self) {
        self.tokens.append(other.tokens);
        self.nodes.append(other.nodes);
    }

    #[inline(always)]
    pub(in crate::node) fn tokens(&self) -> &Set<Ident> {
        &self.tokens
    }

    #[inline(always)]
    pub(in crate::node) fn nodes(&self) -> &Set<Ident> {
        &self.nodes
    }

    #[inline(always)]
    pub(in crate::node) fn resolve(&mut self, builder: &mut Builder) -> Result<()> {
        for node in self.nodes.clone() {
            {
                let variant = match builder.get_variant(&node) {
                    Some(variant) => variant,

                    None => {
                        return Err(Error::new(
                            node.span(),
                            format!(
                                "Reference \"{}\" in the leftmost position leads to a left \
                                recursion. Left recursion is forbidden.",
                                node,
                            ),
                        ));
                    }
                };

                if let Some(resolution) = variant.get_leftmost() {
                    self.append(resolution.clone());
                    continue;
                }
            }

            builder.modify(&node, |builder, variant| variant.build_leftmost(builder))?;

            self.append(builder.variant(&node).leftmost().clone());
        }

        Ok(())
    }

    #[inline(always)]
    fn new_token(token: Ident) -> Self {
        Self {
            span: token.span(),
            optional: false,
            tokens: Set::new([token]),
            nodes: Set::empty(),
        }
    }

    #[inline(always)]
    fn new_node(node: Ident) -> Self {
        Self {
            span: node.span(),
            optional: false,
            tokens: Set::empty(),
            nodes: Set::new([node]),
        }
    }
}

impl RegexPrefix for Regex {
    fn leftmost(&self) -> Leftmost {
        match self {
            Self::Operand(RegexOperand::Unresolved { .. }) => debug_panic!("Unresolved operand."),

            Self::Operand(RegexOperand::Debug { inner, .. }) => inner.leftmost(),

            Self::Operand(RegexOperand::Token { name, .. }) => Leftmost::new_token(name.clone()),

            Self::Operand(RegexOperand::Rule { name, .. }) => Leftmost::new_node(name.clone()),

            Self::Unary {
                operator, inner, ..
            } => {
                let mut leftmost = inner.leftmost();

                match operator {
                    RegexOperator::ZeroOrMore { separator } => match leftmost.optional {
                        true => {
                            if let Some(separator) = separator {
                                leftmost.append(separator.leftmost());
                            }
                        }

                        false => leftmost.optional = true,
                    },

                    RegexOperator::OneOrMore { separator } => {
                        if leftmost.optional {
                            if let Some(separator) = separator {
                                let separator = separator.leftmost();

                                leftmost.optional = separator.optional;
                                leftmost.append(separator);
                            }
                        }
                    }

                    RegexOperator::Optional => leftmost.optional = true,

                    _ => debug_panic!("Unsupported Unary operator."),
                }

                leftmost
            }

            Self::Binary {
                operator,
                left,
                right,
            } => {
                let mut left = left.leftmost();

                match operator {
                    RegexOperator::Union => {
                        let right = right.leftmost();

                        left.optional = left.optional | right.optional;
                        left.append(right);

                        left
                    }

                    RegexOperator::Concat => {
                        if left.optional {
                            let right = right.leftmost();

                            left.optional = right.optional;
                            left.append(right);
                        }

                        left
                    }

                    _ => debug_panic!("Unsupported Binary operator."),
                }
            }
        }
    }
}

pub(in crate::node) trait RegexPrefix {
    fn leftmost(&self) -> Leftmost;
}
