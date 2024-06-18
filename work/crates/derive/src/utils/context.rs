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

use std::{hash::Hash, mem::replace};

use crate::utils::{
    automata::Automata,
    system_panic,
    transitions::Transitions,
    Map,
    PredictableCollection,
    Set,
    SetImpl,
};

pub type State = usize;

pub trait AutomataContext: Sized {
    type Terminal: AutomataTerminal;

    fn gen_state(&mut self) -> State;

    fn copy(&mut self, automata: &Automata<Self>) -> Automata<Self> {
        let mut state_map =
            Map::with_capacity(automata.transitions.len() + automata.finish.len() + 1);

        let start = self.gen_state();

        let _ = state_map.insert(automata.start, start);

        let mut transitions = automata.transitions.clone();

        transitions.rename(|state| *state_map.entry(state).or_insert_with(|| self.gen_state()));

        let finish = automata
            .finish
            .iter()
            .map(|state| *state_map.entry(*state).or_insert_with(|| self.gen_state()))
            .collect::<Set<_>>();

        Automata {
            start,
            finish,
            transitions,
        }
    }

    fn terminal(&mut self, terminals: Set<Self::Terminal>) -> Automata<Self> {
        if terminals.is_empty() {
            system_panic!("An attempt to create a terminal of empty set.");
        }

        let start = self.gen_state();
        let finish = self.gen_state();

        let mut transitions = Transitions::with_capacity(terminals.len());

        for terminal in terminals {
            transitions.through(start, terminal, finish);
        }

        Automata {
            start,
            finish: Set::new([finish]),
            transitions,
        }
    }

    fn union(&mut self, mut a: Automata<Self>, b: Automata<Self>) -> Automata<Self> {
        let mut fits = true;

        if let Some(a_outgoing) = a.transitions.outgoing(&a.start) {
            if let Some(b_outgoing) = b.transitions.outgoing(&b.start) {
                'outer: for a_out in a_outgoing {
                    for b_out in b_outgoing {
                        if a_out.0 == b_out.0 {
                            fits = false;
                            break 'outer;
                        }
                    }
                }
            }
        }

        if fits {
            a.transitions
                .rename_and_merge(b.transitions, b.start, a.start);

            for mut b_finish in b.finish {
                if b_finish == b.start {
                    b_finish = a.start;
                }

                a.finish.insert(b_finish);
            }

            return a;
        }

        let start = self.gen_state();

        a.transitions.merge(b.transitions);

        a.transitions.through_null(start, a.start);
        a.transitions.through_null(start, b.start);

        a.start = start;
        a.finish.append(b.finish);

        self.optimize(&mut a);

        a
    }

    fn concatenate(&mut self, mut a: Automata<Self>, b: Automata<Self>) -> Automata<Self> {
        if let Some(a_finish) = a.finish.single() {
            let mut fits = true;

            if let Some(b_outgoing) = b.transitions.outgoing(&b.start) {
                'outer: for (b_out, _) in b_outgoing {
                    if let Some(a_outgoing) = a.transitions.outgoing(&a_finish) {
                        for (a_out, _) in a_outgoing {
                            if a_out == b_out {
                                fits = false;
                                break 'outer;
                            }
                        }
                    }
                }
            }

            if fits {
                a.transitions
                    .rename_and_merge(b.transitions, b.start, a_finish);

                a.finish.clear();

                for mut b_finish in b.finish {
                    if b_finish == b.start {
                        b_finish = a_finish;
                    }

                    let _ = a.finish.insert(b_finish);
                }

                return a;
            }
        }

        for a_finish in replace(&mut a.finish, b.finish) {
            a.transitions.through_null(a_finish, b.start);
        }

        a.transitions.merge(b.transitions);

        self.optimize(&mut a);

        a
    }

    fn repeat_one(&mut self, mut inner: Automata<Self>) -> Automata<Self> {
        for finish in &inner.finish {
            inner.transitions.through_null(*finish, inner.start);
        }

        self.optimize(&mut inner);

        inner
    }

    fn repeat_zero(&mut self, mut inner: Automata<Self>) -> Automata<Self> {
        for finish in &inner.finish {
            inner.transitions.through_null(*finish, inner.start);
        }

        let start = self.gen_state();

        inner.transitions.through_null(start, inner.start);

        inner.start = start;
        inner.finish.insert(start);

        self.optimize(&mut inner);

        inner
    }

    fn optional(&mut self, mut inner: Automata<Self>) -> Automata<Self> {
        if inner.finish.contains(&inner.start) {
            return inner;
        }

        let start = self.gen_state();

        inner.transitions.through_null(start, inner.start);

        inner.start = start;
        inner.finish.insert(start);

        self.optimize(&mut inner);

        inner
    }

    fn optimize(&mut self, automata: &mut Automata<Self>) {
        match self.strategy() {
            Strategy::CANONICALIZE => automata.canonicalize(self),
            Strategy::DETERMINIZE => automata.determinize(self),
        }
    }

    fn strategy(&self) -> Strategy {
        Strategy::CANONICALIZE
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    DETERMINIZE,
    CANONICALIZE,
}

pub trait AutomataTerminal: Clone + Eq + Hash + 'static {
    fn null() -> Self;

    fn is_null(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use crate::utils::{AutomataContext, AutomataTerminal, Set, SetImpl, State, Strategy};

    struct TestContext(State, Strategy);

    impl AutomataContext for TestContext {
        type Terminal = TestTerminal;

        #[inline(always)]
        fn gen_state(&mut self) -> State {
            let state = self.0;

            self.0 += 1;

            state
        }

        #[inline(always)]
        fn strategy(&self) -> Strategy {
            self.1
        }
    }

    type TestTerminal = &'static str;

    impl AutomataTerminal for TestTerminal {
        #[inline(always)]
        fn null() -> Self {
            "ε"
        }

        #[inline(always)]
        fn is_null(&self) -> bool {
            self == &"ε"
        }
    }

    #[test]
    fn test_automata() {
        let mut context = TestContext(1, Strategy::CANONICALIZE);

        let foo = context.terminal(Set::new(["foo"]));
        let bar = context.terminal(Set::new(["bar"]));
        let comma = context.terminal(Set::new([","]));

        assert!(foo.test(vec!["foo"]));
        assert!(!foo.test(vec!["bar"]));
        assert!(!foo.test(vec![]));

        let foo_or_bar = context.union(foo, bar);
        let comma_foo_or_bar = {
            let foo_or_bar = context.copy(&foo_or_bar);
            context.concatenate(comma, foo_or_bar)
        };
        let repeat_comma_foo_or_bar = context.repeat_zero(comma_foo_or_bar);
        let one_or_more = context.concatenate(foo_or_bar, repeat_comma_foo_or_bar);

        assert!(!one_or_more.test(vec![]));

        let zero_or_more = context.optional(one_or_more);

        assert!(zero_or_more.test(vec![]));
        assert!(zero_or_more.test(vec!["foo"]));
        assert!(!zero_or_more.test(vec!["foo", "bar"]));
        assert!(zero_or_more.test(vec!["foo", ",", "bar"]));
        assert!(!zero_or_more.test(vec!["foo", ",", "bar", "foo"]));
        assert!(zero_or_more.test(vec!["foo", ",", "bar", ",", "foo"]));
        assert!(zero_or_more.test(vec!["foo", ",", "bar", ",", "foo", ",", "foo"]));
    }
}
