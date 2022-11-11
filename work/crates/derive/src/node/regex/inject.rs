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

use syn::spanned::Spanned;

use crate::node::regex::{operand::RegexOperand, operator::RegexOperator, span::SetSpan, Regex};

impl Inject for Regex {
    fn surround(&mut self, injection: &Self) {
        match self {
            Self::Operand(RegexOperand::Debug { inner, .. }) => {
                inner.surround(injection);
            }

            operand @ Self::Operand { .. } => {
                let mut injection = injection.clone();

                if let Self::Operand(operand) = operand {
                    injection.set_span(operand.span());
                }

                *operand = Self::Binary {
                    operator: RegexOperator::Concat,
                    left: Box::new(injection.clone()),
                    right: Box::new(Self::Binary {
                        operator: RegexOperator::Concat,
                        left: Box::new(operand.clone()),
                        right: Box::new(injection),
                    }),
                };
            }

            Self::Unary { operator, inner } => {
                match operator {
                    RegexOperator::ZeroOrMore {
                        separator: Some(separator),
                    } => {
                        separator.surround(injection);
                    }

                    RegexOperator::OneOrMore {
                        separator: Some(separator),
                    } => {
                        separator.surround(injection);
                    }

                    _ => (),
                }

                inner.surround(injection);
            }

            Self::Binary { left, right, .. } => {
                left.surround(injection);
                right.surround(injection);
            }
        }
    }

    fn inject(&mut self, injection: &Self) {
        match self {
            Self::Operand(RegexOperand::Debug { inner, .. }) => {
                inner.inject(injection);
            }

            Self::Operand { .. } => (),

            Self::Unary { operator, inner } => {
                match operator {
                    RegexOperator::ZeroOrMore {
                        separator: Some(separator),
                    } => {
                        separator.surround(injection);
                    }

                    RegexOperator::ZeroOrMore { separator } => {
                        let mut injection = injection.clone();

                        injection.set_span(inner.span());

                        *separator = Some(Box::new(injection));
                    }

                    RegexOperator::OneOrMore {
                        separator: Some(separator),
                    } => {
                        separator.surround(injection);
                    }

                    RegexOperator::OneOrMore { separator } => {
                        let mut injection = injection.clone();

                        injection.set_span(inner.span());

                        *separator = Some(Box::new(injection));
                    }

                    _ => (),
                }

                inner.inject(injection);
            }

            Self::Binary {
                operator,
                left,
                right,
            } => {
                left.inject(injection);
                right.inject(injection);

                match operator {
                    RegexOperator::Concat => {
                        let mut injection = injection.clone();

                        injection.set_span(left.span());

                        *self = Self::Binary {
                            operator: RegexOperator::Concat,
                            left: left.clone(),
                            right: Box::new(Self::Binary {
                                operator: RegexOperator::Concat,
                                left: Box::new(injection),
                                right: right.clone(),
                            }),
                        }
                    }

                    _ => (),
                }
            }
        }
    }
}

pub(in crate::node) trait Inject {
    fn surround(&mut self, injection: &Self);

    fn inject(&mut self, injection: &Self);
}
