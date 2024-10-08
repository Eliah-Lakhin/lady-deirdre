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

use std::collections::BTreeMap;

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{parse::ParseStream, spanned::Spanned, Attribute, Error, LitStr, Result};

use crate::{
    node::{
        automata::{NodeAutomata, NodeAutomataImpl, Scope, Terminal},
        globals::{GlobalVar, Globals},
        index::Index,
        input::NodeInput,
        leftmost::Leftmost,
        regex::{Operand, Operator, Regex, RegexImpl},
        token::TokenLit,
        variables::VariableMap,
    },
    utils::{
        expect_some,
        null,
        system_panic,
        AutomataContext,
        Facade,
        Map,
        PredictableCollection,
        Set,
        SetImpl,
        State,
    },
};

pub(super) struct Rule {
    pub(super) span: Span,
    pub(super) regex: Regex,
    pub(super) leftmost: Option<Leftmost>,
    pub(super) automata: Option<NodeAutomata>,
    pub(super) variables: Option<VariableMap>,
}

impl TryFrom<Attribute> for Rule {
    type Error = Error;

    #[inline(always)]
    fn try_from(attr: Attribute) -> Result<Self> {
        let span = attr.span();

        attr.parse_args_with(|input: ParseStream| {
            let regex = input.parse::<Regex>()?;

            Ok(Self {
                span,
                regex,
                leftmost: None,
                automata: None,
                variables: None,
            })
        })
    }
}

impl Rule {
    #[inline]
    pub(super) fn zero_or_more(mut self) -> Self {
        self.regex = Regex::Unary(Operator::ZeroOrMore(None), Box::new(self.regex));

        self
    }

    #[inline]
    pub(super) fn greedy(mut self) -> Self {
        self.regex = Regex::Binary(
            Box::new(self.regex),
            Operator::Concat,
            Box::new(Regex::Operand(Operand::Token(
                None,
                TokenLit::EOI(self.span),
            ))),
        );

        self
    }

    #[inline]
    pub(super) fn encode(&mut self, scope: &mut Scope) -> Result<()> {
        self.leftmost = Some(Leftmost::from(&self.regex));

        let mut automata = self.regex.encode(scope)?;
        automata.merge_captures(scope)?;

        let variables = VariableMap::try_from(&automata)?;

        self.automata = Some(automata);
        self.variables = Some(variables);

        Ok(())
    }

    pub(super) fn compile(
        &self,
        input: &NodeInput,
        globals: &mut Globals,
        context: &Index,
        recovery_var: &GlobalVar,
        with_trivia: bool,
        surround_trivia: bool,
        output_comments: bool,
    ) -> TokenStream {
        let automata = expect_some!(self.automata.as_ref(), "Missing automata.",);
        let variables = expect_some!(self.variables.as_ref(), "Missing variable map.",);

        let delimiter = automata.delimiter();

        let automata = {
            let mut scope = Scope::default();
            scope.copy(automata)
        };

        let span = self.span;
        let core = span.face_core();
        let unreachable = span.face_unreachable();

        let start = automata.start();

        let init_vars = variables.init();
        let init_first;
        let init_step;

        match with_trivia {
            false => {
                init_first = None;
                init_step = quote_spanned!(span=>
                    let step_start_ref = #core::lexis::TokenCursor::site_ref(session, 0);
                );
            }

            true => match surround_trivia {
                true => {
                    init_first = None;
                    init_step = quote_spanned!(span=>
                        let step_start_ref = #core::lexis::TokenCursor::site_ref(session, 0);
                        skip_trivia(session);
                    );
                }

                false => {
                    init_first = Some(quote_spanned!(span=> let mut first = true;));
                    init_step = quote_spanned!(span=>
                        let step_start_ref = #core::lexis::TokenCursor::site_ref(session, 0);

                        match first {
                            true => first = false,
                            false => skip_trivia(session),
                        }
                    );
                }
            },
        }

        let mut transitions = automata
            .transitions()
            .view()
            .keys()
            .map(|from| {
                let handler = self.compile_outgoing(
                    input,
                    globals,
                    context,
                    &automata,
                    &variables,
                    delimiter,
                    recovery_var,
                    output_comments,
                    *from,
                );

                (
                    *from,
                    quote_spanned!(span=> #from => {
                        #handler
                    }),
                )
            })
            .collect::<Vec<_>>();

        transitions.sort_by_key(|(key, _)| *key);

        let transitions = transitions.into_iter().map(|(_, stream)| stream);

