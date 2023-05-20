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
    node::regex::{operand::RegexOperand, Regex},
    utils::debug_panic,
};

impl IsSkipRegex for Regex {
    fn is_skip(&self) -> Result<()> {
        match self {
            Self::Operand(RegexOperand::Unresolved { .. }) => debug_panic!("Unresolved operand."),

            Self::Operand(RegexOperand::Exclusion { .. }) => debug_panic!("Unresolved exclusion."),

            Self::Operand(RegexOperand::Debug { inner, .. }) => inner.is_skip(),

            Self::Operand(RegexOperand::Token {
                capture: Some(target),
                ..
            }) => Err(Error::new(
                target.span(),
                "Capturing is not allowed in the skip expression.",
            )),

            Self::Operand(RegexOperand::Token { .. }) => Ok(()),

            Self::Operand(RegexOperand::Rule { name, .. }) => {
                return Err(Error::new(
                    name.span(),
                    "Rule reference is not allowed in the skip expression.",
                ));
            }

            Self::Unary { inner, .. } => inner.is_skip(),

            Self::Binary { left, right, .. } => {
                left.is_skip()?;
                right.is_skip()?;
                Ok(())
            }
        }
    }
}

pub(in crate::node) trait IsSkipRegex {
    fn is_skip(&self) -> Result<()>;
}
