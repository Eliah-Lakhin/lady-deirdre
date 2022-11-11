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

use crate::utils::{AutomataTerminal, PredictableCollection, Set, SetImpl, State};

pub type Transitions<S, T> = Set<(S, T, S)>;

impl<C, S, T> TransitionsImpl<C> for Transitions<S, T>
where
    S: State<C>,
    T: AutomataTerminal,
{
    type State = S;
    type Terminal = T;

    #[inline(always)]
    fn through(&mut self, from: Self::State, symbol: Self::Terminal, to: Self::State) {
        let _ = self.insert((from, symbol, to));
    }

    #[inline(always)]
    fn through_null(&mut self, from: Self::State, to: Self::State) {
        self.through(from, <Self::Terminal as AutomataTerminal>::null(), to);
    }

    #[inline]
    fn alphabet(&self) -> Set<Self::Terminal> {
        self.iter().map(|(_, symbol, _)| symbol).cloned().collect()
    }

    fn closure_of(&self, state: Self::State, symbol: Self::Terminal) -> Set<Self::State> {
        let mut closure = Set::empty();

        if symbol.is_null() {
            self.closure_of_null(state, &mut closure);

            return closure;
        }

        for (from, through, to) in self {
            if from == &state && through == &symbol {
                let mut null_closure = Set::empty();

                self.closure_of_null(*to, &mut null_closure);

                closure.append(null_closure);
            }
        }

        closure
    }

    fn closure_of_null(&self, state: Self::State, closure: &mut Set<Self::State>) {
        let _ = closure.insert(state);

        for (from, through, to) in self {
            if from == &state && through.is_null() {
                let to = *to;

                if closure.insert(to) {
                    self.closure_of_null(to, closure);
                }
            }
        }
    }
}

pub(super) trait TransitionsImpl<C> {
    type State: State<C>;
    type Terminal: AutomataTerminal;

    fn through(&mut self, from: Self::State, symbol: Self::Terminal, to: Self::State);

    fn through_null(&mut self, from: Self::State, to: Self::State);

    fn alphabet(&self) -> Set<Self::Terminal>;

    fn closure_of(&self, state: Self::State, symbol: Self::Terminal) -> Set<Self::State>;

    fn closure_of_null(&self, state: Self::State, closures: &mut Set<Self::State>);
}
