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

use std::{collections::VecDeque, ops::RangeFrom};

use proc_macro2::{Ident, TokenStream};

use crate::utils::{debug_panic, State};
use crate::{
    node::{
        automata::{variables::VariableMap, NodeAutomata},
        builder::kind::VariantKind,
        compiler::{
            delimiters::{PanicDelimiters, SynchronizationAction},
            inserts::InsertRecovery,
            transitions::{TransitionsVector, TransitionsVectorImpl},
            Compiler,
        },
        regex::terminal::Terminal,
    },
    utils::{Map, PredictableCollection, Set, SetImpl},
};

pub(in crate::node) struct Function<'a, 'b> {
    exclude_skips: bool,
    synchronization_context: bool,
    context_name: &'a str,
    compiler: &'a Compiler<'b>,
    automata: &'a NodeAutomata,
    delimiters: PanicDelimiters<'a>,
    variables: &'a VariableMap,
    pending: VecDeque<&'a State>,
    visited: Set<&'a State>,
    transitions: Vec<TokenStream>,
    state_map: Map<&'a State, usize>,
    state_generator: RangeFrom<usize>,
}

impl<'a, 'b> Function<'a, 'b> {
    pub(in crate::node) fn compile_skip_function(
        compiler: &'a mut Compiler<'b>,
    ) -> Option<TokenStream> {
        let core = compiler.facade().core_crate();
        let unreachable = compiler.facade().unreachable();

        let node_type = compiler.node_type();
        let code_lifetime = compiler.generics().code_lifetime();
        let outer_lifetime = compiler.generics().outer_lifetime();
        let function_impl_generics = compiler.generics().function_impl_generics();
        let function_where_clause = compiler.generics().function_where_clause();

        let variables = VariableMap::default();
        let delimiters = PanicDelimiters::default();

        let transitions = {
            let automata = match compiler.builder().skip_automata() {
                None => return None,
                Some(automata) => automata,
            };

            let mut function = Function {
                exclude_skips: false,
                synchronization_context: true,
                context_name: "Skip",
                compiler,
                automata,
                delimiters,
                variables: &variables,
                pending: VecDeque::from([automata.start()]),
                visited: Set::new([automata.start()]),
                transitions: Vec::with_capacity(automata.transitions().length() * 3),
                state_map: Map::empty(),
                state_generator: 1..,
            };

            loop {
                if let Some(state) = function.pending.pop_front() {
                    function.handle_state(state);

                    continue;
                }

                break;
            }

            function.transitions
        };

        Some(quote! {
            #[inline]
            #[allow(unused_mut)]
            #[allow(unused_labels)]
            #[allow(unused_assignments)]
            #[allow(unused_variables)]
            fn skip #function_impl_generics(
                session: &mut impl #core::syntax::SyntaxSession<
                    #code_lifetime,
                    Node = #node_type,
                >,
            )
            #function_where_clause
            {
                let mut state = 1usize;
                let mut start = #core::lexis::TokenCursor::site_ref(session, 0);

                #outer_lifetime: loop {
                    match (state, #core::lexis::TokenCursor::token(session, 0)) {
                        #( #transitions )*

                        _ => #unreachable("Unknown state."),
                    }
                }
            }
        })
    }

    pub(in crate::node) fn compile_variant_function(
        compiler: &'a mut Compiler<'b>,
        variant_name: &'a Ident,
    ) {
        let core = compiler.facade().core_crate();
        let option = compiler.facade().option();
        let convert = compiler.facade().convert();
        let unreachable = compiler.facade().unreachable();

        let variant = compiler.builder().variant(variant_name);

        match variant.kind() {
            VariantKind::Unspecified(..) => return,
            _ => (),
        }

        let kind = compiler.kind_of(variant_name);
        let function_name = compiler.function_of(variant_name);
        let node_type = compiler.node_type();
        let code_lifetime = compiler.generics().code_lifetime();
        let outer_lifetime = compiler.generics().outer_lifetime();
        let function_impl_generics = compiler.generics().function_impl_generics();
        let function_where_clause = compiler.generics().function_where_clause();
        let variables = variant.variables();

        let init_variables = variables.init(compiler);
        let result = variant.constructor().compile(compiler, variant_name);

        let context_name = variant_name.to_string();

        let transitions = {
            let exclude_skips = match variant.kind() {
                VariantKind::Comment(..) => false,
                _ => compiler.builder.skip_automata().is_some(),
            };

            let synchronization_context = variant.is_global_synchronization();

            let automata = variant.automata();

            let delimiters = PanicDelimiters::new(variant, compiler.builder());

            let mut function = Function {
                exclude_skips,
                synchronization_context,
                context_name: &context_name,
                compiler,
                automata,
                delimiters,
                variables,
                pending: VecDeque::from([automata.start()]),
                visited: Set::empty(),
                transitions: Vec::with_capacity(automata.transitions().length() * 3),
                state_map: Map::empty(),
                state_generator: 1..,
            };

            loop {
                if let Some(state) = function.pending.pop_front() {
                    function.handle_state(state);

                    continue;
                }

                break;
            }

            function.transitions
        };

        let end_of_input_check = match variant.kind() {
            VariantKind::Root(..) => {
                let error_type = compiler.builder().error_type();

                Some(quote! {
                    if let #option::Some(_) = #core::lexis::TokenCursor::token(session, 0) {
                        start = #core::lexis::TokenCursor::site_ref(session, 0);
                        let end = #core::lexis::TokenCursor::end_site_ref(session);

                        let _ = #core::syntax::SyntaxSession::error(
                            session,
                            <#error_type as #convert::<#core::syntax::SyntaxError>>::from(
                                #core::syntax::SyntaxError::UnexpectedEndOfInput {
                                    span: start..end,
                                    context: #context_name,
                                }
                            ),
                        );
                    }
                })
            }

            _ => None,
        };

        let body = quote! {
            #[allow(non_snake_case)]
            #[allow(unused_mut)]
            #[allow(unused_labels)]
            #[allow(unused_assignments)]
            #[allow(unused_variables)]
            fn #function_name #function_impl_generics(
                session: &mut impl #core::syntax::SyntaxSession<
                    #code_lifetime,
                    Node = #node_type,
                >,
            ) -> #node_type
            #function_where_clause
            {
                let mut state = 1usize;
                let mut start = #core::lexis::TokenCursor::site_ref(session, 0);

                #init_variables

                #outer_lifetime: loop {
                    match (state, #core::lexis::TokenCursor::token(session, 0)) {
                        #( #transitions )*

                        _ => #unreachable("Unknown state."),
                    }
                }

                #end_of_input_check

                #result
            }
        };

        compiler.add_function(kind, body);
    }

    fn handle_state(&mut self, state: &'a State) {
        let core = self.compiler.facade().core_crate();
        let option = self.compiler.facade().option();

        let from_name = self.name_of(state);
        let token_type = self.compiler.builder().token_type();
        let mut outgoing = TransitionsVector::outgoing(self.automata, state);

        for (_, through, to) in &outgoing {
            let is_final = !self.has_outgoing(to);
            let is_looping = &state == to;

            let (finalize, set_state) = match (is_final, is_looping) {
                (true, _) => (Some(quote! { break; }), None),

                (false, true) => (None, None),

                (false, false) => {
                    let to_name = self.name_of(to);

                    (None, Some(quote! { state = #to_name; }))
                }
            };

            let set_start = match is_final || through.is_skip(self.compiler.builder()) {
                true => None,

                false => Some(quote! {
                    start = #core::lexis::TokenCursor::site_ref(session, 0);
                }),
            };

            match through {
                Terminal::Null => debug_panic!("Automata with null transition."),

                Terminal::Token { name, capture } => {
                    let write = match capture {
                        None => None,
                        Some(name) => Some(self.variables.get(name).write(
                            self.compiler.facade(),
                            quote! {
                                #core::lexis::TokenCursor::token_ref(session, 0)
                            },
                        )),
                    };

                    self.transitions.push(quote! {
                        (#from_name, #option::Some(#token_type::#name { .. })) => {
                            #set_state
                            #write
                            let _ = #core::lexis::TokenCursor::advance(session);
                            #set_start
                            #finalize
                        }
                    });
                }

                Terminal::Node { name, capture } => {
                    let kind = self.compiler.kind_of(name);

                    let descend = match capture {
                        None => {
                            quote! {
                                let _ = #core::syntax::SyntaxSession::descend(session, &#kind);
                            }
                        }

                        Some(name) => self.variables.get(name).write(
                            self.compiler.facade(),
                            quote! {
                                #core::syntax::SyntaxSession::descend(session, &#kind)
                            },
                        ),
                    };

                    let leftmost = self.compiler.builder().variant(name).leftmost();

                    for token in leftmost.tokens() {
                        self.transitions.push(quote! {
                            (#from_name, #option::Some(#token_type::#token { .. })) => {
                                #set_state
                                #descend
                                #set_start
                                #finalize
                            }
                        });
                    }
                }
            }

            if !self.visited.contains(to) {
                let _ = self.visited.insert(to);

                if !is_final {
                    self.pending.push_back(to);
                }
            }
        }

        match self.automata.finish().contains(state) {
            true => {
                self.transitions.push(quote! {
                    (#from_name, _) => {
                        break;
                    }
                });
            }

            false => {
                if self.exclude_skips {
                    outgoing = outgoing.filter_skip(self.compiler.builder());
                }

                self.insert_recover(state, &outgoing);
                self.panic_recovery(state, &outgoing);
            }
        }
    }

    fn insert_recover(&mut self, state: &'a State, outgoing: &TransitionsVector<'a>) {
        let core = self.compiler.facade().core_crate();
        let option = self.compiler.facade().option();
        let convert = self.compiler.facade().convert();

        let token_type = self.compiler.builder().token_type();
        let error_type = self.compiler.builder().error_type();
        let context_name = self.context_name;

        let from_name = self.name_of(state);

        let recovery = InsertRecovery::prepare(self.compiler.builder(), self.automata, &outgoing);

        for insert in recovery {
            let error = match insert.expected_terminal() {
                Terminal::Null => debug_panic!("Automata with null transition."),

                Terminal::Token { name, .. } => {
                    let token = name.to_string();

                    quote! {
                        let _ = #core::syntax::SyntaxSession::error(
                            session,
                            <#error_type as #convert::<#core::syntax::SyntaxError>>::from(
                                #core::syntax::SyntaxError::MissingToken {
                                    span: start..start,
                                    context: #context_name,
                                    token: #token,
                                }
                            ),
                        );
                    }
                }

                Terminal::Node { name, .. } => {
                    let rule = name.to_string();

                    quote! {
                        let _ = #core::syntax::SyntaxSession::error(
                            session,
                            <#error_type as #convert::<#core::syntax::SyntaxError>>::from(
                                #core::syntax::SyntaxError::MissingRule {
                                    span: start..start,
                                    context: #context_name,
                                    rule: #rule,
                                }
                            ),
                        );
                    }
                }
            };

            let is_final = !self.has_outgoing(insert.destination_state());
            let is_looping = state == insert.destination_state();

            let (finalize, set_state) = match (is_final, is_looping) {
                (true, _) => (Some(quote! { break; }), None),

                (false, true) => (None, None),

                (false, false) => {
                    let destination_name = self.name_of(insert.destination_state());

                    (None, Some(quote! { state = #destination_name; }))
                }
            };

            let set_start = match is_final
                || insert
                    .destination_terminal()
                    .is_skip(self.compiler.builder())
            {
                true => None,

                false => Some(quote! {
                    start = #core::lexis::TokenCursor::site_ref(session, 0);
                }),
            };

            let insertion = match insert.expected_terminal().capture() {
                None => None,

                Some(capture) => self
                    .variables
                    .get(capture)
                    .insert(self.compiler.facade(), insert.expected_terminal()),
            };

            let reading = match insert.destination_terminal() {
                Terminal::Null => debug_panic!("Automata with null transition."),

                Terminal::Token { capture, .. } => {
                    let write = match capture {
                        None => None,
                        Some(name) => Some(self.variables.get(name).write(
                            self.compiler.facade(),
                            quote! {
                                #core::lexis::TokenCursor::token_ref(session, 0)
                            },
                        )),
                    };

                    quote! {
                        #write
                        let _ = #core::lexis::TokenCursor::advance(session);
                    }
                }

                Terminal::Node { name, capture } => {
                    let kind = self.compiler.kind_of(name);

                    match capture {
                        None => {
                            quote! {
                                let _ = #core::syntax::SyntaxSession::descend(session, &#kind);
                            }
                        }

                        Some(name) => self.variables.get(name).write(
                            self.compiler.facade(),
                            quote! {
                                #core::syntax::SyntaxSession::descend(session, &#kind)
                            },
                        ),
                    }
                }
            };

            let matching = insert.matching();

            self.transitions.push(quote! {
                (#from_name, #option::Some(#token_type::#matching { .. })) => {
                    #error
                    #insertion
                    #set_state
                    #reading
                    #set_start
                    #finalize
                }
            });
        }
    }

    fn panic_recovery(&mut self, state: &'a State, outgoing: &TransitionsVector<'a>) {
        let core = self.compiler.facade().core_crate();
        let option = self.compiler.facade().option();
        let vec = self.compiler.facade().vec();
        let convert = self.compiler.facade().convert();

        let token_type = self.compiler.builder().token_type();
        let error_type = self.compiler.builder().error_type();
        let outer_lifetime = self.compiler.generics().outer_lifetime();
        let context_name = self.context_name;

        let from_name = self.name_of(state);

        let error = {
            let (expected_tokens, expected_rules) = outgoing.split_terminals();

            let expected_tokens_len = expected_tokens.len();
            let expected_rules_len = expected_rules.len();

            quote! {
                let _ = #core::syntax::SyntaxSession::error(
                    session,
                    <#error_type as #convert::<#core::syntax::SyntaxError>>::from(
                        #core::syntax::SyntaxError::Mismatch {
                            span: start..end,
                            context: #context_name,
                            expected_tokens: <#vec<&'static str> as #convert::<[&'static str; #expected_tokens_len]>>::from(
                                [#( #expected_tokens ),*]
                            ),
                            expected_rules: <#vec<&'static str> as #convert::<[&'static str; #expected_rules_len]>>::from(
                                [#( #expected_rules ),*]
                            ),
                        }
                    ),
                );
            }
        };

        let mut panic_transitions =
            Vec::with_capacity(outgoing.len() + self.delimiters.global().len() + 2);

        for (_, through, _) in outgoing {
            match through {
                Terminal::Null => debug_panic!("Automata with null transition."),

                Terminal::Token { name, .. } => {
                    panic_transitions.push(self.handle_panic_expected(
                        &self.delimiters,
                        &error,
                        name,
                    ));
                }

                Terminal::Node { name, .. } => {
                    let leftmost = self.compiler.builder().variant(name).leftmost();

                    for token in leftmost.tokens() {
                        panic_transitions.push(self.handle_panic_expected(
                            &self.delimiters,
                            &error,
                            token,
                        ));
                    }
                }
            }
        }

        match self.delimiters.single() {
            None => (),

            Some(delimiter) => match self.delimiters.global().is_empty() {
                true => panic_transitions.push(quote! {
                    #option::Some(#token_type::#delimiter { .. }) => {
                        let _ = #core::lexis::TokenCursor::advance(session);
                        end = #core::lexis::TokenCursor::site_ref(session, 0);
                        #error
                        break #outer_lifetime;
                    }
                }),

                false => panic_transitions.push(quote! {
                    #option::Some(#token_type::#delimiter { .. }) => {
                        match #vec::is_empty(&synchronization_stack) {
                            false => {
                                let _ = #core::lexis::TokenCursor::advance(session);
                                end = #core::lexis::TokenCursor::site_ref(session, 0);
                            }

                            true => {
                                let _ = #core::lexis::TokenCursor::advance(session);
                                end = #core::lexis::TokenCursor::site_ref(session, 0);
                                #error
                                break #outer_lifetime;
                            }
                        }
                    }
                }),
            },
        }

        for (token, action) in self.delimiters.global() {
            match action {
                SynchronizationAction::Push { state, outer } if *outer => {
                    panic_transitions.push(quote! {
                        #option::Some(#token_type::#token { .. }) => {
                            #vec::push(&mut synchronization_stack, #state);
                            let _ = #core::lexis::TokenCursor::advance(session);
                            end = #core::lexis::TokenCursor::site_ref(session, 0);
                        }
                    });
                }

                SynchronizationAction::Pop { state, outer } if *outer => {
                    let synchronization =
                        match (self.synchronization_context, self.delimiters.local()) {
                            (true, Some((open, close))) if close != *token => {
                                quote! {
                                    let mut balance = 1usize;

                                    for lookahead in 1usize.. {
                                        match #core::lexis::TokenCursor::token(session, lookahead) {
                                            #option::Some(#token_type::#open { .. }) => {
                                                balance += 1usize;
                                            }

                                            #option::Some(#token_type::#close { .. }) => {
                                                balance -= 1usize;

                                                if balance == 0usize {
                                                    break;
                                                }
                                            }

                                            #option::Some(_) => (),

                                            #option::None => break,
                                        }
                                    }

                                    match balance {
                                        0usize => {
                                            let _ = #core::lexis::TokenCursor::advance(session);
                                            end = #core::lexis::TokenCursor::site_ref(session, 0);

                                            break;
                                        }
                                        _ => {
                                            #error
                                            break #outer_lifetime;
                                        }
                                    }
                                }
                            }

                            (true, _) => {
                                quote! {
                                    let _ = #core::lexis::TokenCursor::advance(session);
                                    end = #core::lexis::TokenCursor::site_ref(session, 0);

                                    break;
                                }
                            }

                            (false, _) => quote! {
                                #error
                                break #outer_lifetime;
                            },
                        };

                    panic_transitions.push(quote! {
                        #option::Some(#token_type::#token { .. }) => {
                            loop {
                                match synchronization_stack.pop() {
                                    #option::None => {
                                        #synchronization
                                    },

                                    #option::Some(top) => {
                                        if top != #state {
                                            continue;
                                        }
                                    },
                                }
                            }
                        }
                    });
                }

                _ => (),
            }
        }

        panic_transitions.push(quote! {
            #option::Some(_) => {
                let _ = #core::lexis::TokenCursor::advance(session);
                end = #core::lexis::TokenCursor::site_ref(session, 0);
            }
        });

        panic_transitions.push(quote! {
            #option::None => {
                #error
                break #outer_lifetime;
            }
        });

        let init_synchronization = match self.delimiters.global().is_empty() {
            true => None,

            false => Some(quote! {
                let mut synchronization_stack = #vec::<usize>::new();
            }),
        };

        let skip = match self.exclude_skips {
            false => None,

            true => Some(quote! {
                skip(session);
            }),
        };

        self.transitions.push(quote! {
            (#from_name, _) => {
                start = #core::lexis::TokenCursor::site_ref(session, 0);
                let mut end = start;

                #init_synchronization

                loop {
                    match #core::lexis::TokenCursor::token(session, 0) {
                        #( #panic_transitions )*
                    }

                    #skip
                }
            }
        });
    }

    fn handle_panic_expected(
        &self,
        delimiters: &PanicDelimiters,
        error: &TokenStream,
        expected: &Ident,
    ) -> TokenStream {
        let core = self.compiler.facade().core_crate();
        let option = self.compiler.facade().option();
        let vec = self.compiler.facade().vec();

        let token_type = self.compiler.builder().token_type();

        match delimiters.global().get(expected) {
            None => {
                quote! {
                    #option::Some(#token_type::#expected { .. }) => {
                        #error
                        break;
                    }
                }
            }

            Some(SynchronizationAction::Push { state, .. }) => {
                quote! {
                    #option::Some(#token_type::#expected { .. }) => {
                        match #vec::is_empty(&synchronization_stack) {
                            false => {
                                #vec::push(&mut synchronization_stack, #state);
                                let _ = #core::lexis::TokenCursor::advance(session);
                                end = #core::lexis::TokenCursor::site_ref(session, 0);
                            },

                            true => {
                                #error
                                break;
                            }
                        }
                    }
                }
            }

            Some(SynchronizationAction::Pop { state, .. }) => {
                quote! {
                    #option::Some(#token_type::#expected { .. }) => {
                        match #vec::pop(&mut synchronization_stack) {
                            #option::Some(top) if top == #state => {
                                let _ = #core::lexis::TokenCursor::advance(session);
                                end = #core::lexis::TokenCursor::site_ref(session, 0);
                            },

                            _ => {
                                #error
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    #[inline]
    fn has_outgoing(&self, state: &State) -> bool {
        for (from, _, _) in self.automata.transitions() {
            if from == state {
                return true;
            }
        }

        return false;
    }

    #[inline(always)]
    fn name_of(&mut self, state: &'a State) -> usize {
        *self.state_map.entry(state).or_insert_with(|| {
            self.state_generator
                .next()
                .expect("Internal error. State generator exceeded.")
        })
    }
}
