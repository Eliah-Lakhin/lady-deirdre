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
    hash::{Hash, Hasher},
    mem::discriminant,
};

use proc_macro2::{Ident, Span, TokenStream};
use syn::{
    parse::{Parse, ParseStream},
    Result,
    Type,
};

#[derive(Clone)]
pub(super) enum TokenLit {
    Ident(Ident),
    Other(Span),
}

impl Display for TokenLit {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ident(ident) => formatter.write_fmt(format_args!("${ident}")),
            Self::Other(..) => formatter.write_str("$_"),
        }
    }
}

impl Hash for TokenLit {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        discriminant(self).hash(state);

        match self {
            Self::Ident(ident) => ident.hash(state),
            Self::Other(..) => (),
        }
    }
}

impl PartialEq for TokenLit {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ident(this), Self::Ident(other)) => this.eq(other),
            (Self::Other(..), Self::Other(..)) => true,
            _ => false,
        }
    }
}

impl Eq for TokenLit {}

impl PartialOrd for TokenLit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TokenLit {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Ident(this), Self::Ident(other)) => this.cmp(other),
            (Self::Other(..), Self::Other(..)) => Ordering::Equal,
            (Self::Ident(..), ..) => Ordering::Less,
            (Self::Other(..), ..) => Ordering::Greater,
        }
    }
}

impl Parse for TokenLit {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<Token![$]>()?;
        let ident = input.parse::<Ident>()?;

        Ok(Self::Ident(ident))
    }
}

impl TokenLit {
    #[inline]
    pub(super) fn span(&self) -> Span {
        match self {
            Self::Ident(ident) => ident.span(),
            Self::Other(span) => *span,
        }
    }

    #[inline]
    pub(super) fn set_span(&mut self, span: Span) {
        match self {
            Self::Ident(ident) => ident.set_span(span),
            Self::Other(other) => *other = span,
        }
    }

    pub(super) fn as_enum_variant(&self, token_type: &Type) -> Option<TokenStream> {
        let ident = match self {
            Self::Ident(ident) => ident,
            Self::Other(..) => return None,
        };

        Some(quote_spanned!(ident.span()=> #token_type::#ident))
    }
}
