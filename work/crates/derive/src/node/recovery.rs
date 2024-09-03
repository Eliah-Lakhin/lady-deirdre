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

use std::{cmp::Ordering, collections::BTreeSet};

use proc_macro2::{Span, TokenStream};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
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
                    expect_some!(
                        lit.as_token_index(token_type),
                        "Unfiltered Unexpected token.",
                    )
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
