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

use proc_macro2::Span;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Error,
    LitChar,
    Result,
};

use crate::{
    token::{scope::Scope, terminal::Terminal, NULL},
    utils::{Automata, AutomataContext, PredictableCollection, Set, SetImpl},
};

#[derive(Clone)]
pub(super) struct CharacterSet {
    span: Span,
    set: Set<char>,
}

impl Parse for CharacterSet {
    fn parse(input: ParseStream) -> Result<Self> {
        struct Component(Set<char>);

        impl Parse for Component {
            fn parse(input: ParseStream) -> Result<Self> {
                let lookahead = input.lookahead1();

                if lookahead.peek(syn::LitChar) {
                    let start_literal = input.parse::<LitChar>()?;
                    let start_character = start_literal.value();

                    if start_character == NULL {
                        return Err(Error::new(
                            start_literal.span(),
                            "Null characters forbidden.",
                        ));
                    }

                    if input.peek(Token![..]) {
                        let span = input.parse::<Token![..]>()?.span();

                        let end_literal = input.parse::<LitChar>()?;
                        let end_character = end_literal.value();

                        if start_character >= end_character {
                            return Err(Error::new(
                                span,
                                "Range start must be lesser than the range end.",
                            ));
                        }

                        return Ok(Self(
                            (start_character..=end_character)
                                .into_iter()
                                .map(|character| {
                                    if character == NULL {
                                        return Err(Error::new(span, "Null characters forbidden."));
                                    }

                                    Ok(character)
                                })
                                .collect::<Result<_>>()?,
                        ));
                    }

                    return Ok(Self(Set::new([start_character])));
                }

                Err(lookahead.error())
            }
        }

        let span = input.span();

        let set = Punctuated::<Component, Token![,]>::parse_terminated(input)?
            .into_iter()
            .fold(Set::empty(), |accumulator, component| {
                accumulator.merge(component.0)
            });

        if set.is_empty() {
            return Err(Error::new(span, "Empty character sets are forbidden."));
        }

        Ok(Self { span, set })
    }
}

impl Default for CharacterSet {
    #[inline(always)]
    fn default() -> Self {
        Self {
            span: Span::call_site(),
            set: Set::empty(),
        }
    }
}

impl Spanned for CharacterSet {
    #[inline(always)]
    fn span(&self) -> Span {
        self.span
    }
}

impl From<LitChar> for CharacterSet {
    fn from(literal: LitChar) -> Self {
        Self {
            span: literal.span(),
            set: Set::new([literal.value()]),
        }
    }
}

impl CharacterSet {
    #[inline(always)]
    pub(super) fn merge(self, other: Self) -> Self {
        Self {
            span: self.span,
            set: self.set.merge(other.set),
        }
    }

    #[inline]
    pub(super) fn into_inclusion(self, scope: &mut Scope) -> Automata<Scope> {
        scope.terminal(
            self.set
                .into_iter()
                .map(|character| Terminal::Character(character))
                .collect(),
        )
    }

    #[inline]
    pub(super) fn into_exclusion(self, scope: &mut Scope) -> Result<Automata<Scope>> {
        let mut alphabet = scope.alphabet().clone();

        for character in self.set {
            if !alphabet.set.remove(&character) {
                return Err(Error::new(
                    self.span,
                    format!(
                        "An exclusion character '{}' not found in any parsable rule \
                        alphabet.",
                        character.escape_debug(),
                    ),
                ));
            }
        }

        let alphabet = alphabet.into_inclusion(scope);
        let other = scope.other();

        Ok(scope.union(alphabet, other))
    }

    #[inline(always)]
    pub(super) fn into_inner(self) -> Set<char> {
        self.set
    }
}
