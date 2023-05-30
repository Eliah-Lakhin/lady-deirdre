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
