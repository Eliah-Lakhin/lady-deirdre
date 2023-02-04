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

use proc_macro2::Ident;

use crate::utils::debug_panic;
use crate::{
    node::regex::{operand::RegexOperand, operator::RegexOperator, Regex},
    utils::{PredictableCollection, Set, SetImpl},
};

#[derive(Clone, Default)]
pub(in crate::node) struct Leftmost {
    tokens: Set<Ident>,
    nodes: Set<Ident>,
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
    fn new_token(token: Ident) -> Self {
        Self {
            tokens: Set::new([token]),
            nodes: Set::empty(),
        }
    }

    #[inline(always)]
    fn new_node(node: Ident) -> Self {
        Self {
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

            Self::Unary { inner, .. } => inner.leftmost(),

            Self::Binary {
                operator,
                left,
                right,
            } => {
                let mut left = left.leftmost();

                match operator {
                    RegexOperator::Union => {
                        left.append(right.leftmost());

                        left
                    }

                    RegexOperator::Concat => left,

                    _ => debug_panic!("Unsupported Binary operator."),
                }
            }
        }
    }
}

pub(in crate::node) trait RegexPrefix {
    fn leftmost(&self) -> Leftmost;
}
