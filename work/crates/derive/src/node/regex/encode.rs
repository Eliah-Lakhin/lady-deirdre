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

use syn::{Error, Result};

use crate::{
    node::{
        automata::{
            merge::AutomataMergeCaptures,
            scope::Scope,
            variables::AutomataVariables,
            NodeAutomata,
        },
        regex::{
            operand::RegexOperand,
            operator::RegexOperator,
            prefix::RegexPrefix,
            terminal::Terminal,
            Regex,
        },
    },
    utils::{debug_panic, AutomataContext, OptimizationStrategy, Set, SetImpl},
};

impl Encode for Regex {
    #[inline(always)]
    fn encode(&self, scope: &mut Scope) -> Result<NodeAutomata> {
        match self {
            Self::Operand(RegexOperand::Unresolved { .. }) => debug_panic!("Unresolved operand."),

            Self::Operand(RegexOperand::Debug { span, inner }) => {
                let leftmost = inner.leftmost();
                scope.set_strategy(OptimizationStrategy::CANONICALIZE);
                let mut inner = inner.encode(scope)?;

                inner.merge_captures(scope)?;

                let variables = inner.variable_map()?;

                return Err(Error::new(
                    *span,
                    format!(
                        "This expression is a subject for debugging.\n\nCapturing variables \
                        are:\n{:#}\nState machine transitions are:\n{:#}\nLeftmost set \
                        is:\n{:#}\n",
                        variables, inner, leftmost
                    ),
                ));
            }

            Self::Operand(RegexOperand::Token { name, capture }) => {
                Ok(scope.terminal(Set::new([Terminal::Token {
                    name: name.clone(),
                    capture: capture.clone(),
                }])))
            }

            Self::Operand(RegexOperand::Rule { name, capture }) => {
                Ok(scope.terminal(Set::new([Terminal::Node {
                    name: name.clone(),
                    capture: capture.clone(),
                }])))
            }

            Self::Unary { operator, inner } => {
                let inner = inner.encode(scope)?;

                match operator {
                    RegexOperator::OneOrMore { separator: None } => {
                        let zero_or_more = {
                            let inner = scope.copy(&inner);
                            scope.repeat(inner)
                        };

                        Ok(scope.concatenate(inner, zero_or_more))
                    }

                    RegexOperator::OneOrMore {
                        separator: Some(separator),
                    } => {
                        let separator = separator.encode(scope)?;

                        let rest = {
                            let inner = scope.copy(&inner);
                            scope.concatenate(separator, inner)
                        };
                        let repeat_rest = scope.repeat(rest);

                        Ok(scope.concatenate(inner, repeat_rest))
                    }

                    RegexOperator::ZeroOrMore { separator: None } => Ok(scope.repeat(inner)),

                    RegexOperator::ZeroOrMore {
                        separator: Some(separator),
                    } => {
                        let separator = separator.encode(scope)?;

                        let rest = {
                            let inner = scope.copy(&inner);
                            scope.concatenate(separator, inner)
                        };
                        let repeat_rest = scope.repeat(rest);
                        let one_or_more = scope.concatenate(inner, repeat_rest);

                        Ok(scope.optional(one_or_more))
                    }

                    RegexOperator::Optional => Ok(scope.optional(inner)),

                    _ => debug_panic!("Unsupported Unary operator."),
                }
            }

            Self::Binary {
                operator,
                left,
                right,
            } => {
                let left = left.encode(scope)?;
                let right = right.encode(scope)?;

                match operator {
                    RegexOperator::Union => Ok(scope.union(left, right)),
                    RegexOperator::Concat => Ok(scope.concatenate(left, right)),
                    _ => debug_panic!("Unsupported Binary operator."),
                }
            }
        }
    }
}

pub(in crate::node) trait Encode {
    fn encode(&self, scope: &mut Scope) -> Result<NodeAutomata>;
}
