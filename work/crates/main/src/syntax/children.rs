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
    compiler::CompilationUnit,
    lexis::{Site, SiteSpan, TokenRef},
    std::*,
    syntax::{node::Node, NodeRef, PolyRef, PolyVariant, RefKind},
};

#[derive(Clone)]
pub struct Children {
    vector: Vec<(&'static str, Child)>,
    map: StdMap<&'static str, usize>,
}

impl Debug for Children {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        let mut debug_struct = formatter.debug_struct("Children");

        for (key, value) in &self.vector {
            debug_struct.field(key, value);
        }

        debug_struct.finish()
    }
}

impl PartialEq for Children {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.vector.eq(&other.vector)
    }
}

impl Eq for Children {}

impl Default for Children {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Borrow<str>> Index<S> for Children {
    type Output = Child;

    #[inline(always)]
    fn index(&self, index: S) -> &Self::Output {
        self.get(index.borrow()).expect("Unknown key.")
    }
}

impl<'a> IntoIterator for &'a Children {
    type Item = &'a Child;
    type IntoIter = ChildrenIter<'a>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        ChildrenIter {
            iterator: self.vector.iter(),
        }
    }
}

impl IntoIterator for Children {
    type Item = Child;
    type IntoIter = ChildrenIntoIter;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        ChildrenIntoIter {
            iterator: self.vector.into_iter(),
        }
    }
}

impl FromIterator<(&'static str, Child)> for Children {
    fn from_iter<T: IntoIterator<Item = (&'static str, Child)>>(iter: T) -> Self {
        let vector = iter.into_iter().collect::<Vec<_>>();

        #[cfg(not(feature = "std"))]
        let mut map = StdMap::new();

        #[cfg(feature = "std")]
        let mut map = StdMap::with_capacity(vector.len());

        for (index, (key, _)) in vector.iter().enumerate() {
            if map.insert(*key, index).is_some() {
                panic!("Duplicate child {key:?}.");
            }
        }

        Self { vector, map }
    }
}

impl Children {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            vector: Vec::new(),
            map: StdMap::new(),
        }
    }

    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        #[cfg(not(feature = "std"))]
        let map = StdMap::new();

        #[cfg(feature = "std")]
        let map = StdMap::with_capacity(capacity);

        Self {
            vector: Vec::with_capacity(capacity),
            map,
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.vector.is_empty()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.vector.len()
    }

    #[inline]
    pub fn span(&self, unit: &impl CompilationUnit) -> Option<SiteSpan> {
        let start = self.start(unit)?;
        let end = self.end(unit)?;

        Some(start..end)
    }

    pub fn set(&mut self, key: &'static str, value: impl Into<Child>) {
        let value = value.into();

        let index = self.vector.len();
        self.vector.push((key, value));

        if self.map.insert(key, index).is_some() {
            panic!("Duplicate child {key:?}.");
        }
    }

    #[inline(always)]
    pub fn get(&self, key: &str) -> Option<&Child> {
        let index = *self.map.get(key)?;

        // Safety: `map` values are always valid indices into `vector`.
        let (_, value) = unsafe { self.vector.get_unchecked(index) };

        Some(value)
    }

    #[inline(always)]
    pub fn entries(&self) -> &[(&'static str, Child)] {
        &self.vector[..]
    }

    #[inline(always)]
    pub fn into_entries(self) -> Vec<(&'static str, Child)> {
        self.vector
    }

    #[inline(always)]
    pub fn flatten(&self) -> impl Iterator<Item = &dyn PolyRef> {
        self.vector
            .iter()
            .map(|(_, child)| child.into_iter())
            .flatten()
            .filter(|child| !child.is_nil())
    }

    #[inline(always)]
    pub fn nodes(&self) -> impl Iterator<Item = &NodeRef> {
        self.flatten().flat_map(|variant| match variant.kind() {
            RefKind::Token => None,
            RefKind::Node => Some(variant.as_node_ref()),
        })
    }

    pub fn prev_node(&self, current: &NodeRef) -> Option<&NodeRef> {
        let mut nodes = self.nodes().peekable();

        loop {
            let probe = nodes.peek()?;

            if *probe == current {
                return nodes.next();
            }
        }
    }

    pub fn next_node(&self, current: &NodeRef) -> Option<&NodeRef> {
        let mut nodes = self.nodes();

        loop {
            let probe = nodes.next()?;

            if probe == current {
                return nodes.next();
            }
        }
    }

    fn start(&self, unit: &impl CompilationUnit) -> Option<Site> {
        for (_, child) in self.vector.iter() {
            match child.start(unit) {
                None => continue,
                Some(site) => return Some(site),
            }
        }

        None
    }

    fn end(&self, unit: &impl CompilationUnit) -> Option<Site> {
        for (_, child) in self.vector.iter().rev() {
            match child.end(unit) {
                None => continue,
                Some(site) => return Some(site),
            }
        }

        None
    }
}

#[repr(transparent)]
pub struct ChildrenIter<'a> {
    iterator: Iter<'a, (&'static str, Child)>,
}

impl<'a> Iterator for ChildrenIter<'a> {
    type Item = &'a Child;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let (_, result) = self.iterator.next()?;
        Some(result)
    }
}

impl<'a> FusedIterator for ChildrenIter<'a> {}

#[repr(transparent)]
pub struct ChildrenIntoIter {
    iterator: IntoIter<(&'static str, Child)>,
}

impl Iterator for ChildrenIntoIter {
    type Item = Child;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let (_, result) = self.iterator.next()?;
        Some(result)
    }
}

impl FusedIterator for ChildrenIntoIter {}

