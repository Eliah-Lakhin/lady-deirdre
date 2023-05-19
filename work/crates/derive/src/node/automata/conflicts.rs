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
use syn::{spanned::Spanned, Error, Result};

use crate::{
    node::{automata::NodeAutomata, builder::Builder, regex::terminal::Terminal},
    utils::{debug_panic, Map, PredictableCollection, State},
};

impl CheckConflicts for NodeAutomata {
    fn check_conflicts(&self, builder: &Builder, allow_skips: bool) -> Result<()> {
        struct OutgoingView<'a> {
            map: Map<&'a State, Map<&'a Ident, &'a Terminal>>,
        }

        impl<'a> OutgoingView<'a> {
            fn insert(
                &mut self,
                from: &'a State,
                token: &'a Ident,
                terminal: &'a Terminal,
            ) -> Result<()> {
                let map = self.map.entry(from).or_insert_with(|| Map::empty());

                if let Some(existed) = map.insert(token, terminal) {
                    let mut message = String::new();

                    match terminal {
                        Terminal::Null => debug_panic!("Automata with null transition."),

                        Terminal::Token { name, .. } => {
                            message.push_str(&format!(
                                "Token matching \"${}\" conflicts with ",
                                name.to_string()
                            ));
                        }

                        Terminal::Node { name, .. } => {
                            message.push_str(&format!(
                                "Rule {:?} with \"${}\" token in the leftmost position conflicts \
                                with ",
                                name.to_string(),
                                token.to_string(),
                            ));
                        }
                    }

                    match existed {
                        Terminal::Null => debug_panic!("Automata with null transition."),

                        Terminal::Token { .. } => {
                            message.push_str("matching of the same token in this expression.");
                        }

                        Terminal::Node { name, .. } => {
                            message.push_str(&format!(
                                "rule {:?} that contains the same token match in its leftmost \
                                position.",
                                name.to_string(),
                            ));
                        }
                    }

                    return Err(Error::new(terminal.span(), message));
                }

                Ok(())
            }
        }

        let mut view = OutgoingView { map: Map::empty() };

        for (from, through, _) in self.transitions() {
            match through {
                Terminal::Null => debug_panic!("Automata with null transition."),

                Terminal::Token { name, capture } => {
                    if let Some(capture) = capture {
                        if !allow_skips && builder.skip_leftmost().tokens().contains(name) {
                            return Err(Error::new(
                                name.span(),
                                format!(
                                    "Token capturing \"{}: ${}\" conflicts with Skip expression.",
                                    capture, name,
                                ),
                            ));
                        }
                    }

                    view.insert(from, name, through)?;
                }

                Terminal::Node { name, .. } => {
                    for token in builder.variant(name).leftmost().tokens() {
                        view.insert(from, token, through)?;
                    }
                }
            }
        }

        Ok(())
    }
}

pub(in crate::node) trait CheckConflicts {
    fn check_conflicts(&self, builder: &Builder, allow_skips: bool) -> Result<()>;
}
