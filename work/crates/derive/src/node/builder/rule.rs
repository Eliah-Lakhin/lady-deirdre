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
use syn::{spanned::Spanned, Attribute, Error, Result};

use crate::{
    node::{
        automata::{
            merge::AutomataMergeCaptures,
            synchronization::{AutomataSynchronization, Synchronization},
            variables::{AutomataVariables, VariableMap},
            NodeAutomata,
        },
        builder::{kind::VariantKind, Builder},
        regex::{
            encode::Encode,
            inject::Inject,
            inline::Inline,
            prefix::{Leftmost, RegexPrefix},
            references::CheckReferences,
            Regex,
        },
    },
    utils::{AutomataContext, OptimizationStrategy, Set},
};

pub(in crate::node) struct Rule {
    span: Span,
    regex: Regex,
    leftmost: Option<Leftmost>,
    synchronization: Option<Synchronization>,
    automata: Option<NodeAutomata>,
    variables: Option<VariableMap>,
}

impl Spanned for Rule {
    #[inline(always)]
    fn span(&self) -> Span {
        self.span
    }
}

impl<'a> TryFrom<&'a Attribute> for Rule {
    type Error = Error;

    fn try_from(attribute: &'a Attribute) -> Result<Self> {
        let span = attribute.span();
        let regex = attribute.parse_args::<Regex>()?;

        Ok(Self {
            span,
            regex,
            leftmost: None,
            automata: None,
            synchronization: None,
            variables: None,
        })
    }
}

impl From<Regex> for Rule {
    #[inline(always)]
    fn from(regex: Regex) -> Self {
        let span = regex.span();

        Self {
            span,
            regex,
            leftmost: None,
            automata: None,
            synchronization: None,
            variables: None,
        }
    }
}

impl Rule {
    #[inline(always)]
    pub(in crate::node) fn get_leftmost(&self) -> Option<&Leftmost> {
        self.leftmost.as_ref()
    }

    #[inline(always)]
    pub(in crate::node) fn variables(&self) -> &VariableMap {
        self.variables
            .as_ref()
            .expect("Internal error. Missing rule Variable Map.")
    }

    #[inline(always)]
    pub(in crate::node) fn automata(&self) -> &NodeAutomata {
        self.automata
            .as_ref()
            .expect("Internal error. Missing rule Automata.")
    }

    #[inline(always)]
    pub(in crate::node) fn synchronization(&self) -> &Synchronization {
        self.synchronization
            .as_ref()
            .expect("Internal error. Missing rule Synchronization.")
    }

    #[inline(always)]
    pub(in crate::node) fn inline(&mut self, builder: &Builder) -> Result<()> {
        assert!(
            self.leftmost.is_none(),
            "Internal error. Rule leftmost already built."
        );

        self.regex.inline(builder)
    }

    #[inline(always)]
    pub(in crate::node) fn check_references(
        &self,
        context: &VariantKind,
        builder: &Builder,
    ) -> Result<Set<Ident>> {
        self.regex.check_references(context, builder)
    }

    pub(in crate::node) fn build_leftmost(&mut self, builder: &mut Builder) -> Result<()> {
        if self.leftmost.is_some() {
            return Ok(());
        }

        let mut leftmost = self.regex.leftmost();

        leftmost.resolve(builder)?;

        self.leftmost = Some(leftmost);

        Ok(())
    }

    #[inline(always)]
    pub(in crate::node) fn surround(&mut self, injection: &Regex) {
        assert!(
            self.automata.is_none(),
            "Internal error. Rule automata already built.",
        );

        self.regex.surround(injection);
    }

    #[inline(always)]
    pub(in crate::node) fn inject(&mut self, injection: &Regex) {
        assert!(
            self.automata.is_none(),
            "Internal error. Rule automata already built.",
        );

        self.regex.inject(injection);
    }

    pub(in crate::node) fn build_automata(
        &mut self,
        builder: &mut Builder,
        variant_name: &Ident,
        synchronization_span: &Option<Span>,
    ) -> Result<()> {
        assert!(
            self.automata.is_none(),
            "Internal error. Rule automata already built.",
        );

        builder
            .scope()
            .set_strategy(OptimizationStrategy::DETERMINE);
        let mut automata = self.regex.encode(builder.scope())?;
        builder
            .scope()
            .set_strategy(OptimizationStrategy::CANONICALIZE);
        builder.scope().optimize(&mut automata);

        automata.merge_captures(builder.scope())?;

        self.variables = Some(automata.variable_map()?);

        self.synchronization = Some(automata.synchronization(
            variant_name.clone(),
            synchronization_span.clone().unwrap_or(self.span),
        ));

        self.automata = Some(automata);

        Ok(())
    }
}
