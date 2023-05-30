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

use std::ops::RangeInclusive;

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::LitChar;

use crate::{
    token::{terminal::Terminal, Token, NULL},
    utils::{null, system_panic, Facade, Map, PredictableCollection, Set, SetImpl, State},
};

impl Token {
    pub(super) fn output(self, facade: &Facade) -> TokenStream {
        let core = facade.core_crate();
        let option = facade.option();

        let alphabet = self.scope.alphabet();

        let mut transitions = Vec::with_capacity(self.automata.transitions().length());

        for (from, outgoing) in self.automata.transitions().view() {
            let mut unmatched = alphabet.inner().clone();
            let mut other = None;
            let mut by_state = Map::<State, Set<char>>::empty();

            for (through, to) in outgoing {
                let through = match through {
                    Terminal::Null => null!(),
                    Terminal::Product(..) => system_panic!("Unfiltered production terminal."),
                    Terminal::Character(character) => character,
                };

                match through {
                    &NULL => other = Some(*to),
                    through => {
                        let _ = unmatched.remove(through);

                        by_state
                            .entry(*to)
                            .and_modify(|set| {
                                let _ = set.insert(*through);
                            })
                            .or_insert_with(|| Set::new([*through]));
                    }
                }
            }

            let mut by_state = by_state.into_iter().collect::<Vec<_>>();

            by_state.sort_by_key(|(to, _)| *to);

            let by_state = by_state.into_iter().map(|(to, through)| {
                let pattern = Self::char_pattern(through.into_iter());

                quote!(#pattern => {
                    state = #to;
                    continue;
                })
            });

            let other = match other {
                None => quote!(_ => break,),

                Some(to) => {
                    let _ = unmatched.insert(NULL);

                    let unmatched = Self::char_pattern(unmatched.into_iter());

                    quote! {
                        #unmatched => break,

                        _ => {
                            state = #to;
                            continue;
                        }
                    }
                }
            };

            let product = match self.products.get(from) {
                None => None,
                Some(rule) => {
                    let rule = *rule + 1;

                    Some(quote!(
                        #core::lexis::LexisSession::submit(session);
                        token = #rule;
                    ))
                }
            };

            let inner_transitions = match outgoing.is_empty() {
                true => None,
                false => Some(quote!(
                    let input = #core::lexis::LexisSession::character(session);
                    #core::lexis::LexisSession::advance(session);

                    match input {
                        #(
                        #by_state
                        )*
                        #other
                    }
                )),
            };

            transitions.push((
                from,
                quote!(
                    #from => {
                        #product
                        #inner_transitions
                    }
                ),
            ));
        }

        transitions.sort_by_key(|(from, _)| **from);

        let transitions = transitions.into_iter().map(|(_, transition)| transition);

        let start = self.automata.start();

        let token_name = self.token_name;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let (mismatch_ident, mismatch_description) = self.mismatch;

        let description = self
            .rules
            .iter()
            .map(|rule| {
                let ident = &rule.name;
                let description = &rule.description;

                quote! {
                    if Self::#ident as u8 == token {
                        return #option::Some(#description);
                    }
                }
            })
            .collect::<Vec<_>>();

        let rules = self.rules.into_iter().map(|rule| rule.output(facade));

        quote! {
            impl #impl_generics #core::lexis::Token for #token_name #ty_generics
            #where_clause
            {
                fn parse(session: &mut impl #core::lexis::LexisSession) -> Self {
                    #[allow(unused_mut)]
                    let mut state = #start;
                    #[allow(unused_mut)]
                    let mut token = 0usize;

                    loop {
                        match state {
                            #( #transitions )*
                            _ => (),
                        }

                        break;
                    }

                    match token {
                        #(
                        #rules
                        )*
                        _ => Self::#mismatch_ident
                    }
                }

                #[inline(always)]
                fn index(self) -> #core::lexis::TokenIndex {
                    self as u8
                }

                #[inline(always)]
                fn describe(token: #core::lexis::TokenIndex) -> #option<&'static str> {
                    #(#description)*

                    if Self::#mismatch_ident as u8 == token {
                        return #option::Some(#mismatch_description);
                    }

                    None
                }
            }
        }
    }

    fn char_pattern(source: impl Iterator<Item = char>) -> TokenStream {
        enum Group {
            Single(char),
            Range(RangeInclusive<char>),
        }

        impl ToTokens for Group {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                let append = match self {
                    Self::Single(character) => {
                        let literal = LitChar::new(*character, Span::call_site());

                        quote! {
                            #literal
                        }
                    }

                    Self::Range(range) => {
                        let start = LitChar::new(*range.start(), Span::call_site());
                        let end = LitChar::new(*range.end(), Span::call_site());

                        quote! {
                            #start..=#end
                        }
                    }
                };

                append.to_tokens(tokens)
            }
        }

        let mut sequential = source.collect::<Vec<_>>();

        sequential.sort();

        let grouped = sequential
            .iter()
            .fold(None, |accumulator, character| match accumulator {
                None => Some(vec![Group::Single(*character)]),
                Some(mut grouped) => {
                    let last = grouped
                        .pop()
                        .expect("Internal error. Empty subgroup sequence.");

                    match last {
                        Group::Single(single) => {
                            if single as u32 + 1 == *character as u32 {
                                grouped.push(Group::Range(single..=*character))
                            } else {
                                grouped.push(Group::Single(single));
                                grouped.push(Group::Single(*character));
                            }
                        }

                        Group::Range(range) => {
                            if *range.end() as u32 + 1 == *character as u32 {
                                grouped.push(Group::Range(*range.start()..=*character))
                            } else {
                                grouped.push(Group::Range(range));
                                grouped.push(Group::Single(*character));
                            }
                        }
                    }

                    Some(grouped)
                }
            })
            .expect("Internal error. Empty character set.");

        quote! { #( #grouped )|* }
    }
}
