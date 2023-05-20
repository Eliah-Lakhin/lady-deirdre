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
};

use proc_macro2::{Ident, Span};
use syn::spanned::Spanned;

use crate::{
    node::{builder::Builder, regex::operand::TokenLit},
    utils::AutomataTerminal,
};

#[derive(Clone, Hash, PartialEq, Eq)]
pub(in crate::node) enum Terminal {
    Null,
    Token {
        name: TokenLit,
        capture: Option<Ident>,
    },
    Node {
        name: Ident,
        capture: Option<Ident>,
    },
}

impl AutomataTerminal for Terminal {
    #[inline(always)]
    fn null() -> Self {
        Self::Null
    }

    #[inline(always)]
    fn is_null(&self) -> bool {
        match self {
            Self::Null => true,
            _ => false,
        }
    }
}

impl Ord for Terminal {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        use Ordering::*;

        match self.order().cmp(&other.order()) {
            Less => Less,
            Greater => Greater,
            Equal => match self.string().cmp(&other.string()) {
                Less => Less,
                Greater => Greater,
                Equal => self.capture().cmp(&other.capture()),
            },
        }
    }
}

impl PartialOrd for Terminal {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Terminal {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => formatter.write_str("null"),

            Self::Token {
                name,
                capture: None,
            } => Display::fmt(name, formatter),

            Self::Token {
                name,
                capture: Some(target),
            } => formatter.write_fmt(format_args!("{}: {}", target.to_string(), name)),

            Self::Node {
                name,
                capture: None,
            } => formatter.write_fmt(format_args!("{}", name.to_string())),

            Self::Node {
                name,
                capture: Some(target),
            } => formatter.write_fmt(format_args!("{}: {}", target.to_string(), name.to_string())),
        }
    }
}

impl Spanned for Terminal {
    #[inline(always)]
    fn span(&self) -> Span {
        match self {
            Terminal::Null => Span::call_site(),
            Terminal::Token { name, .. } => name.span(),
            Terminal::Node { name, .. } => name.span(),
        }
    }
}

impl Terminal {
    #[inline(always)]
    pub(in crate::node) fn capture(&self) -> Option<&Ident> {
        match self {
            Self::Null => None,
            Self::Token { capture, .. } => capture.as_ref(),
            Self::Node { capture, .. } => capture.as_ref(),
        }
    }

    #[inline(always)]
    pub(in crate::node) fn is_skip(&self, builder: &Builder) -> bool {
        match self {
            Self::Null => false,
            Self::Token { name, .. } => builder.skip_leftmost().tokens().contains(name),
            Self::Node { name, .. } => builder.skip_leftmost().nodes().contains(name),
        }
    }

    #[inline(always)]
    fn order(&self) -> u8 {
        match self {
            Self::Null => 0,
            Self::Token { capture, .. } => 1 + (capture.is_some() as u8),
            Self::Node { capture, .. } => 3 + (capture.is_some() as u8),
        }
    }

    #[inline(always)]
    fn string(&self) -> Option<String> {
        match self {
            Self::Null => None,
            Self::Token { name, .. } => Some(name.to_string()),
            Self::Node { name, .. } => Some(name.to_string()),
        }
    }
}
