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

use std::fmt::{Display, Formatter};

use syn::Result;

use crate::{
    token::{
        chars::Class,
        input::{ProductMap, Variants},
        variant::TokenIndex,
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
}

pub(super) trait AutomataImpl {
    fn merge(&mut self, scope: &mut Scope, variants: &Variants) -> Result<()>;

    fn filter_out(&mut self, variants: &Variants) -> Result<ProductMap>;
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
    Product(TokenIndex),
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
