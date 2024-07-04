////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::fmt::{Display, Formatter};

use proc_macro2::Span;
use syn::Result;

use crate::{
    token::{
        chars::Class,
        input::{ProductMap, Variants},
        variant::TokenRule,
    },
    utils::{
        error,
        expect_some,
        system_panic,
        Automata,
        AutomataContext,
        AutomataTerminal,
        Map,
        PredictableCollection,
        Set,
        State,
        Strategy,
    },
};

pub(super) type TokenAutomata = Automata<Scope>;

impl AutomataImpl for TokenAutomata {
    fn merge(&mut self, scope: &mut Scope, variants: &Variants) -> Result<()> {
        loop {
            let mut has_changes = false;

            self.try_map(|_, transitions| {
                let mut products = Map::empty();
                let mut conflict = None;

                transitions.retain(|(through, to)| {
                    let index = match through {
                        Terminal::Product(index) => *index,

                        _ => return true,
                    };

                    let variant = expect_some!(variants.get(index as usize), "Missing variant.",);
                    let priority = variant.priority;

                    if let Some((previous, _)) = products.insert(priority, (index, *to)) {
                        let previous =
                            expect_some!(variants.get(previous as usize), "Missing variant.",);

                        conflict = Some((variant.ident.clone(), previous.ident.clone()));
                    }

                    false
                });

                if let Some((a, b)) = conflict {
                    return Err(error!(
                        a.span(),
                        "Rules {a} and {b} conflict. Both rules can match \
                        the same substring.\nTo resolve this ambiguity try \
                        to set distinct priorities to these variants using \
                        #[priority(<number>)] attribute.\nDefault priority \
                        is 0. Rules with higher priority have precedence \
                        over the rules with lower priority value.",
                    ));
                }

                if products.len() > 1 {
                    has_changes = true;
                }

                let product = products.iter().max_by_key(|(priority, _)| *priority);

                if let Some((_, (index, to))) = product {
                    let _ = transitions.insert((Terminal::Product(*index), *to));
                }

                Ok(())
            })?;

            if !has_changes {
                break;
            }

            scope.optimize(self);
        }

        Ok(())
    }

    fn filter_out(&mut self, variants: &Variants) -> Result<ProductMap> {
        let mut products = ProductMap::with_capacity(variants.len());
        let mut matched = Set::with_capacity(variants.len());

        self.retain(|from, through, _| match through {
            Terminal::Product(index) => {
                let index = *index as usize;

                if products.insert(*from, index).is_some() {
                    system_panic!("Unresolved ambiguity.",);
                }

                let _ = matched.insert(index);

                false
            }

            _ => true,
        });

        for (index, variant) in variants.iter().enumerate() {
            if variant.rule.is_none() {
                continue;
            }

            if !matched.contains(&index) {
                let ident = &variant.ident;

                return Err(error!(
                    ident.span(),
                    "Parsable rule {ident} is overlapping by other \
                    parsable rules due to a low priority. This rule never \
                    matches.\nTry to increase rule's priority using \
                    #[priority(<number>)] attribute.\nDefault priority is \
                    0. Rules with higher priority value have precedence \
                    over the rules with lower priority value.",
                ));
            }
        }

        Ok(products)
    }

    fn check_property_conflicts(&self, span: Span) -> Result<()> {
        for (_, outgoing) in self.transitions().view() {
            let mut other = false;
            let mut props = Vec::new();

            for (through, to) in outgoing {
                let Terminal::Class(class) = through else {
                    continue;
                };

                match class {
                    Class::Char(_) => continue,
                    Class::Props(through) => {
                        props.push((through, to));
                    }
                    Class::Other => {
                        other = true;
                    }
                }

                if other {
                    if let Some((through, _)) = props.first() {
                        return Err(error!(
                            span,
                            "Char properties choice ambiguity.\n\
                            Choice branching in form of \"{through} | .\" or \
                            \"{through} | ^[...]\" is forbidden.",
                        ));
                    }
                }

                if props.len() > 1 {
                    let (first_props, first_state) = &props[0];
                    let (second_props, second_state) = &props[1];

                    return match first_state == second_state {
                        true => {
                            let union = first_props.union(**second_props);

                            Err(error!(
                                span,
                                "Char properties choice ambiguity.\n\
                                Choice branching in form of \"{first_props} | \
                                {second_props}\" is forbidden.\n\
                                Consider introducing union property class \
                                instead: {union}.",
                            ))
                        }

                        false => Err(error!(
                            span,
                            "Char properties choice ambiguity.\n\
                            Choice branching between two distinct property \
                            classes ({first_props} and {second_props}) \
                            is forbidden.",
                        )),
                    };
                }
            }
        }

        Ok(())
    }
}

pub(super) trait AutomataImpl {
    fn merge(&mut self, scope: &mut Scope, variants: &Variants) -> Result<()>;

    fn filter_out(&mut self, variants: &Variants) -> Result<ProductMap>;

    fn check_property_conflicts(&self, span: Span) -> Result<()>;
}

pub(super) struct Scope {
    state: State,
    strategy: Strategy,
}

impl AutomataContext for Scope {
    type Terminal = Terminal;

    #[inline(always)]
    fn gen_state(&mut self) -> State {
        let state = self.state;

        self.state += 1;

        state
    }

    #[inline(always)]
    fn strategy(&self) -> Strategy {
        self.strategy
    }
}

impl Scope {
    #[inline(always)]
    pub(super) fn new() -> Self {
        Self {
            state: 1,
            strategy: Strategy::CANONICALIZE,
        }
    }

    #[inline(always)]
    pub(super) fn reset(&mut self) {
        self.state = 1;
    }

    #[inline(always)]
    pub(super) fn set_strategy(&mut self, strategy: Strategy) {
        self.strategy = strategy;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(super) enum Terminal {
    Null,
    Class(Class),
    Product(TokenRule),
}

impl AutomataTerminal for Terminal {
    #[inline(always)]
    fn null() -> Self {
        Self::Null
    }

    #[inline(always)]
    fn is_null(&self) -> bool {
        match self {
            Self::Null => true,
            _ => false,
        }
    }
}

impl Display for Terminal {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => formatter.write_str("null"),

            Self::Class(class) => Display::fmt(class, formatter),

            Self::Product(ident) => formatter.write_fmt(format_args!("Token({ident})")),
        }
    }
}
