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

use std::cmp::Ordering;

use crate::node::{
    automata::NodeAutomata,
    builder::{kind::VariantKind, Builder},
    regex::terminal::Terminal,
};
use crate::utils::{debug_panic, State};

pub(in crate::node) type TransitionsVector<'a> = Vec<(&'a State, &'a Terminal, &'a State)>;

impl<'a> TransitionsVectorImpl<'a> for TransitionsVector<'a> {
    fn outgoing(automata: &'a NodeAutomata, state: &'a State) -> Self {
        let mut outgoing = match automata.transitions().outgoing(state) {
            None => return vec![],

            Some(outgoing) => outgoing
                .iter()
                .map(|(through, to)| (state, through, to))
                .collect::<Vec<_>>(),
        };

        outgoing.sort_by(|a, b| {
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

        outgoing
    }

    fn filter_skip(self, builder: &Builder) -> Self {
        let skip_tokens = builder.skip_leftmost().tokens();

        self.into_iter()
            .filter(|(_, through, _)| match through {
                Terminal::Null => debug_panic!("Automata with null transition."),

                Terminal::Token { name, .. } => !skip_tokens.contains(name),

                Terminal::Node { name, .. } => match builder.variant(name).kind() {
                    VariantKind::Comment(..) => false,
                    _ => true,
                },
            })
            .collect()
    }

    fn split_terminals(&self) -> (Vec<String>, Vec<String>) {
        let mut tokens = Vec::with_capacity(self.len());
        let mut nodes = Vec::with_capacity(self.len());

        for (_, through, _) in self {
            match through {
                Terminal::Null => debug_panic!("Automata with null transition."),

                Terminal::Token { name, .. } => tokens.push(name.to_string()),

                Terminal::Node { name, .. } => nodes.push(name.to_string()),
            }
        }

        (tokens, nodes)
    }
}

pub(in crate::node) trait TransitionsVectorImpl<'a> {
    fn outgoing(automata: &'a NodeAutomata, state: &'a State) -> Self;

    fn filter_skip(self, builder: &Builder) -> Self;

    fn split_terminals(&self) -> (Vec<String>, Vec<String>);
}
