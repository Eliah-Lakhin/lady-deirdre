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

use std::{hash::Hash, mem::replace};

use crate::utils::{
    automata::Automata,
    transitions::{Transitions, TransitionsImpl},
    PredictableCollection,
    Set,
    SetImpl,
    State,
};

pub trait AutomataContext: Sized {
    type State: State<Self>;
    type Terminal: AutomataTerminal;

    fn terminal(&mut self, terminals: Set<Self::Terminal>) -> Automata<Self> {
        if terminals.is_empty() {
            unreachable!("An attempt to create a terminal of empty set.");
        }

        let start = Self::State::gen_state(self);
        let finish = Self::State::gen_state(self);

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
        let start = Self::State::gen_state(self);

        a.transitions.append(b.transitions);

        a.transitions.through_null(start, a.start);
        a.transitions.through_null(start, b.start);

        a.start = start;
        a.finish.append(b.finish);

        a.canonicalize(self);

        a
    }

    fn concatenate(&mut self, mut a: Automata<Self>, b: Automata<Self>) -> Automata<Self> {
        for a_finish in replace(&mut a.finish, b.finish) {
            a.transitions.through_null(a_finish, b.start);
        }

        a.transitions.append(b.transitions);

        a.canonicalize(self);

        a
    }

    fn repeat(&mut self, mut inner: Automata<Self>) -> Automata<Self> {
        for finish in &inner.finish {
            inner.transitions.through_null(*finish, inner.start);
            inner.transitions.through_null(inner.start, *finish);
        }

        inner.canonicalize(self);

        inner
    }

    fn optional(&mut self, mut inner: Automata<Self>) -> Automata<Self> {
        let start = Self::State::gen_state(self);

        inner.finish.insert(start);
        inner
            .transitions
            .through_null(start, replace(&mut inner.start, start));

        inner.canonicalize(self);

        inner
    }
}

pub trait AutomataTerminal: Clone + Eq + Hash {
    fn null() -> Self;

    fn is_null(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use std::ops::RangeFrom;

    use crate::utils::{AutomataContext, AutomataTerminal, Set, SetImpl, State};

    struct TestContext(RangeFrom<TestState>);

    impl AutomataContext for TestContext {
        type State = TestState;
        type Terminal = TestTerminal;
    }

    type TestTerminal = &'static str;

    impl AutomataTerminal for TestTerminal {
        #[inline(always)]
        fn null() -> Self {
            ""
        }

        #[inline(always)]
        fn is_null(&self) -> bool {
            self.is_empty()
        }
    }

    type TestState = usize;

    impl State<TestContext> for TestState {
        fn gen_state(context: &mut TestContext) -> Self {
            context.0.next().unwrap()
        }
    }

    #[test]
    fn test_automata() {
        let mut context = TestContext(1..);

        let foo = context.terminal(Set::new(["foo"]));
        let bar = context.terminal(Set::new(["bar"]));
        let comma = context.terminal(Set::new([","]));

        assert!(foo.test(vec!["foo"]));
        assert!(!foo.test(vec!["bar"]));
        assert!(!foo.test(vec![]));

        let foo_or_bar = context.union(foo, bar);
        let comma_foo_or_bar = context.concatenate(comma, foo_or_bar.clone());
        let repeat_comma_foo_or_bar = context.repeat(comma_foo_or_bar);
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
