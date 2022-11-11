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
use syn::{Error, Result};

use crate::{
    node::{
        builder::{kind::VariantKind, Builder},
        regex::{operand::RegexOperand, Regex},
    },
    utils::{PredictableCollection, Set, SetImpl},
};

impl CheckReferences for Regex {
    fn check_references(&self, context: &VariantKind, builder: &Builder) -> Result<Set<Ident>> {
        use VariantKind::*;

        match self {
            Self::Operand(RegexOperand::Unresolved { .. }) => unreachable!("Unresolved operand."),

            Self::Operand(RegexOperand::Debug { inner, .. }) => {
                inner.check_references(context, builder)
            }

            Self::Operand(RegexOperand::Rule { name, capture }) => {
                let reference = match builder.get_variant(name) {
                    Some(variant) => variant,

                    None if capture.is_some() => {
                        return Err(Error::new(
                            name.span(),
                            format!(
                                "Unresolved reference \"{}\".\nTry to introduce an enum variant \
                                with this name.",
                                name,
                            ),
                        ));
                    }

                    _ => {
                        return Err(Error::new(
                            name.span(),
                            format!(
                                "Unresolved reference \"{}\".\nEither introduce an enum variant \
                                with this name, or an inline expression using #[define(...)] \
                                attribute on the enum type.",
                                name,
                            ),
                        ));
                    }
                };

                match (context, reference.kind()) {
                    (Unspecified(..), _) => unreachable!("Unspecified variant with rule."),

                    (Comment(..), _) => {
                        return Err(Error::new(
                            name.span(),
                            format!(
                                "Reference \"{}\" points to a rule from the comment context. \
                                Comments cannot refer other rules.",
                                name,
                            ),
                        ));
                    }

                    (_, Root(..)) => {
                        return Err(Error::new(
                            name.span(),
                            format!(
                                "Reference \"{}\" points to the root rule. Root rule cannot be \
                                referred.",
                                name,
                            ),
                        ));
                    }

                    (_, Comment(..)) => {
                        return Err(Error::new(
                            name.span(),
                            format!(
                                "Reference \"{}\" points to a comment rule. Comment rule cannot be \
                                referred.",
                                name,
                            ),
                        ));
                    }

                    (_, Unspecified(..)) => {
                        return Err(Error::new(
                            name.span(),
                            format!(
                                "Reference \"{}\" points to an enum variant without associated \
                                parsing rule.\nAssociate that variant with parsing rule using \
                                #[rule(..)] attribute.",
                                name,
                            ),
                        ));
                    }

                    _ => (),
                }

                Ok(Set::new([name.clone()]))
            }

            Self::Operand(RegexOperand::Token { .. }) => Ok(Set::empty()),

            Self::Unary { inner, .. } => inner.check_references(context, builder),

            Self::Binary { left, right, .. } => {
                let left = left.check_references(context, builder)?;
                let right = right.check_references(context, builder)?;

                Ok(left.merge(right))
            }
        }
    }
}

pub(in crate::node) trait CheckReferences {
    fn check_references(&self, context: &VariantKind, builder: &Builder) -> Result<Set<Ident>>;
}
