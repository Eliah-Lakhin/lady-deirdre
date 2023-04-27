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

use proc_macro2::{Ident, Span};
use syn::{spanned::Spanned, AttrStyle, Error, Result, Variant};

use crate::{
    node::{
        automata::{synchronization::Synchronization, variables::VariableMap, NodeAutomata},
        builder::{constructor::Constructor, kind::VariantKind, rule::Rule, Builder},
        regex::{prefix::Leftmost, Regex},
    },
    utils::{debug_panic, PredictableCollection, Set},
};

pub(in crate::node) struct NodeVariant {
    name: Ident,
    kind: VariantKind,
    rule: Option<Rule>,
    synchronization: Option<Span>,
    constructor: Option<Constructor>,
}

impl Spanned for NodeVariant {
    #[inline(always)]
    fn span(&self) -> Span {
        self.name.span()
    }
}

impl<'a> TryFrom<&'a Variant> for NodeVariant {
    type Error = Error;

    fn try_from(variant: &'a Variant) -> Result<Self> {
        use VariantKind::*;

        let name = variant.ident.clone();

        let mut kind = Unspecified(variant.span());
        let mut rule = None;
        let mut synchronization = None;
        let mut constructor = None;

        for attribute in &variant.attrs {
            match attribute.style {
                AttrStyle::Inner(_) => continue,
                AttrStyle::Outer => (),
            }

            let name = match attribute.path.get_ident() {
                None => continue,
                Some(name) => name,
            };

            match name.to_string().as_str() {
                "root" => {
                    kind.is_vacant(attribute.span())?;
                    kind = Root(attribute.span());
                }

                "comment" => {
                    kind.is_vacant(attribute.span())?;
                    kind = Comment(attribute.span());
                }

                "rule" => {
                    if rule.is_some() {
                        return Err(Error::new(attribute.span(), "Duplicate Rule attribute."));
                    }

                    rule = Some(Rule::try_from(attribute)?);
                }

                "synchronization" => {
                    if synchronization.is_some() {
                        return Err(Error::new(
                            attribute.span(),
                            "Duplicate Synchronization attribute.",
                        ));
                    }

                    synchronization = Some(attribute.span());
                }

                "constructor" => {
                    if constructor.is_some() {
                        return Err(Error::new(
                            attribute.span(),
                            "Duplicate Constructor attribute.",
                        ));
                    }

                    constructor = Some(Constructor::try_from(attribute)?);
                }

                _ => (),
            }
        }

        let kind = match (kind, &rule) {
            (Unspecified(..), Some(rule)) => Sentence(rule.span()),

            (kind @ Root(..), Some(..)) => kind,

            (Root(span), None) => {
                return Err(Error::new(
                    span,
                    "Root annotation is not applicable to non-parsable rules.\n\
                    Associate this variant with #[rule(...)] attribute.",
                ));
            }

            (kind @ Comment(..), Some(..)) => kind,

            (Comment(span), None) => {
                return Err(Error::new(
                    span,
                    "Comment annotation is not applicable to non-parsable rules.\n\
                    Associate this variant with #[rule(...)] attribute.",
                ));
            }

            (kind @ Unspecified(..), None) => kind,

            (Sentence(..), _) => debug_panic!("Variant kind set to Sentence."),
        };

        match (&kind, &synchronization) {
            (Unspecified(..), Some(span)) => {
                return Err(Error::new(
                    *span,
                    "Synchronization annotation is not applicable to non-parsable rules.\n\
                    Associate this variant with #[rule(...)] attribute.",
                ));
            }

            (Root(..), Some(span)) => {
                return Err(Error::new(
                    *span,
                    "Synchronization annotation is not applicable to the Root rule.",
                ));
            }

            (Comment(..), Some(span)) => {
                return Err(Error::new(
                    *span,
                    "Synchronization annotation is not applicable to the Comment rule.",
                ));
            }

            _ => (),
        }

        let constructor = match (&kind, constructor) {
            (Unspecified(..), Some(constructor)) => {
                return Err(Error::new(
                    constructor.span(),
                    "Explicit constructor is not applicable to non-parsable rules.\n\
                    Associate this variant with rule type.",
                ));
            }

            (Unspecified(..), None) => None,

            (_, Some(constructor)) => Some(constructor),

            (_, None) => Some(Constructor::try_from(variant)?),
        };

        Ok(Self {
            name,
            kind,
            rule,
            synchronization,
            constructor,
        })
    }
}

impl NodeVariant {
    #[inline(always)]
    pub(in crate::node) fn name(&self) -> &Ident {
        &self.name
    }

