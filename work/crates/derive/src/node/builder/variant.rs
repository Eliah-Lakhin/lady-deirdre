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
        builder::{
            constructor::Constructor,
            index::RuleIndex,
            kind::VariantKind,
            rule::Rule,
            Builder,
        },
        regex::{prefix::Leftmost, references::CheckReferences, Regex},
    },
    utils::{debug_panic, PredictableCollection, Set},
};

pub(in crate::node) struct NodeVariant {
    name: Ident,
    kind: VariantKind,
    index: Option<RuleIndex>,
    rule: Option<Rule>,
    parser: Option<(Ident, bool, Leftmost)>,
    synchronization: Option<Span>,
    constructor: Option<Constructor>,
    secondary: bool,
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
        let mut index = None;
        let mut synchronization = None;
        let mut constructor = None;
        let mut secondary = None;
        let mut leftmost = None;
        let mut parser = None;

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

                "index" => {
                    if index.is_some() {
                        return Err(Error::new(attribute.span(), "Duplicate Index attribute."));
                    }

                    index = Some(RuleIndex::try_from(attribute)?);
                }

                "secondary" => {
                    if secondary.is_some() {
                        return Err(Error::new(
                            attribute.span(),
                            "Duplicate Secondary marker attribute.",
                        ));
                    }

                    secondary = Some(attribute.span());
                }

                "leftmost" => {
                    if leftmost.is_some() {
                        return Err(Error::new(
                            attribute.span(),
                            "Duplicate Leftmost specification attribute.",
                        ));
                    }

                    leftmost = Some(Leftmost::try_from(attribute)?);
                }

                "parser" => {
                    if parser.is_some() {
                        return Err(Error::new(attribute.span(), "Duplicate Parser attribute."));
                    }

                    parser = Some(attribute.parse_args::<Ident>()?);
                }

