////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{
    collections::hash_map::Keys,
    fmt::{Display, Formatter},
};

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{Error, Result};

use crate::{
    node::automata::{NodeAutomata, Terminal},
    utils::{error, expect_some, Facade, Map, PredictableCollection, Set, SetImpl, State},
};

#[derive(Default)]
pub(super) struct VariableMap {
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

impl<'a> TryFrom<&'a NodeAutomata> for VariableMap {
    type Error = Error;

    fn try_from(automata: &'a NodeAutomata) -> Result<Self> {
        let mut kinds = Map::empty();

        for (_, through, _) in automata.transitions() {
            match through {
                Terminal::Token(Some(capture), _) => {
                    match kinds.insert(capture.clone(), VariableKind::TokenRef) {
                        Some(VariableKind::NodeRef) => {
                            return Err(error!(
                                capture.span(),
                                "Variable \"{capture}\" captures two distinct \
                                types: TokenRef and NodeRef.",
                            ))
                        }

                        _ => (),
                    }
                }

                Terminal::Node(Some(capture), _) => {
                    match kinds.insert(capture.clone(), VariableKind::NodeRef) {
                        Some(VariableKind::TokenRef) => {
                            return Err(error!(
                                capture.span(),
                                "Variable \"{capture}\" captures two distinct \
                                types: TokenRef and NodeRef.",
                            ))
                        }
                        _ => (),
                    }
                }

                _ => (),
            }
        }

        let mut result = Map::with_capacity(kinds.len());

        for (capture, kind) in kinds {
            let mut optional = Set::new([automata.start()]);
            automata.spread_without(&capture, &mut optional);

            let mut single = automata.step_with(&capture, &optional);
            automata.spread_without(&capture, &mut single);

            let mut multiple = automata.step_with(&capture, &single);
            automata.spread(&mut multiple);

            let mut is_optional = false;
            let mut is_multiple = false;

            for finish in automata.finish() {
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

        Ok(Self { map: result })
    }
}

impl ToTokens for VariableMap {
    #[inline]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for meta in self.map.values() {
            meta.to_tokens(tokens);
        }
    }
}

impl VariableMap {
    #[inline(always)]
    pub(super) fn contains(&self, variable: &Ident) -> bool {
        self.map.contains_key(variable)
    }

    #[inline(always)]
    pub(super) fn len(&self) -> usize {
        self.map.len()
    }

    #[inline(always)]
    pub(super) fn get(&self, variable: &Ident) -> &VariableMeta {
        expect_some!(self.map.get(variable), "Missing variable \"{variable}\".",)
    }

    pub(super) fn init(&self) -> TokenStream {
        let mut tokens = TokenStream::new();

        for meta in self.map.values() {
            meta.init().to_tokens(&mut tokens);
        }

        tokens
    }
}

pub(super) struct VariableMeta {
    name: Ident,
    kind: VariableKind,
    repetition: VariableRepetition,
}

impl Display for VariableMeta {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        use VariableRepetition::*;

        match self.repetition {
            Single => formatter.write_fmt(format_args!("{:?}", self.kind)),
            Optional => formatter.write_fmt(format_args!("{:?}?", self.kind)),
            Multiple => formatter.write_fmt(format_args!("{:?}*", self.kind)),
        }
    }
}

impl ToTokens for VariableMeta {
    #[inline(always)]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let span = name.span();

        format_ident!("capture_{name}", span = span).to_tokens(tokens)
    }
}

impl VariableMeta {
    pub(super) fn write(&self, value: TokenStream) -> TokenStream {
        use VariableRepetition::*;

        let span = self.name.span();

        match &self.repetition {
            Single | Optional => quote_spanned!(span=> #self = #value;),
            Multiple => {
                let vec = span.face_vec();

                quote_spanned!(span=> #vec::push(&mut #self, #value);)
            }
        }
    }

    pub(super) fn write_nil(&self) -> TokenStream {
        use VariableKind::*;
        use VariableRepetition::*;

        if let Multiple = &self.repetition {
            let span = self.name.span();
            let core = span.face_core();
            let vec = span.face_vec();

            return match &self.kind {
                TokenRef => {
                    quote_spanned!(span=> #vec::push(&mut #self, #core::lexis::TokenRef::nil());)
                }
                NodeRef => {
                    quote_spanned!(span=> #vec::push(&mut #self, #core::syntax::NodeRef::nil());)
                }
            };
        }

        TokenStream::new()
    }

    pub(super) fn ty(&self) -> TokenStream {
        use VariableKind::*;
        use VariableRepetition::*;

        let span = self.name.span();
        let core = span.face_core();

        match (&self.kind, &self.repetition) {
            (TokenRef, Single | Optional) => {
                quote_spanned!(span=> #core::lexis::TokenRef)
            }

            (NodeRef, Single | Optional) => {
                quote_spanned!(span=> #core::syntax::NodeRef::nil())
            }

            (TokenRef, Multiple) => {
                let vec = span.face_vec();

                quote_spanned!(span=> #vec::<#core::lexis::TokenRef>)
            }

            (NodeRef, Multiple) => {
                let vec = span.face_vec();

                quote_spanned!(span=> #vec::<#core::syntax::NodeRef>)
            }
        }
    }

    fn init(&self) -> TokenStream {
        use VariableKind::*;
        use VariableRepetition::*;

        let span = self.name.span();
        let core = span.face_core();

        match (&self.kind, &self.repetition) {
            (TokenRef, Single | Optional) => {
                quote_spanned!(span=>
                    let mut #self = #core::lexis::TokenRef::nil();
                )
            }

            (NodeRef, Single | Optional) => {
                quote_spanned!(span=>
                    let mut #self = #core::syntax::NodeRef::nil();
                )
            }

            (TokenRef, Multiple) => {
                let vec = span.face_vec();

                quote_spanned!(span=>
                    let mut #self = #vec::<#core::lexis::TokenRef>::with_capacity(1);
                )
            }

            (NodeRef, Multiple) => {
                let vec = span.face_vec();

                quote_spanned!(span=>
                    let mut #self = #vec::<#core::syntax::NodeRef>::with_capacity(1);
                )
            }
        }
    }
}

#[derive(Debug)]
enum VariableKind {
    TokenRef,
    NodeRef,
}

enum VariableRepetition {
    Single,
    Optional,
    Multiple,
}

impl AutomataExt for NodeAutomata {
    #[inline]
    fn spread(&self, states: &mut Set<State>) {
        loop {
            let mut new_states = false;

            for (from, _, to) in self.transitions() {
                if !states.contains(&from) || states.contains(&to) {
                    continue;
                }

                let _ = states.insert(to);
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
                if !states.contains(&from) || states.contains(&to) {
                    continue;
                }

                let transits = match through {
                    Terminal::Token(Some(capture), _) => capture == variable,
                    Terminal::Node(Some(capture), _) => capture == variable,
                    _ => false,
                };

                if !transits {
                    let _ = states.insert(to);
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
            if !states.contains(&from) || result.contains(&to) {
                continue;
            }

            let transits = match through {
                Terminal::Token(Some(capture), _) => capture == variable,
                Terminal::Node(Some(capture), _) => capture == variable,
                _ => false,
            };

            if transits {
                let _ = result.insert(to);
            }
        }

        result
    }
}

trait AutomataExt {
    fn spread(&self, states: &mut Set<State>);

    fn spread_without(&self, variable: &Ident, states: &mut Set<State>);

    fn step_with(&self, variable: &Ident, states: &Set<State>) -> Set<State>;
}
