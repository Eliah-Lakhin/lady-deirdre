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
use syn::Generics;

use crate::utils::{debug_panic, AutomataContext};
use crate::{
    token::{
        rule::{RuleIndex, RuleMeta},
        scope::Scope,
        terminal::Terminal,
        transition::Transition,
        Token, NULL,
    },
    utils::{Automata, Map, PredictableCollection, Set, SetImpl, State},
};

pub(super) struct Compiler {
    scope: Scope,
    product_map: Map<State, RuleIndex>,
    input: Set<(State, (char, char), State)>,
    names: Map<State, State>,
    pending: Vec<State>,
    registered: Set<State>,
    result: Vec<Transition>,
}

impl Compiler {
    pub(super) fn compile(
        token_name: Ident,
        generics: Generics,
        rules: Vec<RuleMeta>,
        mismatch: Ident,
        scope: Scope,
        automata: Automata<Scope>,
        products: Map<State, RuleIndex>,
    ) -> Token {
        let mut alphabet = scope.alphabet().clone().into_inner();
        alphabet.insert(NULL);

        let input = automata
            .transitions()
            .into_iter()
            .map(|(from, through, to)| {
                let incoming = match through {
                    Terminal::Null => debug_panic!("Automata with null transition."),
                    Terminal::Product(..) => debug_panic!("Unfiltered production terminal."),
                    Terminal::Character(character) => character,
                };

                alphabet
                    .iter()
                    .map(move |peek| (*from, (*incoming, *peek), *to))
            })
            .flatten()
            .collect();

        let mut compiler = Self {
            scope,
            product_map: Map::empty(),
            input,
            names: Map::empty(),
            pending: vec![*automata.start()],
            registered: Set::empty(),
            result: Vec::new(),
        };

        for (state, product) in products {
            compiler.insert_product(state, product);
        }

        compiler.scope.reset();

        while compiler.next() {}

        compiler.result.sort();

        Token {
            token_name,
            generics,
            rules,
            mismatch,
            transitions: compiler.result,
        }
    }

    fn next(&mut self) -> bool {
        let from = match self.pending.pop() {
            None => return false,
            Some(state) => state,
        };

        let _ = self.registered.insert(from);

        let outgoing = self.outgoing_view(&from).group(|transition| transition);

        let from = self.name_of(from);

        for (to, transitions) in outgoing {
            let product = self.product_map.get(&to).cloned();

            let to = match self.is_termination(&to) {
                true => None,
                false => {
                    if !self.registered.contains(&to) {
                        self.pending.push(to);
                    }

                    Some(self.name_of(to))
                }
            };

            let group_by_incoming = transitions.group(|lookahead| lookahead);

            let mut group_both =
                Vec::<(Set<char>, Set<char>)>::with_capacity(group_by_incoming.len());

            'outer: for (incoming, peek) in group_by_incoming {
                for (incoming_set, peek_set) in group_both.iter_mut() {
                    if peek_set == &peek {
                        let _ = incoming_set.insert(incoming);
                        continue 'outer;
                    }
                }

                group_both.push((Set::new([incoming]), peek));
            }

            if group_both.is_empty() {
                continue;
            }

            for (incoming, peek) in group_both {
                self.result
                    .push(Transition::new(from, incoming, peek, to, product));
            }
        }

        true
    }

    #[inline]
    fn name_of(&mut self, original: State) -> State {
        *self
            .names
            .entry(original)
            .or_insert_with(|| self.scope.gen_state())
    }

    fn insert_product(&mut self, state: State, product: RuleIndex) {
        let outgoing = self.outgoing_view(&state);

        let inner_characters = match self.inner_characters(&state, &outgoing) {
            None => {
                self.product_map
                    .entry(state)
                    .and_modify(|previous| {
                        if *previous > product {
                            *previous = product
                        }
                    })
                    .or_insert(product);

                return;
            }

            Some(symbols) => symbols,
        };

        let incoming = self.incoming_view(&state);

        let new_state = self.scope.gen_state();
        let _ = self.product_map.insert(new_state, product);

        for (_, (incoming, peek)) in outgoing {
            if !inner_characters.contains(&peek) {
                let _ = self.input.remove(&(state, (incoming, peek), state));
                let _ = self.input.insert((state, (incoming, peek), new_state));
            }
        }

        for (from, (incoming, peek)) in incoming {
            if !inner_characters.contains(&peek) {
                let _ = self.input.remove(&(from, (incoming, peek), state));
                let _ = self.input.insert((from, (incoming, peek), new_state));
            }
        }
    }

    #[inline]
    fn inner_characters(
        &self,
        from: &State,
        outgoing: &Set<(State, (char, char))>,
    ) -> Option<Set<char>> {
        if outgoing.is_empty() {
            return None;
        }

        let mut result = Set::with_capacity(outgoing.len());

        for (to, (incoming, _)) in outgoing {
            if to != from {
                return None;
            }

            let _ = result.insert(*incoming);
        }

        Some(result)
    }

    #[inline]
    fn outgoing_view(&self, state: &State) -> Set<(State, (char, char))> {
        self.input
            .iter()
            .filter_map(|(from, through, to)| {
                if from != state {
                    return None;
                }

                Some((*to, *through))
            })
            .collect()
    }

    #[inline]
    fn incoming_view(&self, state: &State) -> Set<(State, (char, char))> {
        self.input
            .iter()
            .filter_map(|(from, through, to)| {
                if to != state || from == to {
                    return None;
                }

                Some((*from, *through))
            })
            .collect()
    }

    #[inline]
    fn is_termination(&self, state: &State) -> bool {
        !self.input.iter().any(|(from, _, _)| from == state)
    }
}
