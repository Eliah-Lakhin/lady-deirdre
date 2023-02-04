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

use crate::utils::transitions::Closure;
use crate::utils::{
    transitions::Transitions, Automata, AutomataContext, Map, PredictableCollection, Set, State,
};

pub(super) struct Deterministic<'a, C: AutomataContext> {
    context: &'a mut C,
    original: &'a Automata<C>,
    alphabet: &'a Set<C::Terminal>,
    pending: Vec<(State, Closure)>,
    registered: Map<Closure, State>,
    transitions: Transitions<C::Terminal>,
}

impl<'a, C: AutomataContext> Deterministic<'a, C> {
    pub(super) fn build(
        context: &'a mut C,
        original: &'a Automata<C>,
        alphabet: &'a Set<C::Terminal>,
    ) -> Automata<C> {
        let mut start = Closure::default();

        start.of_null(&original.transitions, original.start);

        let mut pending = Vec::with_capacity(original.transitions.length());
        let registered = Map::with_capacity(original.transitions.length());

        pending.push((original.start, start));

        let mut deterministic = Self {
            context,
            original,
            alphabet,
            pending,
            registered,
            transitions: Transitions::default(),
        };

        while deterministic.pop() {}

        let finish = deterministic
            .registered
            .iter()
            .filter_map(|(closure, state)| {
                for original_state in closure {
                    if original.finish.contains(original_state) {
                        return Some(*state);
                    }
                }

                None
            })
            .collect::<Set<_>>();

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

        let _ = self.registered.insert(closure.clone(), from);

        for symbol in self.alphabet {
            let mut target = Closure::default();

            for state in closure.into_iter().cloned() {
                target.of(&self.original.transitions, state, symbol);
            }

            if target.is_empty() {
                continue;
            }

            let to = self.push(target);

            self.transitions.through(from, symbol.clone(), to);
        }

        true
    }

    fn push(&mut self, closure: Closure) -> State {
        if let Some(state) = self.registered.get(&closure) {
            return *state;
        }

        for (state, pending) in self.pending.iter() {
            if pending == &closure {
                return *state;
            }
        }

        match closure.state() {
            Some(state) => {
                self.pending.push((state, closure));

                state
            }

            None => {
                let state = self.context.gen_state();

                self.pending.push((state, closure));

                state
            }
        }
    }
}
