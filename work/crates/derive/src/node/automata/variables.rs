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
    collections::hash_map::Keys,
    fmt::{Display, Formatter},
};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::{
    node::{automata::NodeAutomata, builder::constructor::Constructor, regex::terminal::Terminal},
    utils::{Map, PredictableCollection, Set, SetImpl, State},
};

impl AutomataVariables for NodeAutomata {
    fn variable_map(&self) -> Result<VariableMap> {
        let mut kinds = Map::empty();

        for (_, through, _) in self.transitions() {
            match through {
                Terminal::Token {
                    capture: Some(capture),
                    ..
                } => match kinds.insert(capture.clone(), VariableKind::TokenRef) {
                    Some(VariableKind::NodeRef) => {
                        return Err(Error::new(
                            capture.span(),
                            format!(
                                "Variable {:?} captures two distinct types: TokenRef and NodeRef.",
                                capture.to_string(),
                            ),
                        ))
                    }
                    _ => (),
                },

                Terminal::Node {
                    capture: Some(capture),
                    ..
                } => match kinds.insert(capture.clone(), VariableKind::NodeRef) {
                    Some(VariableKind::TokenRef) => {
                        return Err(Error::new(
                            capture.span(),
                            format!(
                                "Variable {:?} captures two distinct types: TokenRef and NodeRef.",
                                capture.to_string(),
                            ),
                        ))
                    }
                    _ => (),
                },

                _ => (),
            }
        }

        let mut result = Map::with_capacity(kinds.len());

        for (capture, kind) in kinds {
            let mut optional = Set::new([*self.start()]);
            self.spread_without(&capture, &mut optional);

            let mut single = self.step_with(&capture, &optional);
            self.spread_without(&capture, &mut single);

            let mut multiple = self.step_with(&capture, &single);
            self.spread(&mut multiple);

            let mut is_optional = false;
            let mut is_multiple = false;

            for finish in self.finish() {
                if optional.contains(finish) {
                    is_optional = true;
                }

                if multiple.contains(finish) {
                    is_multiple = true;
                }

                if is_optional && is_multiple {
                    break;
                }
            }

            let repetition = match (is_optional, is_multiple) {
                (_, true) => VariableRepetition::Multiple,
                (true, false) => VariableRepetition::Optional,
                (false, false) => VariableRepetition::Single,
            };

            result.insert(
                capture.clone(),
                VariableMeta {
                    name: capture,
                    kind,
                    repetition,
                },
            );
        }

        Ok(VariableMap { map: result })
    }
}

impl AutomataPrivate for NodeAutomata {
    #[inline]
    fn spread(&self, states: &mut Set<State>) {
        loop {
            let mut new_states = false;

            for (from, _, to) in self.transitions() {
                if !states.contains(&from) || states.contains(to) {
                    continue;
                }

                let _ = states.insert(*to);
                new_states = true;
            }

            if !new_states {
                break;
            }
        }
    }

    fn spread_without(&self, variable: &Ident, states: &mut Set<State>) {
        loop {
            let mut new_states = false;

            for (from, through, to) in self.transitions() {
                if !states.contains(&from) || states.contains(to) {
                    continue;
                }

                let transits = match through {
                    Terminal::Token {
                        capture: Some(capture),
                        ..
                    } => capture == variable,

                    Terminal::Node {
                        capture: Some(capture),
                        ..
                    } => capture == variable,

                    _ => false,
                };

                if !transits {
                    let _ = states.insert(*to);
                    new_states = true;
                }
            }

            if !new_states {
                break;
            }
        }
    }

    #[inline]
    fn step_with(&self, variable: &Ident, states: &Set<State>) -> Set<State> {
        let mut result = Set::empty();

        for (from, through, to) in self.transitions() {
            if !states.contains(&from) || result.contains(to) {
                continue;
            }

            let transits = match through {
                Terminal::Token {
                    capture: Some(capture),
                    ..
                } => capture == variable,

                Terminal::Node {
                    capture: Some(capture),
                    ..
                } => capture == variable,

                _ => false,
            };

            if transits {
                let _ = result.insert(*to);
            }
        }

        result
    }
}

