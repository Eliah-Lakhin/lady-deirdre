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

use crate::utils::{
    transitions::{Transitions, TransitionsImpl},
    Automata,
    AutomataContext,
    AutomataTerminal,
    PredictableCollection,
    Set,
    SetImpl,
    State,
};

pub(super) struct Deterministic<'a, C: AutomataContext> {
    context: &'a mut C,
    original: &'a Automata<C>,
    alphabet: Set<C::Terminal>,
    pending: Vec<(C::State, Set<C::State>)>,
    registered: Vec<(C::State, Set<C::State>)>,
    transitions: Transitions<C::State, C::Terminal>,
}

impl<'a, C: AutomataContext> Deterministic<'a, C> {
    pub(super) fn build(context: &'a mut C, original: &'a Automata<C>) -> Automata<C> {
        let mut alphabet = original.transitions.alphabet();
        alphabet.remove(&C::Terminal::null());

        let start = original
            .transitions
            .closure_of(original.start, C::Terminal::null());

        let mut pending = Vec::with_capacity(original.transitions.len());
        let registered = Vec::with_capacity(original.transitions.len());

        pending.push((original.start, start));

        let mut deterministic = Self {
            context,
            original,
            alphabet,
            pending,
            registered,
            transitions: Transitions::empty(),
        };

        while deterministic.pop() {}

        let finish = deterministic
            .registered
            .iter()
            .filter_map(|(state, closure)| {
                if closure.intersection(&original.finish).next().is_some() {
                    Some(*state)
                } else {
                    None
                }
            })
            .collect();

        Automata {
            start: original.start,
            finish,
            transitions: deterministic.transitions,
        }
    }

    fn pop(&mut self) -> bool {
        let (from, closure) = match self.pending.pop() {
            None => return false,
            Some(pair) => pair,
        };

        self.registered.push((from, closure.clone()));

        for symbol in self.alphabet.clone() {
            let mut target = Set::empty();

            for state in closure.iter().cloned() {
                target.append(self.original.transitions.closure_of(state, symbol.clone()));
            }

            if target.is_empty() {
                continue;
            }

            let to = self.push(target);

            self.transitions.insert((from, symbol, to));
        }

        true
    }

    fn push(&mut self, closure: Set<C::State>) -> C::State {
        for (state, registered) in self.registered.iter() {
            if registered == &closure {
                return *state;
            }
        }

        for (state, pending) in self.pending.iter() {
            if pending == &closure {
                return *state;
            }
        }

        match closure.single() {
            None => {
                let state = C::State::gen_state(&mut self.context);

                self.pending.push((state, closure));

                state
            }

            Some(state) => {
                self.pending.push((state, closure));

                state
            }
        }
    }
}
