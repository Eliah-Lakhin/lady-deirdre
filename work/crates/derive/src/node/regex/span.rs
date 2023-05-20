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

use proc_macro2::Span;

use crate::node::regex::{operand::RegexOperand, operator::RegexOperator, Regex};

impl SetSpan for Regex {
    fn set_span(&mut self, span: Span) {
        match self {
            Self::Operand(RegexOperand::Unresolved {
                name,
                capture: None,
            }) => {
                name.set_span(span);
            }

            Self::Operand(RegexOperand::Unresolved {
                name,
                capture: Some(capture),
            }) => {
                name.set_span(span);
                capture.set_span(span);
            }

            Self::Operand(RegexOperand::Debug { inner, .. }) => {
                inner.set_span(span);
            }

            Self::Operand(RegexOperand::Token {
                name,
                capture: Some(capture),
                ..
            }) => {
                name.set_span(span);
                capture.set_span(span);
            }

            Self::Operand(RegexOperand::Token {
                name,
                capture: None,
                ..
            }) => {
                name.set_span(span);
            }

            Self::Operand(RegexOperand::Rule {
                name,
                capture: Some(capture),
            }) => {
                name.set_span(span);
                capture.set_span(span);
            }

            Self::Operand(RegexOperand::Rule {
                name,
                capture: None,
            }) => {
                name.set_span(span);
            }

            Self::Operand(RegexOperand::Exclusion {
                set,
                capture: Some(capture),
                ..
            }) => {
                set.set_span(span);
                capture.set_span(span);
            }

            Self::Operand(RegexOperand::Exclusion {
                set, capture: None, ..
            }) => {
                set.set_span(span);
            }

            Self::Unary { inner, operator } => {
                inner.set_span(span);

                match operator {
                    RegexOperator::OneOrMore {
                        separator: Some(separator),
                    } => separator.set_span(span),

                    RegexOperator::ZeroOrMore {
                        separator: Some(separator),
                    } => separator.set_span(span),

                    _ => (),
                }
            }

            Self::Binary { left, right, .. } => {
                left.set_span(span);
                right.set_span(span);
            }
        }
    }
}

pub(in crate::node) trait SetSpan {
    fn set_span(&mut self, span: Span);
}
