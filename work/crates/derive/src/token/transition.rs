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

use std::{cmp::Ordering, ops::RangeInclusive};

use proc_macro2::{Span, TokenStream};
use syn::LitChar;

use crate::{
    token::{
        rule::{RuleIndex, RuleMeta},
        scope::ScannerState,
        NULL,
    },
    utils::{Facade, Set},
};

#[derive(PartialEq, Eq, PartialOrd)]
pub(super) struct Transition {
    from: ScannerState,
    incoming: Group,
    peek: Group,
    to: Option<ScannerState>,
    product: Option<RuleIndex>,
}

impl Ord for Transition {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        if self.from < other.from {
            return Ordering::Less;
        }

        if self.from > other.from {
            return Ordering::Greater;
        }

        match self.incoming.cmp(&other.incoming) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => self.peek.cmp(&other.peek),
        }
    }
}

impl Transition {
    #[inline(always)]
    pub(super) fn new(
        from: ScannerState,
        incoming: Set<char>,
        peek: Set<char>,
        to: Option<ScannerState>,
        product: Option<RuleIndex>,
    ) -> Self {
        Self {
            from,
            incoming: incoming.into(),
            peek: peek.into(),
            to,
            product,
        }
    }

    pub(super) fn output(&self, facade: &Facade, rules: &mut Vec<RuleMeta>) -> TokenStream {
        let core = facade.core_crate();

        let from = &self.from;

        let pattern = match (self.incoming.is_placeholder(), self.peek.is_placeholder()) {
            (true, true) => {
                quote! {
                    (#from, current, _) if current != '\0'
                }
            }

            (false, true) => {
                let incoming = self.incoming.output();

                quote! {
                    (#from, #incoming, _)
                }
            }

            (true, false) => {
                let peek = self.peek.output();

                quote! {
                    (#from, #peek, _)
                }
            }

            (false, false) => {
                let incoming = self.incoming.output();
                let peek = self.peek.output();

                quote! {
                    (#from, #incoming, #peek)
                }
            }
        };

        let to = match (&self.to, &self.product) {
            (None, None) => unreachable!("Dead state."),

            (None, Some(index)) => {
                let in_place = rules[*index].output_in_place(facade);

                quote! {
                    {
                        #core::lexis::LexisSession::submit(session);
                        return #in_place;
                    }
                }
            }

            (Some(state), None) => {
                if state == from {
                    quote! { (), }
                } else {
                    quote! {
                        state = #state,
                    }
                }
            }

            (Some(state), Some(index)) => {
                let core = facade.core_crate();
                let derived = rules[*index].output_derive();

                if state == from {
                    quote! {
                        {
                            #core::lexis::LexisSession::submit(session);
                            #derived;
                        }
                    }
                } else {
                    quote! {
                        {
                            #core::lexis::LexisSession::submit(session);
                            #derived;
                            state = #state;
                        }
                    }
                }
            }
        };

        quote! {
            #pattern => #to
        }
    }
}

#[derive(PartialEq, Eq)]
enum Group {
    Placeholder,
    Subgroups {
        sequential: Vec<char>,
        grouped: Vec<Subgroup>,
    },
}

impl PartialOrd for Group {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Group {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Placeholder, Self::Placeholder) => Ordering::Equal,
            (Self::Placeholder, _) => Ordering::Greater,
            (_, Self::Placeholder) => Ordering::Less,
            (Self::Subgroups { sequential: a, .. }, Self::Subgroups { sequential: b, .. }) => {
                a.cmp(b)
            }
        }
    }
}

impl Group {
    #[inline(always)]
    fn is_placeholder(&self) -> bool {
        match self {
            Self::Placeholder => true,
            _ => false,
        }
    }

    fn output(&self) -> TokenStream {
        match self {
            Self::Placeholder => unreachable!("An attempt to output placeholder"),
            Self::Subgroups { grouped, .. } => {
                let grouped = grouped.iter().map(Subgroup::output);
                quote! { #( #grouped )|* }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Subgroup {
    Single(char),
    Range(RangeInclusive<char>),
}

impl From<Set<char>> for Group {
    fn from(set: Set<char>) -> Self {
        if set.contains(&NULL) {
            return Self::Placeholder;
        }

        let mut sequential = set.into_iter().collect::<Vec<_>>();

        sequential.sort();

        let grouped = sequential
            .iter()
            .fold(None, |accumulator, character| match accumulator {
                None => Some(vec![Subgroup::Single(*character)]),
                Some(mut grouped) => {
                    let last = grouped
                        .pop()
                        .expect("Internal error. Empty subgroup sequence.");

                    match last {
                        Subgroup::Single(single) => {
                            if single as u32 + 1 == *character as u32 {
                                grouped.push(Subgroup::Range(single..=*character))
                            } else {
                                grouped.push(Subgroup::Single(single));
                                grouped.push(Subgroup::Single(*character));
                            }
                        }

                        Subgroup::Range(range) => {
                            if *range.end() as u32 + 1 == *character as u32 {
                                grouped.push(Subgroup::Range(*range.start()..=*character))
                            } else {
                                grouped.push(Subgroup::Range(range));
                                grouped.push(Subgroup::Single(*character));
                            }
                        }
                    }

                    Some(grouped)
                }
            })
            .expect("Internal error. Empty character set.");

        Self::Subgroups {
            sequential,
            grouped,
        }
    }
}

impl Subgroup {
    fn output(&self) -> TokenStream {
        match self {
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
        }
    }
}
