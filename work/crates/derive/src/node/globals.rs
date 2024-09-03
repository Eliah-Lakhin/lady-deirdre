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
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::Type;

use crate::{
    node::{index::Index, recovery::Recovery, token::TokenLit},
    utils::{expect_some, Facade},
};

#[derive(Default)]
pub(super) struct Globals {
    recoveries: BTreeMap<Recovery, String>,
    rules: BTreeMap<BTreeSet<Index>, String>,
    tokens: BTreeMap<TokenSet, String>,
}

impl Globals {
    pub(super) fn compile(&self, span: Span, token_type: &Type) -> TokenStream {
        #[inline(always)]
        fn compare_keys(a: &str, b: &str) -> Ordering {
            let ordering = a.len().cmp(&b.len());

            if ordering.is_eq() {
                return a.cmp(&b);
            }

            ordering
        }

        let mut stream = TokenStream::new();

        let core = span.face_core();

        let mut recoveries = self.recoveries.iter().collect::<Vec<_>>();
        recoveries.sort_by(|(_, a), (_, b)| compare_keys(a, b));

        let mut rules = self.rules.iter().collect::<Vec<_>>();
        rules.sort_by(|(_, a), (_, b)| compare_keys(a, b));

        let mut tokens = self.tokens.iter().collect::<Vec<_>>();
        tokens.sort_by(|(_, a), (_, b)| compare_keys(a, b));

        for (recovery, ident) in recoveries {
            let recovery = recovery.compile(token_type);
            let ident = Ident::new(ident, span);

            quote_spanned!(span=> static #ident: #core::syntax::Recovery = #recovery;)
                .to_tokens(&mut stream);
        }

        for (rules, ident) in rules {
            let ident = Ident::new(ident, span);

            quote_spanned!(span=>
                static #ident: #core::syntax::NodeSet =
                    #core::syntax::NodeSet::new(&[#(#rules),*]);
            )
            .to_tokens(&mut stream);
        }

        for (tokens, ident) in tokens {
            let ident = Ident::new(ident, span);

            match tokens {
                TokenSet::Exclusive(lits) => {
                    let set = lits.into_iter().map(|lit| {
                        expect_some!(lit.as_token_index(token_type), "Unfiltered token.",)
                    });

                    quote_spanned!(span=>
                        static #ident: #core::lexis::TokenSet
                            = #core::lexis::TokenSet::exclusive(&[#(#set),*]);
                    )
                    .to_tokens(&mut stream);
                }

                TokenSet::Inclusive(lits) => {
                    let set = lits.into_iter().map(|lit| {
                        expect_some!(lit.as_token_index(token_type), "Unfiltered token.",)
                    });

                    quote_spanned!(span=>
                        static #ident: #core::lexis::TokenSet
                            = #core::lexis::TokenSet::inclusive(&[#(#set),*]);
                    )
                    .to_tokens(&mut stream);
                }
            }
        }

        stream
    }

    pub(super) fn recovery(&mut self, recovery: Recovery) -> GlobalVar {
        if recovery.is_empty() {
            return GlobalVar::UnlimitedRecovery;
        }

        if let Some(ident) = self.recoveries.get(&recovery) {
            return GlobalVar::Static(ident.clone());
        }

        let ident = format!("RECOVERY_{}", self.recoveries.len() + 1);

        let _ = self.recoveries.insert(recovery, ident.clone());

        GlobalVar::Static(ident.clone())
    }

    pub(super) fn rules(&mut self, set: impl Iterator<Item = Index>) -> GlobalVar {
        let set = set.collect::<BTreeSet<_>>();

        if set.is_empty() {
            return GlobalVar::EmptyNodeSet;
        }

        if let Some(ident) = self.rules.get(&set) {
            return GlobalVar::Static(ident.clone());
        }

        let ident = format!("RULES_{}", self.rules.len() + 1);

        let _ = self.rules.insert(set, ident.clone());

        GlobalVar::Static(ident.clone())
    }

    pub(super) fn inclusive_tokens(&mut self, set: impl Iterator<Item = TokenLit>) -> GlobalVar {
        let set = set.collect::<BTreeSet<_>>();

        if set.is_empty() {
            return GlobalVar::EmptyTokenSet;
        }

        self.tokens(TokenSet::Inclusive(set))
    }

    pub(super) fn exclusive_tokens(&mut self, set: impl Iterator<Item = TokenLit>) -> GlobalVar {
        let set = set.collect::<BTreeSet<_>>();

        if set.is_empty() {
            return GlobalVar::FullTokenSet;
        }

        self.tokens(TokenSet::Exclusive(set))
    }

    fn tokens(&mut self, set: TokenSet) -> GlobalVar {
        if set.is_empty() {
            return GlobalVar::EmptyTokenSet;
        }

        if let Some(ident) = self.tokens.get(&set) {
            return GlobalVar::Static(ident.clone());
        }

        let ident = format!("TOKENS_{}", self.tokens.len() + 1);

        let _ = self.tokens.insert(set, ident.clone());

        GlobalVar::Static(ident.clone())
    }
}

pub(super) enum GlobalVar {
    Static(String),
    EmptyTokenSet,
    FullTokenSet,
    EmptyNodeSet,
    UnlimitedRecovery,
}

impl GlobalVar {
    #[inline]
    pub(super) fn compile(&self, span: Span) -> TokenStream {
        match self {
            Self::Static(string) => Ident::new(string, span).to_token_stream(),

            Self::EmptyTokenSet => {
                let core = span.face_core();

                quote_spanned!(span=> #core::lexis::EMPTY_TOKEN_SET)
            }

            Self::FullTokenSet => {
                let core = span.face_core();

                quote_spanned!(span=> #core::lexis::FULL_TOKEN_SET)
            }

            Self::EmptyNodeSet => {
                let core = span.face_core();

                quote_spanned!(span=> #core::syntax::EMPTY_NODE_SET)
            }

            Self::UnlimitedRecovery => {
                let core = span.face_core();

                quote_spanned!(span=> #core::syntax::UNLIMITED_RECOVERY)
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TokenSet {
    Inclusive(BTreeSet<TokenLit>),
    Exclusive(BTreeSet<TokenLit>),
}

impl TokenSet {
    #[inline(always)]
    fn is_empty(&self) -> bool {
        match self {
            Self::Inclusive(set) => set.is_empty(),
            Self::Exclusive(set) => set.is_empty(),
        }
    }
}
