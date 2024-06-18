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

use std::collections::BTreeMap;

use crate::utils::{
    transitions::{Closure, ClosureCache, Transitions},
    Automata,
    AutomataContext,
    AutomataTerminal,
    Map,
    PredictableCollection,
    Set,
    State,
};

const CACHE_THRESHOLD: usize = 10000;
const PENDING_THRESHOLD: usize = 1000;

pub(super) struct Deterministic<'a, C: AutomataContext> {
    context: &'a mut C,
    original: &'a Transitions<C::Terminal>,
    alphabet: &'a Set<C::Terminal>,
    pending: Pending,
    registered: Map<Closure, State>,
    transitions: Transitions<C::Terminal>,
    cache: Option<(usize, ClosureCache<'a, C::Terminal>)>,
}

impl<'a, C: AutomataContext> Deterministic<'a, C> {
    pub(super) fn build(
        context: &'a mut C,
        alphabet: &'a Set<C::Terminal>,
        start: &'a Set<State>,
        finish: &'a Set<State>,
        transitions: &'a Transitions<C::Terminal>,
    ) -> Automata<C> {
        let mut start_closure = Closure::default();

        for start in start {
            start_closure.of(transitions, *start, &C::Terminal::null(), &mut None);
        }

        let capacity = transitions.len();

        let pending = match capacity >= PENDING_THRESHOLD {
            true => Pending::Map(BTreeMap::new()),
            false => Pending::Vec(Vec::with_capacity(capacity)),
        };

        let registered = Map::with_capacity(capacity);

        let cache = match capacity >= CACHE_THRESHOLD {
            true => Some((alphabet.capacity(), Map::with_capacity(capacity))),
            false => None,
        };

        let mut deterministic = Self {
            context,
            original: transitions,
            alphabet,
            pending,
            registered,
            transitions: Transitions::default(),
            cache,
        };

        let start = deterministic.force_push(start_closure);

        while deterministic.pop() {}

        let finish = deterministic
            .registered
            .iter()
            .filter_map(|(closure, state)| {
                for original_state in closure {
                    if finish.contains(original_state) {
                        return Some(*state);
                    }
                }

                None
            })
            .collect::<Set<_>>();

        Automata {
            start,
            finish,
            transitions: deterministic.transitions,
        }
    }

    fn pop(&mut self) -> bool {
        let (closure, from) = match self.pending.pop() {
            None => return false,
            Some(pair) => pair,
        };

        let _ = self.registered.insert(closure.clone(), from);

        for symbol in self.alphabet {
            let mut target = Closure::default();

            for state in closure.into_iter().cloned() {
                target.of(&self.original, state, symbol, &mut self.cache);
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

        if let Some(state) = self.pending.get(&closure) {
            return state;
        }

        self.force_push(closure)
    }

    #[inline]
    fn force_push(&mut self, closure: Closure) -> State {
        match closure.state() {
            Some(state) => {
                self.pending.push(closure, state);

                state
            }

            None => {
                let state = self.context.gen_state();

                self.pending.push(closure, state);

                state
            }
        }
    }
}

enum Pending {
    Vec(Vec<(Closure, State)>),
    Map(BTreeMap<Closure, State>),
}

impl Pending {
    #[inline(always)]
    fn push(&mut self, closure: Closure, state: State) {
        match self {
            Self::Vec(pending) => pending.push((closure, state)),
            Self::Map(pending) => {
                let _ = pending.insert(closure, state);
            }
        }
    }

    #[inline(always)]
    fn pop(&mut self) -> Option<(Closure, State)> {
        match self {
            Self::Vec(pending) => pending.pop(),
            Self::Map(pending) => pending.pop_last(),
        }
    }

    #[inline(always)]
    fn get(&self, closure: &Closure) -> Option<State> {
        match self {
            Self::Vec(pending) => {
                for (pending, state) in pending {
                    if pending == closure {
                        return Some(*state);
                    }
                }

                None
            }

            Self::Map(pending) => pending.get(closure).copied(),
        }
    }
}
