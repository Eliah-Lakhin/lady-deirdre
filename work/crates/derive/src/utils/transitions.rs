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

use std::{collections::BTreeSet, mem::take};

use syn::Result;

use crate::utils::{
    system_panic,
    AutomataTerminal,
    Map,
    PredictableCollection,
    Set,
    SetImpl,
    State,
};

#[derive(Clone)]
pub struct Transitions<T: AutomataTerminal> {
    view: Map<State, Set<(T, State)>>,
    length: usize,
}

impl<T: AutomataTerminal> Default for Transitions<T> {
    fn default() -> Self {
        Self {
            view: Map::empty(),
            length: 0,
        }
    }
}

impl<'a, T: AutomataTerminal> IntoIterator for &'a Transitions<T> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = TransitionsIter<'a, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            from_iterator: self.view.iter(),
            outgoing_iterator: None,
        }
    }
}

impl<T: AutomataTerminal> Transitions<T> {
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            view: Map::with_capacity(capacity),
            ..Default::default()
        }
    }

    #[inline]
    pub fn reverse(&mut self) {
        let mut view = Map::with_capacity(self.view.len());

        for (from, outgoing) in take(&mut self.view) {
            for (through, to) in outgoing {
                match view.get_mut(&to) {
                    None => {
                        let _ = view.insert(to, Set::new([(through, from)]));
                    }

                    Some(view) => {
                        let _ = view.insert((through, from));
                    }
                }
            }
        }

        self.view = view;
    }

    #[inline(always)]
    pub fn outgoing(&self, from: &State) -> Option<&Set<(T, State)>> {
        self.view.get(from)
    }

    #[inline(always)]
    pub fn view(&self) -> &Map<State, Set<(T, State)>> {
        &self.view
    }

    pub fn length(&self) -> usize {
        self.length
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.view.is_empty()
    }

    #[inline(always)]
    pub(super) fn try_map(
        &mut self,
        mut map: impl FnMut(&State, &mut Set<(T, State)>) -> Result<()>,
    ) -> Result<()> {
        for (from, outgoing) in &mut self.view {
            let before = outgoing.len();

            map(from, outgoing)?;

            let after = outgoing.len();

            if before > after {
                self.length -= before - after;
            }

            if after > before {
                self.length += after - before;
            }
        }

        Ok(())
    }

    #[inline(always)]
    pub(super) fn retain(&mut self, mut map: impl FnMut(&State, &T, &State) -> bool) {
        for (from, outgoing) in &mut self.view {
            outgoing.retain(|(through, to)| {
                let retain = map(from, through, to);

                if !retain {
                    self.length -= 1;
                }

                retain
            });
        }
    }

    #[inline]
    pub(super) fn rename(&mut self, mut map: impl FnMut(State) -> State) {
        self.view = take(&mut self.view)
            .into_iter()
            .map(|(mut from, outgoing)| {
                from = map(from);

                (
                    from,
                    outgoing
                        .into_iter()
                        .map(|(through, mut to)| {
                            to = map(to);

                            (through, to)
                        })
                        .collect(),
                )
            })
            .collect();
    }

    #[inline]
    pub(super) fn meta(&self) -> (bool, Set<T>) {
        let mut deterministic = true;
        let mut alphabet = Set::with_capacity(self.view.len());

        for (_, outgoing) in &self.view {
            let mut incoming = match deterministic {
                true => Map::empty(),
                false => Map::with_capacity(outgoing.len()),
            };

            for (through, to) in outgoing {
                match through.is_null() {
                    true => {
                        deterministic = false;
                    }
                    false => {
                        let _ = alphabet.insert(through.clone());
                    }
                }

                if deterministic {
                    if incoming.insert(to, through).is_some() {
                        deterministic = false;
                    }
                }
            }
        }

        (deterministic, alphabet)
    }

    #[inline]
    pub(super) fn into_reversed(self, mut deterministic: bool) -> (bool, Set<T>, Self) {
        let mut view = Map::with_capacity(self.view.len());
        let mut alphabet = Set::with_capacity(self.view.len());

        for (from, outgoing) in self.view {
            for (through, to) in outgoing {
                match through.is_null() {
                    true => {
                        deterministic = false;
                    }

                    false => {
                        let _ = alphabet.insert(through.clone());
                    }
                }

                match view.get_mut(&to) {
                    None => {
                        let _ = view.insert(to, Set::new([(through, from)]));
                    }

                    Some(view) => {
                        if deterministic {
                            for (current_terminal, current_state) in view.iter() {
                                if current_terminal == &through && current_state != &from {
                                    deterministic = false;
                                    break;
                                }
                            }
                        }

                        let _ = view.insert((through, from));
                    }
                }
            }
        }

        (
            deterministic,
            alphabet,
            Self {
                view,
                length: self.length,
            },
        )
    }

    #[inline(always)]
    pub(super) fn through(&mut self, from: State, symbol: T, to: State) {
        match self.view.get_mut(&from) {
            Some(outgoing) => {
                if outgoing.insert((symbol, to)) {
                    self.length += 1;
                }
            }

            None => {
                let _ = self.view.insert(from, Set::new([(symbol, to)]));
                self.length += 1;
            }
        }
    }

    #[inline(always)]
    pub(super) fn through_null(&mut self, from: State, to: State) {
        self.through(from, <T as AutomataTerminal>::null(), to);
    }

    #[inline(always)]
    pub(super) fn merge(&mut self, other: Self) {
        for (from, outgoing) in other.view {
            if self.view.insert(from, outgoing).is_some() {
                system_panic!("Merging of automatas with duplicate states.")
            }
        }

        self.length += other.length;
    }
}

