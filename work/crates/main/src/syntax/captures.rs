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
    lexis::{Site, SiteSpan, TokenRef},
    std::*,
    syntax::{AbstractNode, NodeRef, PolyRef, RefKind},
    units::CompilationUnit,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Key<'a> {
    Name(&'a str),
    Index(usize),
}

impl<'a> Display for Key<'a> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
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

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Capture<'a> {
    SingleNode(&'a NodeRef),
    ManyNodes(&'a Vec<NodeRef>),
    SingleToken(&'a TokenRef),
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
    #[inline(always)]
    pub fn kind(&self) -> RefKind {
        match self {
            Capture::SingleNode(..) | Capture::ManyNodes(..) => RefKind::Node,
            Capture::SingleToken(..) | Capture::ManyTokens(..) => RefKind::Token,
        }
    }

    #[inline(always)]
    pub fn is_single(&self) -> bool {
        match self {
            Capture::SingleNode(..) | Capture::SingleToken(..) => true,
            Capture::ManyTokens(..) | Capture::ManyNodes(..) => false,
        }
    }

    #[inline(always)]
    pub fn is_many(&self) -> bool {
        !self.is_single()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            Capture::SingleNode(..) | Capture::SingleToken(..) => 1,
            Capture::ManyNodes(capture) => capture.len(),
            Capture::ManyTokens(capture) => capture.len(),
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        match self {
            Capture::SingleNode(..) | Capture::SingleToken(..) => false,
            Capture::ManyNodes(capture) => capture.is_empty(),
            Capture::ManyTokens(capture) => capture.is_empty(),
        }
    }

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

    #[inline(always)]
    pub fn first(&self) -> Option<&'a dyn PolyRef> {
        match self {
            Capture::SingleNode(capture) => Some(*capture),
            Capture::ManyNodes(capture) => capture.first().map(|capture| capture as _),
            Capture::SingleToken(capture) => Some(*capture),
            Capture::ManyTokens(capture) => capture.first().map(|capture| capture as _),
        }
    }

    #[inline(always)]
    pub fn last(&self) -> Option<&'a dyn PolyRef> {
        match self {
            Capture::SingleNode(capture) => Some(*capture),
            Capture::ManyNodes(capture) => capture.last().map(|capture| capture as _),
            Capture::SingleToken(capture) => Some(*capture),
            Capture::ManyTokens(capture) => capture.last().map(|capture| capture as _),
        }
    }

    pub fn site_span(&self, unit: &impl CompilationUnit) -> Option<SiteSpan> {
        let start_site = self.start(unit)?;
        let end_site = self.end(unit)?;

        Some(start_site..end_site)
    }

    pub fn start(&self, unit: &impl CompilationUnit) -> Option<Site> {
        match self {
            Capture::SingleNode(capture) => (*capture).deref(unit)?.start(unit),
            Capture::ManyNodes(capture) => capture.first()?.deref(unit)?.start(unit),
            Capture::SingleToken(capture) => Some(capture.chunk(unit)?.start()),
            Capture::ManyTokens(capture) => Some(capture.first()?.chunk(unit)?.start()),
        }
    }

    pub fn end(&self, unit: &impl CompilationUnit) -> Option<Site> {
        match self {
            Capture::SingleNode(capture) => (*capture).deref(unit)?.end(unit),
            Capture::ManyNodes(capture) => capture.last()?.deref(unit)?.end(unit),
            Capture::SingleToken(capture) => Some(capture.chunk(unit)?.end()),
            Capture::ManyTokens(capture) => Some(capture.last()?.chunk(unit)?.end()),
        }
    }
}

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
