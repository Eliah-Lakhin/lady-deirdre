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
    fmt::{Display, Formatter},
    iter::{Flatten, FusedIterator, Map},
};

use crate::{
    lexis::{Site, SiteSpan, TokenRef},
    syntax::{AbstractNode, NodeRef, PolyRef, RefKind},
    units::CompilationUnit,
};

/// A polymorphic key that is either a string or a numeric key.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Key<'a> {
    /// A string key. Usually denotes the enum variant field name.
    Name(&'a str),

    /// A numeric key. Usually denotes the index of the variant field.
    Index(usize),
}

impl<'a> Display for Key<'a> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Name(key) => Display::fmt(key, formatter),
            Self::Index(key) => Display::fmt(key, formatter),
        }
    }
}

impl<'a> From<&'a str> for Key<'a> {
    #[inline(always)]
    fn from(value: &'a str) -> Self {
        Self::Name(value)
    }
}

impl<'a> From<usize> for Key<'a> {
    #[inline(always)]
    fn from(value: usize) -> Self {
        Self::Index(value)
    }
}

/// A set of the node children grouped together.
///
/// During the syntax tree node parsing, the parser usually captures
/// individual tokens and descends into other rules, capturing their node
/// products.
///
/// The parser groups these objects' [TokenRef] and [NodeRef] references
/// together, and puts these groups under the Node's enum variant fields.
///
/// Lady Deirdre refers to these groups as "captures".
///
/// The "Single*" variants of this enum represent captures when the parser
/// captures exactly one child (`foo: bar`), or zero or one
/// child (`foo: bar?`).
///
/// The "Many*" variants of this enum represent captures when the parser
/// captures an array of children (`foo: bar*`).
///
/// Any captured references within this object could be
/// [nil](PolyRef::is_nil) references, and the arrays of children could be empty
/// arrays.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Capture<'a> {
    /// A single node capture.
    ///
    /// Represents zero or one node (`foo: Bar?`), or exactly one
    /// node (`foo: Bar`).
    ///
    /// If the parsing rule didn't capture anything, the value is
    /// [NodeRef::nil].
    SingleNode(&'a NodeRef),

    /// A capture of an array of nodes.
    ///
    /// Represents zero or many nodes (`foo: Bar*`), or one or many nodes
    /// (`foo: Bar+`).
    ManyNodes(&'a Vec<NodeRef>),

    /// A single token capture.
    ///
    /// Represents zero or one token (`foo: $Bar?`), or exactly one
    /// token (`foo: $Bar`).
    ///
    /// If the parsing rule didn't capture anything, the value is
    /// [TokenRef::nil].
    SingleToken(&'a TokenRef),

    /// A capture of an array of tokens.
    ///
    /// Represents zero or many tokens (`foo: $Bar*`), or one or many tokens
    /// (`foo: $Bar+`).
    ManyTokens(&'a Vec<TokenRef>),
}

impl<'a> From<&'a NodeRef> for Capture<'a> {
    #[inline(always)]
    fn from(capture: &'a NodeRef) -> Self {
        Self::SingleNode(capture)
    }
}

impl<'a> From<&'a Vec<NodeRef>> for Capture<'a> {
    #[inline(always)]
    fn from(capture: &'a Vec<NodeRef>) -> Self {
        Self::ManyNodes(capture)
    }
}

impl<'a> From<&'a TokenRef> for Capture<'a> {
    #[inline(always)]
    fn from(capture: &'a TokenRef) -> Self {
        Self::SingleToken(capture)
    }
}

impl<'a> From<&'a Vec<TokenRef>> for Capture<'a> {
    #[inline(always)]
    fn from(capture: &'a Vec<TokenRef>) -> Self {
        Self::ManyTokens(capture)
    }
}

impl<'a> IntoIterator for Capture<'a> {
    type Item = &'a dyn PolyRef;
    type IntoIter = CaptureIntoIter<'a>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        CaptureIntoIter::new(self)
    }
}

impl<'a> Capture<'a> {
    /// Describes captured children kind.
    #[inline(always)]
    pub fn kind(&self) -> RefKind {
        match self {
            Capture::SingleNode(..) | Capture::ManyNodes(..) => RefKind::Node,
            Capture::SingleToken(..) | Capture::ManyTokens(..) => RefKind::Token,
        }
    }

