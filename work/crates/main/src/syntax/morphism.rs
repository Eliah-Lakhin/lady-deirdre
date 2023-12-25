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
    format::{PrintString, Priority, SnippetConfig, SnippetFormatter},
    lexis::{SiteSpan, ToSpan, Token, TokenRef, NIL_TOKEN_REF},
    report::debug_unreachable,
    std::*,
    syntax::{AbstractNode, NodeRef, NIL_NODE_REF},
    units::CompilationUnit,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PolyVariant {
    Token(TokenRef),
    Node(NodeRef),
}

impl Debug for PolyVariant {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
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

impl Borrow<dyn PolyRef> for PolyVariant {
    #[inline(always)]
    fn borrow(&self) -> &dyn PolyRef {
        self
    }
}

impl AsRef<dyn PolyRef> for PolyVariant {
    #[inline(always)]
    fn as_ref(&self) -> &dyn PolyRef {
        self
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
        match self {
            Self::Token(variant) => variant,
            Self::Node(..) => &NIL_TOKEN_REF,
        }
    }

    #[inline(always)]
    fn as_node_ref(&self) -> &NodeRef {
        match self {
            Self::Token(..) => &NIL_NODE_REF,
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

    #[inline(always)]
    fn display<'unit, U: CompilationUnit>(&self, unit: &'unit U) -> DisplayPolyRef<'unit, U>
    where
        Self: Sized,
    {
        DisplayPolyRef {
            unit,
            variant: self.as_variant(),
        }
    }
}

impl ToOwned for dyn PolyRef {
    type Owned = PolyVariant;

    #[inline(always)]
    fn to_owned(&self) -> Self::Owned {
        self.as_variant()
    }
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

pub struct DisplayPolyRef<'unit, U: CompilationUnit> {
    unit: &'unit U,
    variant: PolyVariant,
}

impl<'unit, U: CompilationUnit> Display for DisplayPolyRef<'unit, U> {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        let mut summary = PrintString::empty();
        let span;

        match self.variant {
            PolyVariant::Token(variant) => {
                let chunk = match variant.chunk(self.unit) {
                    None => return Debug::fmt(&variant, formatter),
                    Some(chunk) => chunk,
                };

                span = match chunk.to_site_span(self.unit) {
                    Some(span) => span,

                    // Safety: Chunks are always valid spans.
                    None => unsafe { debug_unreachable!("Invalid chunk span.") },
                };

                let token = chunk.token;

                summary.push_str("Token: ");
                summary.push_str(token.name().unwrap_or("?"));
                summary.push_str("\nDescription: ");
                summary.push_str(token.describe(true).unwrap_or("?"));
                summary.push_str("\nChunk entry: ");
                summary.push_str(&format!("{:?}", variant.chunk_entry));
                summary.push_str("\nLength: ");
                summary.push_str(&chunk.length.to_string());
                summary.push_str("\nSite span: ");
                summary.push_str(&span.start.to_string());
                summary.push_str("..");
                summary.push_str(&span.end.to_string());
                summary.push_str(&format!("\nPosition span: {}", span.display(self.unit)));
                summary.push_str(&format!("\nString: {:?}", chunk.string));
            }

            PolyVariant::Node(variant) => {
                let node = match variant.deref(self.unit) {
                    None => return Debug::fmt(&variant, formatter),
                    Some(chunk) => chunk,
                };

                span = match node.span(self.unit) {
                    None => return Debug::fmt(&variant, formatter),
                    Some(span) => span,
                };

                summary.push_str("Node: ");
                summary.push_str(node.name().unwrap_or("?"));
                summary.push_str("\nDescription: ");
                summary.push_str(node.describe(true).unwrap_or("?"));
                summary.push_str("\nCluster entry: ");
                summary.push_str(&format!("{:?}", variant.cluster_entry));
                summary.push_str("\nNode entry: ");
                summary.push_str(&format!("{:?}", variant.node_entry));
                summary.push_str("\nSite span: ");
                summary.push_str(&span.start.to_string());
                summary.push_str("..");
                summary.push_str(&span.end.to_string());
                summary.push_str(&format!("\nPosition span: {}", span.display(self.unit)));
            }
        }

        static CONFIG: SnippetConfig = SnippetConfig::verbose();

        formatter
            .snippet(self.unit)
            .set_config(&CONFIG)
            .set_caption(format!("Unit({})", self.unit.id()))
            .set_summary(summary)
            .annotate(span, Priority::Default, "")
            .finish()
    }
}