                _ => (),
            }
        }

        let kind = match (kind, &rule) {
            (Unspecified(..), Some(rule)) => {
                if parser.is_some() {
                    return Err(Error::new(
                        rule.span(),
                        "Rule sentence conflicts with #[parser(...)] \
                        parser function specification.\nThe rule sentence creates \
                        parser function implicitly.",
                    ));
                }

                Sentence(rule.span())
            }

            (kind @ Root(..), Some(rule)) => {
                if parser.is_some() {
                    return Err(Error::new(
                        rule.span(),
                        "Rule sentence conflicts with #[parser(...)] \
                        parser function specification.\nThe rule sentence creates \
                        parser function implicitly.",
                    ));
                }

                kind
            }

            (Root(span), None) => {
                if parser.is_none() {
                    return Err(Error::new(
                        span,
                        "Root annotation is not applicable to non-parsable rules.\n\
                        Associate this variant with #[rule(...)] or #[parser(...)] attribute.",
                    ));
                }

                Root(span)
            }

            (kind @ Comment(..), Some(rule)) => {
                if parser.is_some() {
                    return Err(Error::new(
                        rule.span(),
                        "Rule sentence conflicts with #[parser(...)] \
                        parser function specification.\nThe rule sentence creates \
                        parser function implicitly.",
                    ));
                }

                kind
            }

            (Comment(span), None) => {
                if parser.is_none() {
                    return Err(Error::new(
                        span,
                        "Comment annotation is not applicable to non-parsable rules.\n\
                        Associate this variant with #[rule(...)] or #[parser(...)] attribute.",
                    ));
                }

                Comment(span)
            }

            (kind @ Unspecified(..), None) => match &parser {
                Some(name) => Sentence(name.span()),
                None => kind,
            },

            (Sentence(..), _) => debug_panic!("Variant kind set to Sentence."),
        };

        let parser = match (parser, leftmost) {
            (Some(name), None) => {
                return Err(Error::new(
                    name.span(),
                    "If you specify a Parser function, you must also \
                    specify rule's leftmost set using #[leftmost(...)] attribute.",
                ));
            }

            (None, Some(leftmost)) => {
                return Err(Error::new(
                    leftmost.span(),
                    "If you specify a leftmost set explicitly, you must also \
                    specify rule's Parser function that would override default parser.\n\
                    Use #[parser(<function>)] attribute to refer \
                    \"fn Self::function<'a>(session: &mut SyntaxSession<'a, Node = Self>) -> Self \"\
                    parser function.",
                ));
            }

            (Some(name), Some(leftmost)) => Some((name, false, leftmost)),

            (None, None) => None,
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

            (_, Some(span)) => {
                if parser.is_some() {
                    return Err(Error::new(
                        *span,
                        "Synchronization annotation is not applicable \
                        to the rules with explicit parser function.",
                    ));
                }
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

            (_, Some(constructor)) => {
                if parser.is_some() {
                    return Err(Error::new(
                        constructor.span(),
                        "Explicit constructor is not applicable \
                        to the rules with explicit parser function.\nExplicit \
                        parse functions suppose to construct Node variants manually.",
                    ));
                }

                Some(constructor)
            }

            (_, None) => match parser.is_some() {
                true => None,
                false => Some(Constructor::try_from(variant)?),
            },
        };

        let index = match (&kind, index) {
            (Unspecified(..), Some(index)) => {
                return Err(Error::new(
                    index.span(),
                    "Rule index override is not applicable to non-parsable rules.\n\
                    Associate this variant with rule type.",
                ));
            }

            (Unspecified(..), None) => None,

            (Root(..), Some(index)) => {
                if index.index > 0 {
                    return Err(Error::new(index.span(), "Root rule index must be zero."));
                }

                Some(index)
            }

            (Root(span), None) => Some(RuleIndex {
                span: *span,
                index: 0,
                explicit: false,
            }),

            (_, Some(index)) => {
                if index.index == 0 {
                    return Err(Error::new(
                        index.span(),
                        "Zero rule index is not applicable to non-root rules.",
                    ));
                }

                Some(index)
            }

            (_, None) => None,
        };

        let secondary = match (&kind, secondary) {
            (Unspecified(..), Some(span)) => {
                return Err(Error::new(
                    span,
                    "Secondary markers not applicable to non-parsable rules.\n\
                    Associate this variant with rule type.",
                ));
            }

            (Unspecified(..), None) => false,

            (Root(..), Some(span)) => {
                return Err(Error::new(
                    span,
                    "Secondary marker is not applicable to the Root rule.",
                ));
            }

            (Root(..), None) => false,

            (_, marker) => marker.is_some(),
        };

        Ok(Self {
            name,
            kind,
            index,
            rule,
            parser,
            synchronization,
            constructor,
            secondary,
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
    pub(in crate::node) fn index(&self) -> Option<&RuleIndex> {
        self.index.as_ref()
    }

    #[inline(always)]
    pub(in crate::node) fn set_index(&mut self, index: usize) -> bool {
        if self.index.is_some() {
            return false;
        }

        self.index = Some(RuleIndex {
            span: self.kind.span(),
            index,
            explicit: false,
        });

        true
    }

    #[inline(always)]
    pub(in crate::node) fn parser(&self) -> Option<&Ident> {
        self.parser.as_ref().map(|(name, _, _)| name)
    }

    #[inline(always)]
    pub(in crate::node) fn get_leftmost(&self) -> Option<&Leftmost> {
        if let Some((_, built, leftmost)) = &self.parser {
            return match built {
                true => Some(leftmost),
                false => None,
            };
        }

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
    pub(in crate::node) fn is_secondary(&self) -> bool {
        self.secondary
    }

    #[inline(always)]
    pub(in crate::node) fn inline(&mut self, builder: &Builder) -> Result<()> {
        if let Some((_, _, leftmost)) = &self.parser {
            return leftmost.check_inlines(builder);
        }

        match &mut self.rule {
            None => Ok(()),

            Some(rule) => rule.inline(builder),
        }
    }

    #[inline(always)]
    pub(in crate::node) fn check_references(&self, builder: &Builder) -> Result<Set<Ident>> {
        if let Some((_, _, leftmost)) = &self.parser {
            return leftmost.check_references(&self.kind, builder);
        }

        match &self.rule {
            None => Ok(Set::empty()),

            Some(rule) => rule.check_references(&self.kind, builder),
        }
    }

    #[inline(always)]
    pub(in crate::node) fn build_leftmost(&mut self, builder: &mut Builder) -> Result<()> {
        if let Some((_, built, leftmost)) = &mut self.parser {
            if *built {
                return Ok(());
            }

            leftmost.resolve(builder)?;

            *built = true;

            return Ok(());
        }

        let rule = match &mut self.rule {
            None => return Ok(()),

            Some(rule) => rule,
        };

        rule.build_leftmost(builder)
    }

    pub(in crate::node) fn inject_skip(&mut self, injection: &Regex) {
        if self.parser.is_some() {
            return;
        }

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