    /// Returns true, if the Capture represents a "Single*" child.
    #[inline(always)]
    pub fn is_single(&self) -> bool {
        match self {
            Capture::SingleNode(..) | Capture::SingleToken(..) => true,
            Capture::ManyTokens(..) | Capture::ManyNodes(..) => false,
        }
    }

    /// Returns true, if the Capture represents "Many*" children.
    #[inline(always)]
    pub fn is_many(&self) -> bool {
        !self.is_single()
    }

    /// Returns the total number of children in this Capture (including the
    /// [nil](PolyRef::is_nil) entities).
    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            Capture::SingleNode(..) | Capture::SingleToken(..) => 1,
            Capture::ManyNodes(capture) => capture.len(),
            Capture::ManyTokens(capture) => capture.len(),
        }
    }

    /// Returns true, if `self.len() == 0`.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        match self {
            Capture::SingleNode(..) | Capture::SingleToken(..) => false,
            Capture::ManyNodes(capture) => capture.is_empty(),
            Capture::ManyTokens(capture) => capture.is_empty(),
        }
    }

    /// Returns a child within this Capture by `index`.
    ///
    /// If the child is inside the "Single*" capture, the only valid index is 0.
    ///
    /// Returns None if the index is out of bounds.
    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&'a dyn PolyRef> {
        match self {
            Capture::SingleNode(capture) if index == 0 => Some(*capture),
            Capture::SingleToken(capture) if index == 0 => Some(*capture),
            Capture::ManyNodes(capture) => capture.get(index).map(|capture| capture as _),
            Capture::ManyTokens(capture) => capture.get(index).map(|capture| capture as _),
            _ => None,
        }
    }

    /// Returns the same as `self.get(0)`.
    #[inline(always)]
    pub fn first(&self) -> Option<&'a dyn PolyRef> {
        match self {
            Capture::SingleNode(capture) => Some(*capture),
            Capture::ManyNodes(capture) => capture.first().map(|capture| capture as _),
            Capture::SingleToken(capture) => Some(*capture),
            Capture::ManyTokens(capture) => capture.first().map(|capture| capture as _),
        }
    }

    /// Returns the same as `self.get(self.len() - 1)`
    /// or None if the Capture is empty.
    #[inline(always)]
    pub fn last(&self) -> Option<&'a dyn PolyRef> {
        match self {
            Capture::SingleNode(capture) => Some(*capture),
            Capture::ManyNodes(capture) => capture.last().map(|capture| capture as _),
            Capture::SingleToken(capture) => Some(*capture),
            Capture::ManyTokens(capture) => capture.last().map(|capture| capture as _),
        }
    }

    /// Computes the [site span](SiteSpan) from the [first child](Self::first)
    /// start site to the [last child](Self::last) end site.
    ///
    /// If the Capture is empty, or the corresponding child instance does not
    /// exist in the `unit`, or the corresponding sites cannot be inferred
    /// (e.g., if the Node's [span](AbstractNode::span) returns None),
    /// the function returns None.
    pub fn site_span(&self, unit: &impl CompilationUnit) -> Option<SiteSpan> {
        let start_site = self.start(unit)?;
        let end_site = self.end(unit)?;

        Some(start_site..end_site)
    }

    /// Computes the start [site](Site) of the first child in this Capture.
    ///
    /// If the Capture is empty, or the corresponding child instance does not
    /// exist in the `unit`, or the corresponding site cannot be inferred
    /// (e.g., if the Node's [start](AbstractNode::start) returns None),
    /// the function returns None.
    pub fn start(&self, unit: &impl CompilationUnit) -> Option<Site> {
        match self {
            Capture::SingleNode(capture) => (*capture).deref(unit)?.start(unit),
            Capture::ManyNodes(capture) => capture.first()?.deref(unit)?.start(unit),
            Capture::SingleToken(capture) => Some(capture.chunk(unit)?.start()),
            Capture::ManyTokens(capture) => Some(capture.first()?.chunk(unit)?.start()),
        }
    }

    /// Computes the end [site](Site) of the last child in this Capture.
    ///
    /// If the Capture is empty, or the corresponding child instance does not
    /// exist in the `unit`, or the corresponding site cannot be inferred
    /// (e.g., if the Node's [end](AbstractNode::end) returns None),
    /// the function returns None.
    pub fn end(&self, unit: &impl CompilationUnit) -> Option<Site> {
        match self {
            Capture::SingleNode(capture) => (*capture).deref(unit)?.end(unit),
            Capture::ManyNodes(capture) => capture.last()?.deref(unit)?.end(unit),
            Capture::SingleToken(capture) => Some(capture.chunk(unit)?.end()),
            Capture::ManyTokens(capture) => Some(capture.last()?.chunk(unit)?.end()),
        }
    }
}

