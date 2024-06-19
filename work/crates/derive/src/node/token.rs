////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
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

use crate::utils::Facade;

#[derive(Clone)]
pub(super) enum TokenLit {
    Ident(Ident),
    Other(Span),
    EOI(Span),
}

impl Display for TokenLit {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ident(ident) => formatter.write_fmt(format_args!("${ident}")),
            Self::Other(..) => formatter.write_str("$_"),
            Self::EOI(..) => formatter.write_str("$"),
        }
    }
}

impl Hash for TokenLit {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        discriminant(self).hash(state);

        match self {
            Self::Ident(ident) => ident.hash(state),
            Self::Other(..) | Self::EOI(..) => (),
        }
    }
}

impl PartialEq for TokenLit {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ident(this), Self::Ident(other)) => this.eq(other),
            (Self::Other(..), Self::Other(..)) => true,
            (Self::EOI(..), Self::EOI(..)) => true,
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
            (Self::EOI(..), Self::EOI(..)) => Ordering::Equal,
            (Self::Ident(..), ..) => Ordering::Less,
            (Self::Other(..), ..) => Ordering::Less,
            (Self::EOI(..), ..) => Ordering::Greater,
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
            Self::EOI(span) => *span,
        }
    }

    #[inline]
    pub(super) fn set_span(&mut self, span: Span) {
        match self {
            Self::Ident(ident) => ident.set_span(span),
            Self::Other(other) => *other = span,
            Self::EOI(other) => *other = span,
        }
    }

    #[inline(always)]
    pub(super) fn is_eoi(&self) -> bool {
        match self {
            Self::EOI(..) => true,
            _ => false,
        }
    }

    #[inline(always)]
    pub(super) fn is_other(&self) -> bool {
        match self {
            Self::Other(..) => true,
            _ => false,
        }
    }

    pub(super) fn as_enum_variant(&self, token_type: &Type) -> Option<TokenStream> {
        match self {
            Self::Ident(ident) => Some(quote_spanned!(ident.span()=> #token_type::#ident)),
            Self::EOI(span) => {
                let core = span.face_core();

                Some(quote_spanned!(*span=> <#token_type as #core::lexis::Token>::eoi()))
            }
            _ => None,
        }
    }

    pub(super) fn as_token_index(&self, token_type: &Type) -> Option<TokenStream> {
        match self {
            Self::Ident(ident) => Some(quote_spanned!(ident.span()=> #token_type::#ident as u8)),
            Self::EOI(span) => {
                let core = span.face_core();

                Some(quote_spanned!(*span=> #core::lexis::EOI))
            }
            _ => None,
        }
    }
}