        quote_spanned!(span=>
            let mut state = #start;
            #init_first
            #init_vars

            loop {
                #init_step

                match state {
                    #(
                        #transitions
                    )*
                    other => #unreachable("Unknown state {other}."),
                }
            }
        )
    }

    fn compile_outgoing(
        &self,
        input: &NodeInput,
        globals: &mut Globals,
        context: &Index,
        automata: &NodeAutomata,
        variables: &VariableMap,
        delimiter: Option<&TokenLit>,
        recovery_var: &GlobalVar,
        output_comments: bool,
        from: State,
    ) -> TokenStream {
        let outgoing = expect_some!(
            automata.transitions().outgoing(&from),
            "Empty state transitions.",
        );

        let mut stream = TokenStream::new();

        let halts = automata.finish().contains(&from);

        let span = self.span;
        let core = span.face_core();

        let total_alphabet_len = input.alphabet.len() + 2;
        let mut covered = Set::with_capacity(total_alphabet_len);

        let mut expected_tokens = Set::with_capacity(input.alphabet.len());
        let mut expected_nodes = Set::with_capacity(input.variants.len());
        let mut by_token = Map::with_capacity(input.alphabet.len());

        for (through, to) in outgoing {
            match through {
                Terminal::Null => null!(),

                Terminal::Token(capture, lit) => {
                    if !covered.insert(lit.clone()) {
                        system_panic!("Duplicate covered token.",);
                    }

                    let transition = match lit.is_eoi() {
                        true => None,
                        false => Some(*to),
                    };

                    let previous = by_token.insert(
                        lit.clone(),
                        Action {
                            transition,
                            capture: capture.clone(),
                            descend: None,
                            insert: None,
                        },
                    );

                    if previous.is_some() {
                        system_panic!("Duplicate by_token entry.")
                    }

                    if let TokenLit::Ident(ident) = lit {
                        let _ = expected_tokens.insert(ident.clone());
                    }
                }

                Terminal::Node(capture, ident) => {
                    let variant = expect_some!(input.variants.get(ident), "Unresolved reference.",);
                    let rule =
                        expect_some!(variant.rule.as_ref(), "Reference to unparseable variant .",);
                    let leftmost =
                        expect_some!(rule.leftmost.as_ref(), "Missing leftmost of rule.",);
                    let matches = expect_some!(leftmost.matches(), "Unresolved leftmost matches.",);

                    for lit in matches {
                        if !covered.insert(lit.clone()) {
                            system_panic!("Duplicate covered token.");
                        }

                        let transition = match lit.is_eoi() {
                            true => None,
                            false => Some(*to),
                        };

                        let previous = by_token.insert(
                            lit.clone(),
                            Action {
                                transition,
                                capture: capture.clone(),
                                descend: Some(ident.clone()),
                                insert: None,
                            },
                        );

                        if previous.is_some() {
                            system_panic!("Duplicate by_token entry.")
                        }
                    }

                    let index =
                        expect_some!(variant.index.as_ref(), "Missing parsable variant index.",);

                    expected_nodes.insert(index);
                }
            }
        }

        let expected_tokens_var;
        let expected_nodes_var;

        match halts {
            true => {
                expected_tokens_var = GlobalVar::EmptyTokenSet.compile(span);
                expected_nodes_var = GlobalVar::EmptyNodeSet.compile(span);

                let eoi = TokenLit::EOI(span);

                if !by_token.contains_key(&eoi) {
                    let _ = by_token.insert(
                        eoi,
                        Action {
                            transition: None,
                            insert: None,
                            descend: None,
                            capture: None,
                        },
                    );
                }
            }

            false => {
                expected_tokens_var = globals
                    .inclusive_tokens(
                        expected_tokens
                            .into_iter()
                            .map(|ident| TokenLit::Ident(ident)),
                    )
                    .compile(span);

                expected_nodes_var = globals
                    .rules(expected_nodes.into_iter().cloned())
                    .compile(span);
            }
        };

        let mut insert_map = Map::with_capacity(total_alphabet_len - covered.len());

        'outer: for (insert, to) in outgoing {
            if let Terminal::Token(_, TokenLit::Other(..)) = insert {
                insert_map.clear();
                break 'outer;
            }

            if let Some(outgoing) = automata.transitions().outgoing(&to) {
                for (through, to) in outgoing {
                    match through {
                        Terminal::Null => null!(),

                        Terminal::Token(capture, lit) => {
                            if by_token.contains_key(lit) {
                                insert_map.clear();
                                break 'outer;
                            }

                            let transition = match lit.is_eoi() {
                                true => None,
                                false => Some(*to),
                            };

                            let previous = insert_map.insert(
                                lit.clone(),
                                Action {
                                    transition,
                                    capture: capture.clone(),
                                    descend: None,
                                    insert: Some(insert.clone()),
                                },
                            );

                            if previous.is_some() {
                                insert_map.clear();
                                break 'outer;
                            }
                        }

                        Terminal::Node(capture, ident) => {
                            let variant =
                                expect_some!(input.variants.get(ident), "Unresolved reference.",);
                            let rule = expect_some!(
                                variant.rule.as_ref(),
                                "Reference to unparseable variant .",
                            );
                            let leftmost =
                                expect_some!(rule.leftmost.as_ref(), "Missing leftmost of rule.",);
                            let matches =
                                expect_some!(leftmost.matches(), "Unresolved leftmost matches.",);

                            for lit in matches {
                                if by_token.contains_key(lit) {
                                    insert_map.clear();
                                    break 'outer;
                                }

                                let transition = match lit.is_eoi() {
                                    true => None,
                                    false => Some(*to),
                                };

                                let previous = insert_map.insert(
                                    lit.clone(),
                                    Action {
                                        transition,
                                        capture: capture.clone(),
                                        descend: Some(ident.clone()),
                                        insert: Some(insert.clone()),
                                    },
                                );

                                if previous.is_some() {
                                    insert_map.clear();
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }
        }

        for (lit, action) in insert_map {
            let previous = by_token.insert(lit, action);

            if previous.is_some() {
                system_panic!("Duplicate by_token entry.")
            }
        }

        let mut by_action = BTreeMap::<Action, Set<TokenLit>>::new();

        for (lit, action) in by_token {
            by_action
                .entry(action)
                .and_modify(|set| {
                    let _ = set.insert(lit.clone());
                })
                .or_insert_with(|| Set::new([lit]));
        }

        quote_spanned!(span=>
            let token = #core::lexis::TokenCursor::token(session, 0);
        )
        .to_tokens(&mut stream);

        for (action, set) in by_action {
            let mut body = TokenStream::new();

            if let Some(insert) = &action.insert {
                match insert {
                    Terminal::Null => null!(),

                    Terminal::Token(capture, lit) => {
                        if let Some(variable) = capture {
                            variables.get(variable).write_nil().to_tokens(&mut body);
                        }

                        let var = globals
                            .inclusive_tokens([lit.clone()].into_iter())
                            .compile(span);

                        quote_spanned!(span=>
                            let step_end_ref = #core::lexis::TokenCursor::site_ref(session, 0);

                            #core::syntax::SyntaxSession::failure(
                                session,
                                #core::syntax::SyntaxError {
                                    span: step_start_ref..step_end_ref,
                                    context: #context,
                                    recovery: #core::syntax::RecoveryResult::InsertRecover,
                                    expected_tokens: &#var,
                                    expected_nodes: &#core::syntax::EMPTY_NODE_SET,
                                },
                            );
                        )
                        .to_tokens(&mut body);
                    }

                    Terminal::Node(capture, ident) => {
                        if let Some(variable) = capture {
                            variables.get(variable).write_nil().to_tokens(&mut body);
                        }

                        let variant =
                            expect_some!(input.variants.get(ident), "Unresolved reference.",);

                        let index = expect_some!(
                            variant.index.as_ref(),
                            "Missing parsable variant index.",
                        );

                        let var = globals.rules([index.clone()].into_iter()).compile(span);

                        quote_spanned!(span=>
                            let step_end_ref = #core::lexis::TokenCursor::site_ref(session, 0);

                            #core::syntax::SyntaxSession::failure(
                                session,
                                #core::syntax::SyntaxError {
                                    span: step_start_ref..step_end_ref,
                                    context: #context,
                                    recovery: #core::syntax::RecoveryResult::InsertRecover,
                                    expected_tokens: &#core::lexis::EMPTY_TOKEN_SET,
                                    expected_nodes: &#var,
                                },
                            );
                        )
                        .to_tokens(&mut body);
                    }
                }
            }

            match action.descend {
                None => {
                    if let Some(variable) = action.capture {
                        variables
                            .get(&variable)
                            .write(quote_spanned!(span=>
                                #core::lexis::TokenCursor::token_ref(session, 0)))
                            .to_tokens(&mut body);
                    }

                    if set.single() != Some(TokenLit::EOI(span)) {
                        quote_spanned!(span=>
                            #core::lexis::TokenCursor::advance(session);
                        )
                        .to_tokens(&mut body);
                    }
                }

                Some(ident) => {
                    let variant =
                        expect_some!(input.variants.get(&ident), "Unresolved reference.",);

                    let index =
                        expect_some!(variant.index.as_ref(), "Missing parsable variant index.",);

                    let ident = variant.parser_fn_ident();

                    let index = match output_comments {
                        false => index.to_token_stream(),
                        true => {
                            let comment = LitStr::new(&format!(" {}", ident), ident.span());

                            quote_spanned!(span=> #[doc = #comment] #index)
                        }
                    };

                    let descend = match variant.secondary.is_some() {
                        false => quote_spanned!(span=> #core::syntax::SyntaxSession::descend(
                            session,
                            #index,
                        )),

                        true => quote_spanned!(span=> {
                            #core::syntax::SyntaxSession::enter(session, #index);
                            let node = #ident(session);
                            #core::syntax::SyntaxSession::leave(session, node)
                        }),
                    };

                    match action.capture {
                        None => quote_spanned!(span=> #descend;).to_tokens(&mut body),

                        Some(variable) => {
                            variables.get(&variable).write(descend).to_tokens(&mut body);
                        }
                    }
                }
            }

            match action.transition {
                None => quote_spanned!(span=> break;).to_tokens(&mut body),

                Some(to) => {
                    let has_outgoing = automata.transitions().outgoing(&to).is_some();
                    let is_final = automata.finish().contains(&to);
                    let is_looping = from == to;

                    if has_outgoing && !is_looping {
                        quote_spanned!(span=> state = #to;).to_tokens(&mut body);
                    }

                    match !has_outgoing && is_final {
                        true => quote_spanned!(span=> break;).to_tokens(&mut body),
                        false => quote_spanned!(span=> continue;).to_tokens(&mut body),
                    }
                }
            }

            match set.single() {
                None | Some(TokenLit::Other(..)) => {
                    let pattern = Self::make_pattern(input, globals, set).compile(span);

                    quote_spanned!(span=> if #core::lexis::TokenSet::contains(&#pattern, token as u8) {
                        #body
                    })
                        .to_tokens(&mut stream);
                }

                Some(lit) => {
                    let enum_variant =
                        expect_some!(lit.as_enum_variant(&input.token), "Missing enum variant.",);

                    quote_spanned!(span=> if token == #enum_variant {
                        #body
                    })
                    .to_tokens(&mut stream);
                }
            }
        }

        match halts {
            true => {
                quote_spanned!(span=> break;).to_tokens(&mut stream);
            }

            false if covered.len() < total_alphabet_len => {
                let recovery = recovery_var.compile(span);

                let delimiter_halt = match delimiter {
                    Some(delimiter) if !covered.contains(delimiter) => {
                        let _ = covered.insert(delimiter.clone());

                        Some(delimiter)
                    }

                    _ => None,
                };

                let expectations = Self::make_pattern(input, globals, covered).compile(span);

                quote_spanned!(span=>
                    let recovery = #core::syntax::Recovery::recover(
                        &#recovery,
                        session,
                        &#expectations,
                    );

                    let mut recovered = #core::syntax::RecoveryResult::recovered(
                        &recovery,
                    );

                    let step_end_ref = #core::lexis::TokenCursor::site_ref(session, 0);

                    #core::syntax::SyntaxSession::failure(
                        session,
                        #core::syntax::SyntaxError {
                            span: step_start_ref..step_end_ref,
                            context: #context,
                            recovery,
                            expected_tokens: &#expected_tokens_var,
                            expected_nodes: &#expected_nodes_var,
                        },
                    );
                )
                .to_tokens(&mut stream);

                if let Some(delimiter) = delimiter_halt {
                    let delimiter = expect_some!(
                        delimiter.as_enum_variant(&input.token),
                        "Non-ident delimiter.",
                    );

                    quote_spanned!(span=>
                        if recovered {
                            if #core::lexis::TokenCursor::token(session, 0) == #delimiter {
                                #core::lexis::TokenCursor::advance(session);
                                recovered = false;
                            }
                        }
                    )
                    .to_tokens(&mut stream);
                }

                quote_spanned!(span=>
                    if !recovered {
                        break;
                    }
                )
                .to_tokens(&mut stream);
            }

            _ => (),
        }

        stream
    }

    fn make_pattern(input: &NodeInput, globals: &mut Globals, mut set: Set<TokenLit>) -> GlobalVar {
        let mut exclusive = false;

        set.retain(|lit| match lit {
            TokenLit::Ident(..) | TokenLit::EOI(..) => true,
            TokenLit::Other(..) => {
                exclusive = true;
                false
            }
        });

        match exclusive {
            false => globals.inclusive_tokens(set.into_iter()),

            true => {
                let mut excluded = input.alphabet.clone();

                for lit in set {
                    let _ = excluded.remove(&lit);
                }

                globals.exclusive_tokens(excluded.into_iter())
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Action {
    transition: Option<State>,
    insert: Option<Terminal>,
    descend: Option<Ident>,
    capture: Option<Ident>,
}
