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

use std::{
    cmp::Ordering,
    collections::VecDeque,
    fmt::{Display, Formatter},
    mem::take,
    ops::RangeFrom,
};

use syn::Result;

use crate::utils::{
    deterministic::Deterministic,
    system_panic,
    transitions::Transitions,
    AutomataContext,
    Map,
    PredictableCollection,
    Set,
    SetImpl,
    State,
};

pub struct Automata<C: AutomataContext> {
    pub(super) start: State,
    pub(super) finish: Set<State>,
    pub(super) transitions: Transitions<C::Terminal>,
}

impl<C: AutomataContext> Display for Automata<C>
where
    C::Terminal: Display + Ord,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        struct Visitor<'a, 'f, C: AutomataContext> {
            original: &'a Automata<C>,
            formatter: &'a mut Formatter<'f>,
            pending: VecDeque<State>,
            visited: Set<State>,
            names: Map<State, usize>,
            generator: RangeFrom<usize>,
        }

        impl<'a, 'f, C> Visitor<'a, 'f, C>
        where
            C: AutomataContext,
            C::Terminal: Display + Ord,
        {
            fn next(&mut self) -> std::fmt::Result {
                if let Some(state) = self.pending.pop_front() {
                    let mut transitions = self
                        .original
                        .transitions
                        .into_iter()
                        .filter(|(from, _, _)| from == &state)
                        .collect::<Vec<_>>();

                    transitions.sort_by(|a, b| {
                        if a.2 < b.2 {
                            return Ordering::Less;
                        }

                        if a.2 > b.2 {
                            return Ordering::Greater;
                        }

                        if a.1 < b.1 {
                            return Ordering::Less;
                        }

                        if a.1 > b.1 {
                            return Ordering::Greater;
                        }

                        Ordering::Equal
                    });

                    let mut string_from = format!("{}", self.name_of(state));

                    if self.original.finish.contains(&state) {
                        string_from = format!("{}\u{2192}", string_from);
                    }

                    if state == self.original.start {
                        string_from = format!("\u{2192}{}", string_from);
                    }

                    for (_, through, to) in transitions {
                        let mut string_to = format!("{}", self.name_of(to));

                        if self.original.finish.contains(&to) {
                            string_to = format!("{}\u{2192}", string_to);
                        }

                        if to == self.original.start {
                            string_to = format!("\u{2192}{}", string_to);
                        }

                        writeln!(
                            self.formatter,
                            "    {} \u{21D2} {:} \u{21D2} {}",
                            string_from, through, string_to,
                        )?;

                        if !self.visited.contains(&to) {
                            let _ = self.visited.insert(to);
                            self.pending.push_back(to);
                        }
                    }
                }

                Ok(())
            }

            #[inline]
            fn name_of(&mut self, state: State) -> usize {
                *self.names.entry(state).or_insert_with(|| {
                    self.generator
                        .next()
                        .expect("Internal error. Display state generator exceeded.")
                })
            }
        }

        let mut visitor = Visitor {
            original: self,
            formatter,
            pending: VecDeque::from([self.start]),
            visited: Set::new([self.start]),
            names: Map::empty(),
            generator: 1..,
        };

        while !visitor.pending.is_empty() {
            visitor.next()?
        }

        Ok(())
    }
}

impl<C: AutomataContext> Automata<C> {
    #[inline(always)]
    pub fn accepts_null(&self) -> bool {
        self.finish.contains(&self.start) || self.transitions.is_empty()
    }

    #[inline(always)]
    pub fn start(&self) -> State {
        self.start
    }

    #[inline(always)]
    pub fn finish(&self) -> &Set<State> {
        &self.finish
    }

    #[inline(always)]
    pub fn transitions(&self) -> &Transitions<C::Terminal> {
        &self.transitions
    }

    #[inline(always)]
    pub fn try_map(
        &mut self,
        map: impl FnMut(&State, &mut Set<(C::Terminal, State)>) -> Result<()>,
    ) -> Result<()> {
        self.transitions.try_map(map)
    }

    #[inline(always)]
    pub fn retain(&mut self, map: impl FnMut(&State, &C::Terminal, &State) -> bool) {
        self.transitions.retain(map)
    }

    pub(super) fn canonicalize(&mut self, context: &mut C) {
        let (deterministic, alphabet, transitions) =
            take(&mut self.transitions).into_reversed(self.finish.is_single());

        match deterministic {
            true => {
                self.transitions = transitions;

                let finish = match self.finish.single() {
                    Some(finish) => finish,
                    None => system_panic!("Reversed DFA with multiple start states."),
                };

                self.finish = Set::new([self.start]);
                self.start = finish;
            }

            false => {
                *self = Deterministic::build(
                    context,
                    &alphabet,
                    &self.finish,
                    &Set::new([self.start]),
                    &transitions,
                );
            }
        }

        let (deterministic, alphabet, transitions) =
            take(&mut self.transitions).into_reversed(self.finish.is_single());

        match deterministic {
            true => {
                self.transitions = transitions;

                let finish = match self.finish.single() {
                    Some(finish) => finish,
                    None => system_panic!("Reversed DFA with multiple start states."),
                };

                self.finish = Set::new([self.start]);
                self.start = finish;
            }

            false => {
                *self = Deterministic::build(
                    context,
                    &alphabet,
                    &self.finish,
                    &Set::new([self.start]),
                    &transitions,
                );
            }
        }
    }

    #[inline(always)]
    pub(super) fn determine(&mut self, context: &mut C) {
        let (deterministic, alphabet) = self.transitions.meta();

        if deterministic {
            return;
        }

        *self = Deterministic::build(
            context,
            &alphabet,
            &Set::new([self.start]),
            &self.finish,
            &self.transitions,
        );
    }

    #[cfg(test)]
    pub(super) fn test(&self, input: Vec<C::Terminal>) -> bool {
        use crate::utils::context::AutomataTerminal;

        let mut state = self.start;

        'outer: for terminal in &input {
            for (from, through, to) in &self.transitions {
                if from != state {
                    continue;
                }

                assert!(!through.is_null(), "Automata with null-transition.");

                if through == terminal {
                    state = to;
                    continue 'outer;
                }
            }

            return false;
        }

        self.finish.contains(&state)
    }
}