pub(in crate::node) trait AutomataVariables {
    fn variable_map(&self) -> Result<VariableMap>;
}

trait AutomataPrivate {
    fn spread(&self, states: &mut Set<State>);

    fn spread_without(&self, variable: &Ident, states: &mut Set<State>);

    fn step_with(&self, variable: &Ident, states: &Set<State>) -> Set<State>;
}

#[derive(Default)]
pub(in crate::node) struct VariableMap {
    map: Map<Ident, VariableMeta>,
}

impl Display for VariableMap {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        for (key, variable) in &self.map {
            writeln!(formatter, "    {}: {}", key, variable)?;
        }

        Ok(())
    }
}

impl<'a> IntoIterator for &'a VariableMap {
    type Item = &'a Ident;
    type IntoIter = Keys<'a, Ident, VariableMeta>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.map.keys()
    }
}

impl VariableMap {
    pub(in crate::node) fn fits(&self, constructor: &Constructor) -> Result<()> {
        let explicit = constructor.is_explicit();
        let parameters = constructor
            .parameters()
            .iter()
            .map(|parameter| (parameter.name(), parameter))
            .collect::<Map<_, _>>();

        for (name, parameter) in &parameters {
            if self.map.contains_key(name) {
                if parameter.is_default() {
                    return Err(Error::new(
                        parameter.default_attribute().clone(),
                        "Default attribute is not applicable here, because corresponding \
                        variable is explicitly captured in the rule expression.",
                    ));
                }
            } else {
                if explicit {
                    return Err(Error::new(
                        name.span(),
                        "This parameter is missing in the set of the rule capturing \
                        variables.",
                    ));
                } else if !parameter.is_default() {
                    return Err(Error::new(
                        name.span(),
                        "This parameter is missing in the set of the rule capturing \
                        variables.\nIf this is intended, the rule needs an explicit constructor.\n\
                        Use #[constructor(...)] attribute to specify constructor function.\n\
                        Alternatively, associate this parameter with #[default(...)] attribute.",
                    ));
                }
            }
        }

        for argument in self.map.keys() {
            if !parameters.contains_key(argument) {
                return if explicit {
                    Err(Error::new(
                        argument.span(),
                        format!(
                            "Capturing \"{}\" variable is missing in constructor's parameters.",
                            argument,
                        ),
                    ))
                } else {
                    Err(Error::new(
                        argument.span(),
                        format!(
                            "Capturing \"{}\" variable is missing in the list of variant fields.",
                            argument,
                        ),
                    ))
                };
            }
        }

        Ok(())
    }

    #[inline(always)]
    pub(in crate::node) fn get(&self, name: &Ident) -> &VariableMeta {
        self.map
            .get(name)
            .expect("Internal error. Missing variable.")
    }
}

pub(in crate::node) struct VariableMeta {
    name: Ident,
    kind: VariableKind,
    repetition: VariableRepetition,
}

impl Display for VariableMeta {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        use VariableRepetition::*;

        let kind = format!("{:?}", self.kind);

        match self.repetition {
            Single => formatter.write_str(&format!("{}", kind)),
            Optional => formatter.write_str(&format!("{}?", kind)),
            Multiple => formatter.write_str(&format!("{}*", kind)),
        }
    }
}

impl VariableMeta {
    #[inline(always)]
    pub(in crate::node) fn name(&self) -> &Ident {
        &self.name
    }

    #[inline(always)]
    pub(in crate::node) fn kind(&self) -> &VariableKind {
        &self.kind
    }

    #[inline(always)]
    pub(in crate::node) fn repetition(&self) -> &VariableRepetition {
        &self.repetition
    }
}

#[derive(Debug)]
pub(in crate::node) enum VariableKind {
    TokenRef,
    NodeRef,
}

pub(in crate::node) enum VariableRepetition {
    Single,
    Optional,
    Multiple,
}
