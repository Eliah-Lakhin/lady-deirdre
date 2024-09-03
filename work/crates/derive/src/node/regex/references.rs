////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::{
    node::{
        builder::{kind::VariantKind, Builder},
        regex::{operand::RegexOperand, operator::RegexOperator, Regex},
    },
    utils::{debug_panic, PredictableCollection, Set, SetImpl},
};

impl CheckReferences for Regex {
    fn check_references(&self, context: &VariantKind, builder: &Builder) -> Result<Set<Ident>> {
        use VariantKind::*;

        match self {
            Self::Operand(RegexOperand::Unresolved { .. }) => debug_panic!("Unresolved operand."),

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
                    (Unspecified(..), _) => debug_panic!("Unspecified variant with rule."),

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

            Self::Unary { operator, inner } => {
                let inner = inner.check_references(context, builder)?;

                match operator {
                    RegexOperator::ZeroOrMore {
                        separator: Some(separator),
                    } => Ok(separator.check_references(context, builder)?.merge(inner)),

                    RegexOperator::OneOrMore {
                        separator: Some(separator),
                    } => Ok(separator.check_references(context, builder)?.merge(inner)),

                    _ => Ok(inner),
                }
            }

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