pub struct TransitionsIter<'a, T: AutomataTerminal> {
    from_iterator: ::std::collections::hash_map::Iter<'a, State, Set<(T, State)>>,
    outgoing_iterator: Option<(
        &'a State,
        ::std::collections::hash_set::Iter<'a, (T, State)>,
    )>,
}

impl<'a, T: AutomataTerminal> Iterator for TransitionsIter<'a, T> {
    type Item = (State, &'a T, State);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.outgoing_iterator {
                Some((from, outgoing_iterator)) => match outgoing_iterator.next() {
                    Some((through, to)) => return Some((**from, through, *to)),
                    None => (),
                },

                None => (),
            }

            return match self.from_iterator.next() {
                None => None,
                Some((from, outgoing)) => {
                    self.outgoing_iterator = Some((from, outgoing.iter()));
                    continue;
                }
            };
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Default, Hash, PartialEq, Eq)]
pub(super) struct Closure {
    states: BTreeSet<State>,
}

impl<'a> IntoIterator for &'a Closure {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = ::std::collections::btree_set::Iter<'a, State>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.states.iter()
    }
}

impl Closure {
    #[inline(always)]
    pub(super) fn state(&self) -> Option<State> {
        if self.states.len() != 1 {
            return None;
        }

        self.states.iter().next().cloned()
    }

    #[inline(always)]
    pub(super) fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub(super) fn of<T: AutomataTerminal>(
        &mut self,
        transitions: &Transitions<T>,
        state: State,
        symbol: &T,
    ) {
        if symbol.is_null() {
            self.of_null(transitions, state);

            return;
        }

        if let Some(outgoing) = transitions.view.get(&state) {
            for (through, to) in outgoing {
                if through == symbol {
                    self.of_null(transitions, *to);
                }
            }
        }
    }

    pub(super) fn of_null<T: AutomataTerminal>(
        &mut self,
        transitions: &Transitions<T>,
        state: State,
    ) {
        let _ = self.states.insert(state);

        if let Some(outgoing) = transitions.view.get(&state) {
            for (through, to) in outgoing {
                if through.is_null() {
                    let to = *to;

                    if !self.states.contains(&to) {
                        self.of_null(transitions, to);
                    }
                }
            }
        }
    }
}
