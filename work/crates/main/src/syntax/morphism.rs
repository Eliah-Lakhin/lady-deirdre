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
    borrow::Borrow,
    fmt::{Debug, Display, Formatter},
};

use crate::{
    arena::{Id, Identifiable},
    format::{AnnotationPriority, SnippetConfig, SnippetFormatter},
    lexis::{SiteSpan, ToSpan, Token, TokenRef, NIL_TOKEN_REF},
    report::ld_unreachable,
    syntax::{AbstractNode, NodeRef, NIL_NODE_REF},
    units::CompilationUnit,
};

/// An owned wrapper of [NodeRef] and [TokenRef].
///
/// This is a helper object that wraps both kinds of syntax and lexical
/// component references into a single one.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PolyVariant {
    /// This polymorphic variant represents a [TokenRef] reference.
    Token(TokenRef),

    /// This polymorphic variant represents a [NodeRef] reference.
    Node(NodeRef),
}

impl Debug for PolyVariant {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
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

/// A generic interface for the [NodeRef] and the [TokenRef].
///
/// This trait is implemented for the [NodeRef], [TokenRef], and
/// the [PolyVariant], and provides functions common to all of them.
pub trait PolyRef: Identifiable + Debug + 'static {
    /// Returns a discriminant of the underlying reference kind.
    fn kind(&self) -> RefKind;

    /// Returns true, if the underlying reference intentionally does not refer
    /// to any node or token within any compilation unit.
    fn is_nil(&self) -> bool;

    /// Returns an owned wrapper of the [NodeRef] and [TokenRef].
    fn as_variant(&self) -> PolyVariant;

    /// Returns a [TokenRef] if this PolyRef represents a TokenRef; otherwise
    /// returns a [TokenRef::nil].
    fn as_token_ref(&self) -> &TokenRef;

    /// Returns a [NodeRef] if this PolyRef represents a NodeRef; otherwise
    /// returns a [NodeRef::nil].
    fn as_node_ref(&self) -> &NodeRef;

    /// Computes a [site span](SiteSpan) of the underlying object.
    ///
    /// Returns None if the instance referred to by the underlying reference
    /// does not exist in the `unit`.
    ///
    /// If the underlying object is a token, the function returns a span
    /// of its char bounds.
    ///
    /// If the underlying object is a node, the function delegates span
    /// computation to the [AbstractNode::span] function.
    fn span(&self, unit: &impl CompilationUnit) -> Option<SiteSpan>
    where
        Self: Sized;

    /// Returns a displayable object that prints the underlying object metadata
    /// for debugging purposes.
    ///
    /// If the underlying reference is not valid for the specified `unit`,
    /// the returning object would [Debug] the [NodeRef] or a [TokenRef].
    #[inline(always)]
    fn display<'unit>(&self, unit: &'unit impl CompilationUnit) -> impl Debug + Display + 'unit
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

/// A discriminant of the [PolyRef].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefKind {
    /// The underlying polymorphic reference is a [TokenRef].
    Token,

    /// The underlying polymorphic reference is a [NodeRef].
    Node,
}

impl RefKind {
    /// Returns true, if `self == Self::Token`.
    #[inline(always)]
    pub fn is_token(&self) -> bool {
        match self {
            Self::Token => true,
            _ => false,
        }
    }

    /// Returns true, if `self == Self::Node`.
    #[inline(always)]
    pub fn is_node(&self) -> bool {
        match self {
            Self::Node => true,
            _ => false,
        }
    }
}

struct DisplayPolyRef<'unit, U: CompilationUnit> {
    unit: &'unit U,
    variant: PolyVariant,
}

impl<'unit, U: CompilationUnit> Debug for DisplayPolyRef<'unit, U> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, formatter)
    }
}

impl<'unit, U: CompilationUnit> Display for DisplayPolyRef<'unit, U> {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        let mut summary = String::new();
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
                    None => unsafe { ld_unreachable!("Invalid chunk span.") },
                };

                let token = chunk.token;

                summary.push_str("Token: ");
                summary.push_str(token.name().unwrap_or("?"));
                summary.push_str("\nDescription: ");
                summary.push_str(token.describe(true).unwrap_or("?"));
                summary.push_str("\nEntry: ");
                summary.push_str(&format!("{:?}", variant.entry));
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
                summary.push_str("\nNode entry: ");
                summary.push_str(&format!("{:?}", variant.entry));
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
            .annotate(span, AnnotationPriority::Default, "")
            .finish()
    }
}