    #[inline(always)]
    pub(in crate::node) fn kind(&self) -> &VariantKind {
        &self.kind
    }

    #[inline(always)]
    pub(in crate::node) fn get_leftmost(&self) -> Option<&Leftmost> {
        self.rule
            .as_ref()
            .expect("Internal error. Missing variant rule.")
            .get_leftmost()
    }

    #[inline(always)]
    pub(in crate::node) fn leftmost(&self) -> &Leftmost {
        self.get_leftmost()
            .expect("Internal error. Missing variant rule's leftmost.")
    }

    #[inline(always)]
    pub(in crate::node) fn variables(&self) -> &VariableMap {
        self.rule
            .as_ref()
            .expect("Internal error. Missing variant rule.")
            .variables()
    }

    #[inline(always)]
    pub(in crate::node) fn is_global_synchronization(&self) -> bool {
        self.synchronization.is_some()
    }

    #[inline(always)]
    pub(in crate::node) fn synchronization(&self) -> &Synchronization {
        self.rule
            .as_ref()
            .expect("Internal error. Missing variant rule.")
            .synchronization()
    }

    #[inline(always)]
    pub(in crate::node) fn constructor(&self) -> &Constructor {
        self.constructor
            .as_ref()
            .expect("Internal error. Missing variant constructor.")
    }

    #[inline(always)]
    pub(in crate::node) fn automata(&self) -> &NodeAutomata {
        self.rule
            .as_ref()
            .expect("Internal error. Missing variant rule.")
            .automata()
    }

    #[inline(always)]
    pub(in crate::node) fn inline(&mut self, builder: &Builder) -> Result<()> {
        match &mut self.rule {
            None => Ok(()),

            Some(rule) => rule.inline(builder),
        }
    }

    #[inline(always)]
    pub(in crate::node) fn check_references(&self, builder: &Builder) -> Result<Set<Ident>> {
        match &self.rule {
            None => Ok(Set::empty()),

            Some(rule) => rule.check_references(&self.kind, builder),
        }
    }

    #[inline(always)]
    pub(in crate::node) fn build_leftmost(&mut self, builder: &mut Builder) -> Result<()> {
        let rule = match &mut self.rule {
            None => return Ok(()),

            Some(rule) => rule,
        };

        rule.build_leftmost(builder)
    }

    pub(in crate::node) fn inject_skip(&mut self, injection: &Regex) {
        match self.kind {
            VariantKind::Unspecified(..) | VariantKind::Comment(..) => (),

            VariantKind::Root(..) => {
                self.rule
                    .as_mut()
                    .expect("Internal error. Missing Root rule.")
                    .surround(injection);
            }

            VariantKind::Sentence(..) => {
                self.rule
                    .as_mut()
                    .expect("Internal error. Missing Sentence rule.")
                    .inject(injection);
            }
        }
    }

    #[inline(always)]
    pub(in crate::node) fn build_automata(&mut self, builder: &mut Builder) -> Result<()> {
        let rule = match &mut self.rule {
            None => return Ok(()),

            Some(rule) => rule,
        };

        let constructor = self
            .constructor
            .as_ref()
            .expect("Internal error. Missing Variant constructor.");

        rule.build_automata(builder, &self.name, &self.synchronization)?;

        match &self.kind {
            VariantKind::Root(..) => (),

            _ => {
                if rule.automata().accepts_null() {
                    return Err(Error::new(
                        rule.span(),
                        "Variant's rule expression can match empty token sequence.\n\
                        Non-root nodes of empty token sequences not allowed.",
                    ));
                }
            }
        }

        match &self.synchronization {
            None => (),

            Some(span) => {
                let synchronization = rule.synchronization();

                if synchronization.open().is_none() {
                    return Err(Error::new(
                        *span,
                        "Synchronization attribute is not applicable to this rule.\nRule's \
                        leftmost token set contains more than one token, or the leftmost \
                        set refers another rule.",
                    ));
                }

                if synchronization.close().is_none() {
                    return Err(Error::new(
                        *span,
                        "Synchronization attribute is not applicable to this rule.\nRule's \
                        rightmost token set contains more than one token, or the rightmost \
                        set refers another rule.",
                    ));
                }

                if synchronization.open() == synchronization.close() {
                    return Err(Error::new(
                        *span,
                        "Synchronization attribute is not applicable to this rule.\nRule's \
                        leftmost token is equal to the rule's rightmost token.",
                    ));
                }
            }
        }

        rule.variables().fits(constructor)?;

        Ok(())
    }
}
