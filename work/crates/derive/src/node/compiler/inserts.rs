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

use std::vec::IntoIter;

use proc_macro2::Ident;

use crate::{
    node::{
        automata::{scope::SyntaxState, NodeAutomata},
        builder::Builder,
        compiler::transitions::{TransitionsVector, TransitionsVectorImpl},
        regex::terminal::Terminal,
    },
    utils::{PredictableCollection, Set},
};

pub(in crate::node) struct Insert<'a> {
    matching: &'a Ident,
    expected_terminal: &'a Terminal,
    destination_terminal: &'a Terminal,
    destination_state: &'a SyntaxState,
}

impl<'a> Insert<'a> {
    #[inline(always)]
    pub(in crate::node) fn matching(&self) -> &'a Ident {
        self.matching
    }

    #[inline(always)]
    pub(in crate::node) fn expected_terminal(&self) -> &'a Terminal {
        self.expected_terminal
    }

    #[inline(always)]
    pub(in crate::node) fn destination_terminal(&self) -> &'a Terminal {
        self.destination_terminal
    }

    #[inline(always)]
    pub(in crate::node) fn destination_state(&self) -> &'a SyntaxState {
        self.destination_state
    }
}

pub(in crate::node) struct InsertRecovery<'a> {
    forbidden: Set<&'a Ident>,
    inserts: Vec<Insert<'a>>,
}

impl<'a> IntoIterator for InsertRecovery<'a> {
    type Item = Insert<'a>;
    type IntoIter = IntoIter<Insert<'a>>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.inserts.into_iter()
    }
}

impl<'a> InsertRecovery<'a> {
    pub(in crate::node) fn prepare(
        builder: &'a Builder,
        automata: &'a NodeAutomata,
        outgoing: &TransitionsVector<'a>,
    ) -> Self {
        let mut recovery = Self {
            forbidden: Set::with_capacity(outgoing.len()),
            inserts: Vec::new(),
        };

        for (_, through, _) in outgoing {
            match through {
                Terminal::Null => unreachable!("Automata with null transition."),

                Terminal::Token { name, .. } => {
                    let _ = recovery.forbidden.insert(name);
                }

                Terminal::Node { name, .. } => {
                    let leftmost = builder.variant(name).leftmost();

                    for token in leftmost.tokens() {
                        let _ = recovery.forbidden.insert(token);
                    }
                }
            }
        }

        for (_, expected_terminal, expected_state) in outgoing {
            let destination =
                TransitionsVector::outgoing(automata, expected_state).filter_skip(builder);

            for (_, destination_terminal, destination_state) in destination {
                match destination_terminal {
                    Terminal::Null => unreachable!("Automata with null transition."),

                    Terminal::Token { name: matching, .. } => {
                        if !recovery.forbid(matching) {
                            recovery.inserts.push(Insert {
                                matching,
                                expected_terminal,
                                destination_terminal,
                                destination_state,
                            });
                        }
                    }

                    Terminal::Node { name, .. } => {
                        let leftmost = builder.variant(name).leftmost();

                        for matching in leftmost.tokens() {
                            if !recovery.forbid(matching) {
                                recovery.inserts.push(Insert {
                                    matching,
                                    expected_terminal,
                                    destination_terminal,
                                    destination_state,
                                });
                            }
                        }
                    }
                }
            }
        }

        recovery
    }

    fn forbid(&mut self, matching: &'a Ident) -> bool {
        if self.forbidden.contains(matching) {
            return true;
        }

        let mut found = false;

        self.inserts.retain(|insert| {
            if insert.matching == matching {
                found = true;
                return false;
            }

            true
        });

        if found {
            let _ = self.forbidden.insert(matching);
        }

        found
    }
}