/// An owned iterator over the [Capture] children.
///
/// This object is created by the `into_iter` function of the Capture.
pub struct CaptureIntoIter<'a> {
    front: usize,
    back: usize,
    capture: Capture<'a>,
}

impl<'a> Iterator for CaptureIntoIter<'a> {
    type Item = &'a dyn PolyRef;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }

        let index = self.front;

        self.front += 1;

        self.capture.get(index)
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.back - self.front;
        (remaining, Some(remaining))
    }
}

impl<'a> DoubleEndedIterator for CaptureIntoIter<'a> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }

        self.back -= 1;

        self.capture.get(self.back)
    }
}

impl<'a> ExactSizeIterator for CaptureIntoIter<'a> {}

impl<'a> FusedIterator for CaptureIntoIter<'a> {}

impl<'a> CaptureIntoIter<'a> {
    #[inline(always)]
    fn new(capture: Capture<'a>) -> Self {
        Self {
            front: 0,
            back: capture.len(),
            capture,
        }
    }
}

/// An iterator over all [captures](Capture) of the [Node](crate::syntax::Node)
/// interface.
///
/// This object is created by the [AbstractNode::captures_iter] function.
pub struct CapturesIter<'a, N: AbstractNode + ?Sized> {
    front: usize,
    back: usize,
    node: &'a N,
}

impl<'a, N: AbstractNode + ?Sized> Iterator for CapturesIter<'a, N> {
    type Item = Capture<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }

        let index = self.front;

        self.front += 1;

        self.node.capture(Key::Index(index))
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.back - self.front;
        (remaining, Some(remaining))
    }
}

impl<'a, N: AbstractNode + ?Sized> DoubleEndedIterator for CapturesIter<'a, N> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }

        self.back -= 1;

        self.node.capture(Key::Index(self.back))
    }
}

impl<'a, N: AbstractNode + ?Sized> ExactSizeIterator for CapturesIter<'a, N> {}

impl<'a, N: AbstractNode + ?Sized> FusedIterator for CapturesIter<'a, N> {}

impl<'a, N: AbstractNode + ?Sized> CapturesIter<'a, N> {
    #[inline(always)]
    pub(super) fn new(node: &'a N) -> Self {
        Self {
            front: 0,
            back: node.captures_len(),
            node,
        }
    }
}

/// An iterator over all children of the [Node](crate::syntax::Node)
/// interface.
///
/// This object is created by the [AbstractNode::children_iter] function.
#[repr(transparent)]
pub struct ChildrenIter<'a, N: AbstractNode + ?Sized> {
    inner: Flatten<Map<CapturesIter<'a, N>, fn(Capture) -> CaptureIntoIter>>,
}

impl<'a, N: AbstractNode + ?Sized> Iterator for ChildrenIter<'a, N> {
    type Item = &'a dyn PolyRef;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, N: AbstractNode + ?Sized> DoubleEndedIterator for ChildrenIter<'a, N> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'a, N: AbstractNode + ?Sized> FusedIterator for ChildrenIter<'a, N> {}

impl<'a, N: AbstractNode + ?Sized> ChildrenIter<'a, N> {
    #[inline(always)]
    pub(super) fn new(node: &'a N) -> Self {
        fn capture_into_iter(capture: Capture) -> CaptureIntoIter {
            capture.into_iter()
        }

        Self {
            inner: CapturesIter::new(node)
                .map(capture_into_iter as _)
                .flatten(),
        }
    }
}
