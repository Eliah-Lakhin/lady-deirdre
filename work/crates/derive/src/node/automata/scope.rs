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

use crate::{
    node::regex::terminal::Terminal,
    utils::{AutomataContext, OptimizationStrategy, State},
};

pub(in crate::node) struct Scope {
    state: State,
    strategy: OptimizationStrategy,
}

impl Default for Scope {
    #[inline(always)]
    fn default() -> Self {
        Self {
            state: 1,
            strategy: OptimizationStrategy::CANONICALIZE,
        }
    }
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
    fn strategy(&self) -> &OptimizationStrategy {
        &self.strategy
    }
}

impl Scope {
    #[inline(always)]
    pub fn set_strategy(&mut self, strategy: OptimizationStrategy) {
        self.strategy = strategy;
    }
}
