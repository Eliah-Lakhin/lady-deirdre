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

use std::mem::take;

use syn::{Error, Result};

use crate::{
    node::{
        automata::{scope::Scope, NodeAutomata},
        regex::terminal::Terminal,
    },
    utils::{Map, MapImpl, MultimapImpl, PredictableCollection, SetImpl},
};

impl AutomataMergeCaptures for NodeAutomata {
    fn merge_captures(&mut self, scope: &mut Scope) -> Result<()> {
        loop {
            let mut has_changes = false;

            self.transitions = take(&mut self.transitions)
                .group(|(from, through, to)| (from, (through, to)))
                .try_for_each(|_, transitions| {
                    let count = transitions.len();

                    let mut tokens = Map::with_capacity(count);
                    let mut nodes = Map::with_capacity(count);

                    for (terminal, to) in take(transitions) {
                        match &terminal {
                            Terminal::Null => unreachable!("Automata with null transition."),

                            Terminal::Token {
                                name,
                                capture: None,
                            } => {
                                if !tokens.contains_key(name) {
                                    let _ = tokens.insert(name.clone(), (terminal, to));
                                }
                            }

                            rule_a @ Terminal::Token {
                                name,
                                capture: Some(capture),
                            } => match tokens.get(name) {
                                None | Some((Terminal::Token { capture: None, .. }, _)) => {
                                    let _ = tokens.insert(name.clone(), (terminal, to));
                                }

                                Some((
                                    rule_b @ Terminal::Token {
                                        capture: Some(_), ..
                                    },
                                    _,
                                )) => {
                                    return Err(Error::new(
                                        capture.span(),
                                        format!(
                                            "Rule \"{}\" conflicts with rule \"{}\" by capturing \
                                            the same Token in the same source code position into \
                                            two distinct variables.",
                                            rule_a, rule_b,
                                        ),
                                    ))
                                }
                                _ => (),
                            },

                            Terminal::Node {
                                name,
                                capture: None,
                            } => {
                                if !nodes.contains_key(name) {
                                    let _ = nodes.insert(name.clone(), (terminal, to));
                                }
                            }

                            rule_a @ Terminal::Node {
                                name,
                                capture: Some(capture),
                            } => match nodes.get(name) {
                                None | Some((Terminal::Node { capture: None, .. }, _)) => {
                                    let _ = nodes.insert(name.clone(), (terminal, to));
                                }

                                Some((
                                    rule_b @ Terminal::Node {
                                        capture: Some(_), ..
                                    },
                                    _,
                                )) => {
                                    return Err(Error::new(
                                        capture.span(),
                                        format!(
                                            "Rule \"{}\" conflicts with rule \"{}\" by capturing \
                                            the same Node in the same source code position into \
                                            two distinct variables.",
                                            rule_a, rule_b,
                                        ),
                                    ))
                                }
                                _ => (),
                            },
                        }
                    }

                    for (_, token) in tokens {
                        transitions.insert(token);
                    }

                    for (_, node) in nodes {
                        transitions.insert(node);
                    }

                    if count != transitions.len() {
                        has_changes = true;
                    }

                    Ok(())
                })?
                .join(|from, (through, to)| (from, through, to));

            if !has_changes {
                break;
            }

            self.canonicalize(scope);
        }

        Ok(())
    }
}

pub(in crate::node) trait AutomataMergeCaptures {
    fn merge_captures(&mut self, scope: &mut Scope) -> Result<()>;
}
