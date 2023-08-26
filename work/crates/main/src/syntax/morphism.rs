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

use crate::{
    arena::{Id, Identifiable},
    compiler::CompilationUnit,
    lexis::{SiteSpan, TokenRef},
    std::*,
    syntax::NodeRef,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PolyVariant {
    Token(TokenRef),
    Node(NodeRef),
}

impl Debug for PolyVariant {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Token(variant) => Debug::fmt(variant, formatter),
            Self::Node(variant) => Debug::fmt(variant, formatter),
        }
    }
}

impl Identifiable for PolyVariant {
    #[inline(always)]
    fn id(&self) -> Id {
        match self {
            Self::Token(child) => child.id,
            Self::Node(child) => child.id,
        }
    }
}

impl PolyRef for PolyVariant {
    #[inline(always)]
    fn kind(&self) -> RefKind {
        match self {
            Self::Token(..) => RefKind::Token,
            Self::Node(..) => RefKind::Node,
        }
    }

    #[inline(always)]
    fn is_nil(&self) -> bool {
        match self {
            Self::Token(variant) => variant.is_nil(),
            Self::Node(variant) => variant.is_nil(),
        }
    }

    #[inline(always)]
    fn as_variant(&self) -> PolyVariant {
        *self
    }

    #[inline(always)]
    fn as_token_ref(&self) -> &TokenRef {
        static NIL: TokenRef = TokenRef::nil();

        match self {
            Self::Token(variant) => variant,
            Self::Node(..) => &NIL,
        }
    }

    #[inline(always)]
    fn as_node_ref(&self) -> &NodeRef {
        static NIL: NodeRef = NodeRef::nil();

        match self {
            Self::Token(..) => &NIL,
            Self::Node(variant) => variant,
        }
    }

    #[inline(always)]
    fn span(&self, unit: &impl CompilationUnit) -> Option<SiteSpan> {
        match self {
            Self::Token(variant) => variant.span(unit),
            Self::Node(variant) => variant.span(unit),
        }
    }
}

pub trait PolyRef: Identifiable + Debug + 'static {
    fn kind(&self) -> RefKind;

    fn is_nil(&self) -> bool;

    fn as_variant(&self) -> PolyVariant;

    fn as_token_ref(&self) -> &TokenRef;

    fn as_node_ref(&self) -> &NodeRef;

    fn span(&self, unit: &impl CompilationUnit) -> Option<SiteSpan>
    where
        Self: Sized;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefKind {
    Token,
    Node,
}

impl RefKind {
    #[inline(always)]
    pub fn is_token(&self) -> bool {
        match self {
            Self::Token => true,
            _ => false,
        }
    }

    #[inline(always)]
    pub fn is_node(&self) -> bool {
        match self {
            Self::Node => true,
            _ => false,
        }
    }
}
