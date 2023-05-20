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

use crate::{
    node::regex::{
        operand::{RegexOperand, TokenLit},
        operator::RegexOperator,
        Regex,
    },
    utils::{debug_panic, PredictableCollection, Set, SetImpl},
};

impl Alphabet for Regex {
    fn alphabet(&self) -> Set<TokenLit> {
        match self {
            Self::Operand(RegexOperand::Unresolved { .. }) => debug_panic!("Unresolved operand."),

            Self::Operand(RegexOperand::Debug { inner, .. }) => inner.alphabet(),

            Self::Operand(RegexOperand::Token { name, .. }) => Set::new([name.clone()]),

            Self::Operand(RegexOperand::Rule { .. }) => Set::empty(),

            Self::Operand(RegexOperand::Exclusion { set, .. }) => set.set.clone(),

            Self::Unary { operator, inner } => {
                let inner = inner.alphabet();

                match operator {
                    RegexOperator::OneOrMore {
                        separator: Some(separator),
                    } => return separator.alphabet().merge(inner),

                    RegexOperator::ZeroOrMore {
                        separator: Some(separator),
                    } => return separator.alphabet().merge(inner),

                    _ => inner,
                }
            }

            Self::Binary { left, right, .. } => left.alphabet().merge(right.alphabet()),
        }
    }

    fn resolve_exclusions(&mut self, alphabet: &Set<TokenLit>) {
        match self {
            Self::Operand(RegexOperand::Exclusion { set, capture }) => {
                let mut rest = alphabet.clone();

                let _ = rest.insert(TokenLit::Other(set.span));

                for excluded in &set.set {
                    let _ = rest.remove(&excluded);
                }

                let regex = rest.into_iter().fold(None, |result, mut token| {
                    token.set_span(set.span);

                    let right = Regex::Operand(RegexOperand::Token {
                        name: token,
                        capture: capture.clone(),
                    });

                    Some(match result {
                        None => right,
                        Some(left) => Regex::Binary {
                            operator: RegexOperator::Union,
                            left: Box::new(left),
                            right: Box::new(right),
                        },
                    })
                });

                match regex {
                    None => debug_panic!("Exclusion is void."),
                    Some(regex) => *self = regex,
                }
            }

            Self::Operand(RegexOperand::Debug { inner, .. }) => inner.resolve_exclusions(alphabet),

            Self::Operand(..) => (),

            Self::Binary { left, right, .. } => {
                left.resolve_exclusions(alphabet);
                right.resolve_exclusions(alphabet);
            }

            Self::Unary { operator, inner } => {
                inner.resolve_exclusions(alphabet);

                match operator {
                    RegexOperator::OneOrMore {
                        separator: Some(separator),
                    } => {
                        separator.resolve_exclusions(alphabet);
                    }

                    RegexOperator::ZeroOrMore {
                        separator: Some(separator),
                    } => {
                        separator.resolve_exclusions(alphabet);
                    }

                    _ => (),
                }
            }
        }
    }
}

pub(in crate::node) trait Alphabet {
    fn alphabet(&self) -> Set<TokenLit>;

    fn resolve_exclusions(&mut self, alphabet: &Set<TokenLit>);
}