#[derive(Clone, PartialEq, Eq)]
pub enum Child {
    Token(TokenRef),
    TokenSeq(Vec<TokenRef>),
    Node(NodeRef),
    NodeSeq(Vec<NodeRef>),
}

impl Debug for Child {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Token(child) => Debug::fmt(child, formatter),
            Self::TokenSeq(child) => Debug::fmt(child, formatter),
            Self::Node(child) => Debug::fmt(child, formatter),
            Self::NodeSeq(child) => Debug::fmt(child, formatter),
        }
    }
}

impl From<TokenRef> for Child {
    #[inline(always)]
    fn from(value: TokenRef) -> Self {
        Self::Token(value)
    }
}

impl<'a> From<&'a TokenRef> for Child {
    #[inline(always)]
    fn from(value: &'a TokenRef) -> Self {
        Self::Token(*value)
    }
}

impl From<Vec<TokenRef>> for Child {
    #[inline(always)]
    fn from(value: Vec<TokenRef>) -> Self {
        Self::TokenSeq(value)
    }
}

impl<'a> From<&'a Vec<TokenRef>> for Child {
    #[inline(always)]
    fn from(value: &'a Vec<TokenRef>) -> Self {
        Self::TokenSeq(value.clone())
    }
}

impl<'a> From<&'a [TokenRef]> for Child {
    #[inline(always)]
    fn from(value: &'a [TokenRef]) -> Self {
        Self::TokenSeq(value.iter().copied().collect())
    }
}

impl From<NodeRef> for Child {
    #[inline(always)]
    fn from(value: NodeRef) -> Self {
        Self::Node(value)
    }
}

impl<'a> From<&'a NodeRef> for Child {
    #[inline(always)]
    fn from(value: &'a NodeRef) -> Self {
        Self::Node(*value)
    }
}

impl From<Vec<NodeRef>> for Child {
    #[inline(always)]
    fn from(value: Vec<NodeRef>) -> Self {
        Self::NodeSeq(value)
    }
}

impl<'a> From<&'a Vec<NodeRef>> for Child {
    #[inline(always)]
    fn from(value: &'a Vec<NodeRef>) -> Self {
        Self::NodeSeq(value.clone())
    }
}

impl<'a> From<&'a [NodeRef]> for Child {
    #[inline(always)]
    fn from(value: &'a [NodeRef]) -> Self {
        Self::NodeSeq(value.iter().copied().collect())
    }
}

impl<'a> IntoIterator for &'a Child {
    type Item = &'a dyn PolyRef;
    type IntoIter = ChildIter<'a>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        ChildIter {
            child: self,
            index: 0,
        }
    }
}

impl IntoIterator for Child {
    type Item = PolyVariant;
    type IntoIter = ChildIntoIter;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        ChildIntoIter {
            child: self,
            index: 0,
        }
    }
}

impl Index<usize> for Child {
    type Output = dyn PolyRef;

    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("Index out of bounds.")
    }
}

impl Child {
    #[inline(always)]
    pub fn is_singleton(&self) -> bool {
        match self {
            Self::Token(..) | Self::Node(..) => true,
            Self::TokenSeq(..) | Self::NodeSeq(..) => false,
        }
    }

    #[inline(always)]
    pub fn is_sequence(&self) -> bool {
        !self.is_singleton()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        match self {
            Child::Token(..) | Child::Node(..) => false,
            Child::TokenSeq(seq) => seq.is_empty(),
            Child::NodeSeq(seq) => seq.is_empty(),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            Child::Token(..) | Child::Node(..) => 1,
            Child::TokenSeq(seq) => seq.len(),
            Child::NodeSeq(seq) => seq.len(),
        }
    }

    pub fn get(&self, index: usize) -> Option<&dyn PolyRef> {
        match self {
            Self::Token(child) => match index == 0 {
                true => Some(child),
                false => None,
            },

            Self::TokenSeq(child) => child.get(index).map(|poly_ref| poly_ref as &dyn PolyRef),

            Self::Node(child) => match index == 0 {
                true => Some(child),
                false => None,
            },

            Self::NodeSeq(child) => child.get(index).map(|poly_ref| poly_ref as &dyn PolyRef),
        }
    }

    #[inline]
    fn start(&self, unit: &impl CompilationUnit) -> Option<Site> {
        match self {
            Child::Token(child) => Some(child.chunk(unit)?.start()),
            Child::TokenSeq(child) => Some(child.first()?.chunk(unit)?.start()),
            Child::Node(child) => child.deref(unit)?.children().start(unit),
            Child::NodeSeq(child) => child.first()?.deref(unit)?.children().start(unit),
        }
    }

    #[inline]
    fn end(&self, unit: &impl CompilationUnit) -> Option<Site> {
        match self {
            Child::Token(child) => Some(child.chunk(unit)?.end()),
            Child::TokenSeq(child) => Some(child.last()?.chunk(unit)?.end()),
            Child::Node(child) => child.deref(unit)?.children().end(unit),
            Child::NodeSeq(child) => child.last()?.deref(unit)?.children().end(unit),
        }
    }
}

pub struct ChildIter<'a> {
    child: &'a Child,
    index: usize,
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = &'a dyn PolyRef;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.child.get(self.index)?;

        self.index += 1;

        Some(result)
    }
}

impl<'a> FusedIterator for ChildIter<'a> {}

pub struct ChildIntoIter {
    child: Child,
    index: usize,
}

impl Iterator for ChildIntoIter {
    type Item = PolyVariant;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.child.get(self.index)?;

        self.index += 1;

        Some(result.as_variant())
    }
}

impl FusedIterator for ChildIntoIter {}
