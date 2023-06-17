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
    cmp::Ordering,
    fmt::{Display, Formatter},
    mem::take,
};

use proc_macro2::{Ident, Span};
use syn::Result;

use crate::{
    node::{input::VariantMap, rule::Rule, token::TokenLit},
    utils::{
        error,
        expect_some,
        null,
        system_panic,
        Automata,
        AutomataContext,
        AutomataTerminal,
        Map,
        PredictableCollection,
        State,
        Strategy,
    },
};

const RULE_LIMIT: usize = 16;

pub(super) type NodeAutomata = Automata<Scope>;

impl NodeAutomataImpl for NodeAutomata {
    fn merge_captures(&mut self, scope: &mut Scope) -> Result<()> {
        loop {
            let mut has_changes = false;

            self.try_map(|_, transitions| {
                let count = transitions.len();

                let mut tokens = Map::<TokenLit, (Terminal, State)>::with_capacity(count);
                let mut nodes = Map::<Ident, (Terminal, State)>::with_capacity(count);

                for (terminal, to) in take(transitions) {
                    match &terminal {
                        Terminal::Null => null!(),

                        rule_a @ Terminal::Token(None, lit) => {
                            if let Some((rule_b, rule_b_to)) = tokens.get(lit) {
                                if *rule_b_to != to {
                                    return Err(error!(
                                        lit.span(),
                                        "Rule \"{rule_a}\" conflicts with \
                                        capturing rule \"{rule_b}\" that leads \
                                        to the different execution flow.",
                                    ));
                                }
                            }

                            let _ = tokens.insert(lit.clone(), (terminal, to));
                        }

                        rule_a @ Terminal::Token(Some(capture), lit) => match tokens.get(lit) {
                            None | Some((Terminal::Token(None, _), _)) => {
                                if let Some((rule_b, rule_b_state)) = tokens.get(lit) {
                                    if *rule_b_state != to {
                                        return Err(error!(
                                            capture.span(),
                                            "Capturing rule \"{rule_a}\" \
                                            conflicts with rule \"{rule_b}\" \
                                            that leads to the different \
                                            execution flow.",
                                        ));
                                    }
                                }

                                let _ = tokens.insert(lit.clone(), (terminal, to));
                            }

                            Some((rule_b @ Terminal::Token(Some(_), _), _)) => {
                                return Err(error!(
                                    capture.span(),
                                    "Rule \"{rule_a}\" conflicts with rule \
                                    \"{rule_b}\" by capturing the same Token \
                                    in the same source code position into two \
                                    distinct variables.",
                                ))
                            }
                            _ => (),
                        },

                        rule_a @ Terminal::Node(None, name) => {
                            if let Some((rule_b, rule_b_to)) = nodes.get(name) {
                                if *rule_b_to != to {
                                    return Err(error!(
                                        name.span(),
                                        "Rule \"{rule_a}\" conflicts with \
                                        capturing rule \"{rule_b}\" that leads \
                                        to the different execution flow.",
                                    ));
                                }
                            }

                            let _ = nodes.insert(name.clone(), (terminal, to));
                        }

                        rule_a @ Terminal::Node(Some(capture), name) => match nodes.get(name) {
                            None | Some((Terminal::Node(None, _), _)) => {
                                if let Some((rule_b, rule_b_state)) = nodes.get(name) {
                                    if *rule_b_state != to {
                                        return Err(error!(
                                            capture.span(),
                                            "Capturing rule \"{rule_a}\" \
                                            conflicts with rule \"{rule_b}\" \
                                            that leads to the different \
                                            execution flow.",
                                        ));
                                    }
                                }

                                let _ = nodes.insert(name.clone(), (terminal, to));
                            }

                            Some((rule_b @ Terminal::Node(Some(_), _), _)) => {
                                return Err(error!(
                                    capture.span(),
                                    "Rule \"{rule_a}\" conflicts with rule \
                                    \"{rule_b}\" by capturing the same Node in \
                                    the same source code position into two \
                                    distinct variables.",
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
            })?;

            if !has_changes {
                break;
            }

            scope.optimize(self);
        }

        Ok(())
    }

    fn check_conflicts(&self, trivia: Option<&Rule>, map: &VariantMap) -> Result<()> {
        struct OutgoingView<'a> {
            map: Map<State, Map<&'a TokenLit, &'a Terminal>>,
        }

        impl<'a> OutgoingView<'a> {
            fn insert(
                &mut self,
                from: State,
                token: &'a TokenLit,
                terminal: &'a Terminal,
            ) -> Result<()> {
                let map = self.map.entry(from).or_insert_with(|| Map::empty());

                if let Some(existed) = map.insert(token, terminal) {
                    let mut message = String::new();

                    match terminal {
                        Terminal::Null => null!(),

                        Terminal::Token(_, name) => {
                            message
                                .push_str(&format!("Token matching \"{name}\" conflicts with ",));
                        }

                        Terminal::Node(_, name) => {
                            message.push_str(&format!(
                                "Rule \"{name}\" with \"{token}\" token in the \
                                leftmost position conflicts with ",
                            ));
                        }
                    }

                    match existed {
                        Terminal::Null => null!(),

                        Terminal::Token(..) => {
                            message.push_str(
                                "matching of the same token in this \
                                expression.",
                            );
                        }

                        Terminal::Node(_, name) => {
                            message.push_str(&format!(
                                "rule \"{name}\" that contains the same token \
                                match in its leftmost position.",
                            ));
                        }
                    }

                    return Err(error!(terminal.span(), "{}", message));
                }

                Ok(())
            }
        }

        let mut view = OutgoingView { map: Map::empty() };
        let mut concurrency = Map::with_capacity(self.transitions().len());

        for (from, through, _) in self.transitions() {
            match through {
                Terminal::Null => null!(),

                Terminal::Token(_, lit) => {
                    if let Some(trivia) = trivia {
                        let trivia_leftmost =
                            expect_some!(trivia.leftmost.as_ref(), "Missing trivia leftmost.",);

                        if trivia_leftmost.tokens().contains(lit) {
                            return Err(error!(
                                lit.span(),
                                "Token \"{lit}\" conflicts with Trivia expression.",
                            ));
                        }
                    }

                    view.insert(from, lit, through)?;
                }

                Terminal::Node(_, name) => {
                    let variant = expect_some!(map.get(name), "Unresolved reference.",);
                    let rule = expect_some!(variant.rule.as_ref(), "Missing rule.",);
                    let leftmost = expect_some!(rule.leftmost.as_ref(), "Missing leftmost",);
                    let matches = expect_some!(leftmost.matches(), "Unresolved leftmost matches.",);

                    for lit in matches {
                        if let Some(trivia) = trivia {
                            let trivia_leftmost =
                                expect_some!(trivia.leftmost.as_ref(), "Missing trivia leftmost.",);

                            if trivia_leftmost.tokens().contains(lit) {
                                return Err(error!(
                                    name.span(),
                                    "Node \"{name}\" conflicts with Trivia expression.",
                                ));
                            }
                        }

                        view.insert(from, lit, through)?;
                    }

                    let concurrency = *concurrency
                        .entry(from)
                        .and_modify(|count| *count += 1)
                        .or_insert(1);

                    if concurrency > RULE_LIMIT {
                        return Err(error!(
                            name.span(),
                            "Too many concurrent rules in this parse \
                            position.\nRule concurrency limit is {RULE_LIMIT}.",
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn delimiter(&self) -> Option<&TokenLit> {
        let mut delimiter = None;

        for (from, through, to) in self.transitions() {
            if !self.finish().contains(&to) {
                continue;
            }

            if self.start() == from {
                return None;
            }

            if self.transitions().view().contains_key(&to) {
                return None;
            }

            delimiter = match through {
                Terminal::Null => null!(),

                Terminal::Token(None, lit) => match delimiter {
                    None => Some(lit),
                    Some(previous) if previous == lit => continue,
                    _ => return None,
                },

                _ => return None,
            }
        }

        let delimiter = delimiter?;

        if delimiter.is_other() {
            return None;
        }

        Some(delimiter)
    }
}

pub(super) trait NodeAutomataImpl {
    fn merge_captures(&mut self, scope: &mut Scope) -> Result<()>;
    fn check_conflicts(&self, trivia: Option<&Rule>, map: &VariantMap) -> Result<()>;
    fn delimiter(&self) -> Option<&TokenLit>;
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub(super) enum Terminal {
    Null,
    Token(Option<Ident>, TokenLit),
    Node(Option<Ident>, Ident),
}

impl AutomataTerminal for Terminal {
    #[inline(always)]
    fn null() -> Self {
        Self::Null
    }

    #[inline(always)]
    fn is_null(&self) -> bool {
        match self {
            Self::Null => true,
            _ => false,
        }
    }
}

impl Ord for Terminal {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        use Ordering::*;

        match self.order().cmp(&other.order()) {
            Less => Less,
            Greater => Greater,
            Equal => match self.string().cmp(&other.string()) {
                Less => Less,
                Greater => Greater,
                Equal => self.capture().cmp(&other.capture()),
            },
        }
    }
}

impl PartialOrd for Terminal {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Terminal {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => formatter.write_str("null"),
            Self::Token(None, lit) => Display::fmt(lit, formatter),
            Self::Token(Some(target), lit) => formatter.write_fmt(format_args!("{target}: {lit}")),
            Self::Node(None, name) => Display::fmt(name, formatter),
            Self::Node(Some(target), name) => formatter.write_fmt(format_args!("{target}: {name}")),
        }
    }
}

impl Terminal {
    #[inline(always)]
    pub(super) fn span(&self) -> Span {
        match self {
            Terminal::Null => system_panic!("Getting span of Null Terminal."),
            Terminal::Token(_, lit) => lit.span(),
            Terminal::Node(_, name) => name.span(),
        }
    }

    #[inline(always)]
    pub(super) fn capture(&self) -> Option<&Ident> {
        match self {
            Self::Null => None,
            Self::Token(capture, _) => capture.as_ref(),
            Self::Node(capture, _) => capture.as_ref(),
        }
    }

    #[inline(always)]
    fn order(&self) -> u8 {
        match self {
            Self::Null => 0,
            Self::Token(capture, _) => 1 + (capture.is_some() as u8),
            Self::Node(capture, _) => 3 + (capture.is_some() as u8),
        }
    }

    #[inline(always)]
    fn string(&self) -> Option<String> {
        match self {
            Self::Null => None,
            Self::Token(_, lit) => Some(lit.to_string()),
            Self::Node(_, ident) => Some(ident.to_string()),
        }
    }
}

pub(super) struct Scope {
    state: State,
    strategy: Strategy,
}

impl Default for Scope {
    #[inline(always)]
    fn default() -> Self {
        Self {
            state: 1,
            strategy: Strategy::CANONICALIZE,
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
    fn strategy(&self) -> Strategy {
        self.strategy
    }
}

impl Scope {
    #[inline(always)]
    pub fn set_strategy(&mut self, strategy: Strategy) {
        self.strategy = strategy;
    }
}
