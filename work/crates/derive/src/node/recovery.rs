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

use std::{cmp::Ordering, collections::BTreeSet};

use proc_macro2::{Span, TokenStream};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Bracket,
    Result,
    Type,
};

use crate::{
    node::token::TokenLit,
    utils::{error, expect_some, system_panic, Facade},
};

#[derive(Clone)]
pub(super) struct Recovery {
    span: Span,
    groups: BTreeSet<(TokenLit, TokenLit)>,
    unexpected: BTreeSet<TokenLit>,
}

impl PartialEq for Recovery {
    fn eq(&self, other: &Self) -> bool {
        if !self.groups.eq(&other.groups) {
            return false;
        }

        if self.unexpected.eq(&other.unexpected) {
            return false;
        }

        true
    }
}

impl Eq for Recovery {}

impl Ord for Recovery {
    fn cmp(&self, other: &Self) -> Ordering {
        let ordering = self.groups.cmp(&other.groups);

        if ordering != Ordering::Equal {
            return ordering;
        }

        self.unexpected.cmp(&other.unexpected)
    }
}

impl PartialOrd for Recovery {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Parse for Recovery {
    fn parse(input: ParseStream) -> Result<Self> {
        let span = input.span();
        let entries = Punctuated::<SpecEntry, Token![,]>::parse_terminated(input)?;

        let mut groups = BTreeSet::new();
        let mut unexpected = BTreeSet::new();

        for entry in entries {
            match entry {
                SpecEntry::Group(span, open, close) => {
                    let limit = Self::GROUPS_LIMIT;
                    if groups.len() >= limit {
                        return Err(error!(
                            span,
                            "Too many token groups.\nThe recovery interface \
                            supports up to {limit} token groups.",
                        ));
                    }

                    for (group_open, group_close) in &groups {
                        if &open == group_open || &open == group_close {
                            return Err(error!(
                                open.span(),
                                "This token already used in the \
                                [{group_open}..{group_close}] token group.",
                            ));
                        }

                        if &close == group_open || &close == group_close {
                            return Err(error!(
                                close.span(),
                                "This token already used in the \
                                [{group_open}..{group_close}] token group.",
                            ));
                        }
                    }

                    groups.insert((open, close));
                }

                SpecEntry::Unexpected(token) => {
                    for previous in &unexpected {
                        if &token == previous {
                            return Err(error!(token.span(), "Duplicate Unexpected Token.",));
                        }
                    }

                    unexpected.insert(token);
                }
            }
        }

        Ok(Self {
            span,
            groups,
            unexpected,
        })
    }
}

impl Recovery {
    const GROUPS_LIMIT: usize = 4;

    #[inline(always)]
    pub(super) fn empty(span: Span) -> Self {
        Self {
            span,
            groups: Default::default(),
            unexpected: Default::default(),
        }
    }

    #[inline(always)]
    pub(super) fn span(&self) -> Span {
        self.span
    }

    pub(super) fn is_empty(&self) -> bool {
        self.groups.is_empty() && self.unexpected.is_empty()
    }

    pub(super) fn compile(&self, token_type: &Type) -> TokenStream {
        if self.is_empty() {
            system_panic!("Unlimited recovery.");
        }

        let span = self.span;
        let core = span.face_core();

        let unexpected = match self.unexpected.len() {
            0 => None,
            1 => {
                let lit = expect_some!(self.unexpected.iter().next(), "Empty set.",);

                let ident = expect_some!(
                    lit.as_token_index(token_type),
                    "Unfiltered Unexpected token.",
                );

                Some(quote_spanned!(span=> .unexpected(#ident)))
            }

            _ => {
                let set = self.unexpected.iter().map(|lit| {
                    let ident = expect_some!(
                        lit.as_token_index(token_type),
                        "Unfiltered Unexpected token.",
                    );

                    let span = ident.span();

                    quote_spanned!(span=> #ident as u8)
                });

                Some(quote_spanned!(span=>
                    .unexpected_set(#core::lexis::TokenSet::inclusive(&[#(#set),*]))
                ))
            }
        };

        let groups = self.groups.iter().map(|(open, close)| {
            let open = expect_some!(open.as_token_index(token_type), "Unfiltered Open token.",);
            let close = expect_some!(close.as_token_index(token_type), "Unfiltered Close token.",);

            quote_spanned!(span=> #open, #close)
        });

        quote_spanned!(span=>
            #core::syntax::Recovery::unlimited()
            #unexpected
            #(.group(#groups))*)
    }
}

enum SpecEntry {
    Group(Span, TokenLit, TokenLit),
    Unexpected(TokenLit),
}

impl Parse for SpecEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Token![$]) {
            return Ok(SpecEntry::Unexpected(input.parse::<TokenLit>()?));
        }

        if lookahead.peek(Bracket) {
            let content;
            bracketed!(content in input);

            let span = content.span();
            let open = content.parse::<TokenLit>()?;
            let _ = content.parse::<Token![..]>()?;
            let close = content.parse::<TokenLit>()?;

            if open == close {
                return Err(error!(
                    close.span(),
                    "Group close token must distinct from the Open token.",
                ));
            }

            if !content.is_empty() {
                return Err(error!(
                    content.span(),
                    "Unexpected end of input.\nExpected a pair of tokens \
                        [$<open token>..$<close token>].",
                ));
            }

            return Ok(SpecEntry::Group(span, open, close));
        }

        Err(lookahead.error())
    }
}
