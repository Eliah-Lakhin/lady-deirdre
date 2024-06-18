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

use std::{
    fmt::{Display, Formatter},
    mem::take,
};

use proc_macro2::Ident;

use crate::{
    node::{
        input::VariantMap,
        regex::{Operand, Operator, Regex},
        token::TokenLit,
    },
    utils::{expect_some, system_panic, PredictableCollection, Set, SetImpl},
};

#[derive(Clone, Default)]
pub(super) struct Leftmost {
    optional: bool,
    matches: Option<Set<TokenLit>>,
    tokens: Set<TokenLit>,
    nodes: Set<Ident>,
}

impl Display for Leftmost {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let mut tokens = self.tokens.iter().cloned().collect::<Vec<_>>();

        tokens.sort();

        for name in &tokens {
            writeln!(formatter, "    {}", name)?;
        }

        let mut nodes = self.nodes.iter().cloned().collect::<Vec<_>>();

        nodes.sort();

        for name in &nodes {
            writeln!(formatter, "    {}", name)?;
        }

        Ok(())
    }
}

impl From<TokenLit> for Leftmost {
    #[inline(always)]
    fn from(lit: TokenLit) -> Self {
        Self {
            optional: false,
            matches: None,
            tokens: Set::new([lit]),
            nodes: Set::empty(),
        }
    }
}

impl From<Ident> for Leftmost {
    #[inline(always)]
    fn from(rule: Ident) -> Self {
        Self {
            optional: false,
            matches: None,
            tokens: Set::empty(),
            nodes: Set::new([rule]),
        }
    }
}

impl<'a, R: AsRef<Regex>> From<&'a R> for Leftmost {
    fn from(regex: &'a R) -> Self {
        let regex = regex.as_ref();

        match regex {
            Regex::Operand(Operand::Unresolved(..)) => system_panic!("Unresolved operand."),

            Regex::Operand(Operand::Exclusion(..)) => system_panic!("Unresolved exclusion."),

            Regex::Operand(Operand::Dump(_, inner)) => Leftmost::from(inner),

            Regex::Operand(Operand::Token(_, lit)) => lit.clone().into(),

            Regex::Operand(Operand::Rule(_, rule)) => rule.clone().into(),

            Regex::Binary(left, op, right) => {
                let mut left = Leftmost::from(left);

                match op {
                    Operator::Union => {
                        let right = Leftmost::from(right);

                        left.optional = left.optional | right.optional;
                        left.append(right);

                        left
                    }

                    Operator::Concat => {
                        if left.optional {
                            let right = Leftmost::from(right);

                            left.optional = right.optional;
                            left.append(right);
                        }

                        left
                    }

                    _ => system_panic!("Unsupported Binary operator."),
                }
            }

            Regex::Unary(op, inner) => {
                let mut leftmost = Leftmost::from(inner);

                match op {
                    Operator::ZeroOrMore(sep) => match leftmost.optional {
                        true => {
                            if let Some(sep) = sep {
                                leftmost.append(Leftmost::from(sep));
                            }
                        }

                        false => leftmost.optional = true,
                    },

                    Operator::OneOrMore(sep) => {
                        if leftmost.optional {
                            if let Some(sep) = sep {
                                let sep = Leftmost::from(sep);

                                leftmost.optional = sep.optional;
                                leftmost.append(sep);
                            }
                        }
                    }

                    Operator::Optional => leftmost.optional = true,

                    _ => system_panic!("Unsupported Unary operator."),
                }

                leftmost
            }
        }
    }
}

impl Leftmost {
    #[inline(always)]
    pub(super) fn matches(&self) -> Option<&Set<TokenLit>> {
        self.matches.as_ref()
    }

    #[inline(always)]
    pub(super) fn tokens(&self) -> &Set<TokenLit> {
        &self.tokens
    }

    #[inline(always)]
    pub(super) fn is_optional(&self) -> bool {
        self.optional
    }

    pub(super) fn is_self_recursive<'a>(
        &'a self,
        map: &'a VariantMap,
        trace: &mut Vec<&'a Ident>,
    ) -> bool {
        for node in &self.nodes {
            if trace.first() == Some(&node) {
                trace.push(node);
                return true;
            }

            if trace.contains(&node) {
                continue;
            }

            let variant = expect_some!(map.get(node), "Unresolved reference",);
            let rule = expect_some!(variant.rule.as_ref(), "Missing rule.",);
            let leftmost = expect_some!(rule.leftmost.as_ref(), "Missing leftmost.",);

            trace.push(node);

            if leftmost.is_self_recursive(map, trace) {
                return true;
            }

            let _ = trace.pop();
        }

        false
    }

    pub(super) fn resolve_matches(&mut self, map: &mut VariantMap) {
        if self.matches.is_some() {
            return;
        }

        let mut matches = self.tokens.clone();

        for ident in &self.nodes {
            let variant = expect_some!(map.get_mut(ident), "Unresolved reference.",);
            let rule = expect_some!(variant.rule.as_mut(), "Missing rule.",);
            let leftmost = expect_some!(rule.leftmost.as_ref(), "Unresolved leftmost recursion.",);

            if let Some(node_matches) = &leftmost.matches {
                matches.append(node_matches.clone());
                continue;
            }

            let mut leftmost = expect_some!(take(&mut rule.leftmost), "Missing leftmost.",);
            leftmost.resolve_matches(map);

            let node_matches =
                expect_some!(leftmost.matches.as_ref(), "Missing leftmost matches.",);
            matches.append(node_matches.clone());

            let variant = expect_some!(map.get_mut(ident), "Unresolved reference.",);
            let rule = expect_some!(variant.rule.as_mut(), "Missing rule.",);
            rule.leftmost = Some(leftmost);
        }

        self.matches = Some(matches);
    }

    fn append(&mut self, other: Self) {
        self.tokens.append(other.tokens);
        self.nodes.append(other.nodes);
    }
}
