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

use crate::{std::*, syntax::Node};

/// A static identifier of arbitrary syntax grammar rule.
///
/// The exact values of this type are uniquely specified by the particular
/// [`syntax parsing algorithm`](crate::syntax::Node::parse) except the [ROOT_RULE] that is always
/// specifies grammar's an entry rule.
pub type NodeRule = u16;

pub static EMPTY_RULE_SET: NodeSet = NodeSet::empty();

/// A syntax grammar entry rule.
///
/// See [`syntax parser algorithm specification`](crate::syntax::Node::parse) for details.
pub const ROOT_RULE: NodeRule = 0;

pub const NON_RULE: NodeRule = NodeRule::MAX;

#[repr(transparent)]
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct NodeSet {
    vector: [NodeRule; Self::LIMIT],
}

impl Debug for NodeSet {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        let mut debug_list = formatter.debug_list();

        let mut entry = 0;

        while entry < Self::LIMIT {
            let probe = &self.vector[entry];

            if probe == &NON_RULE {
                break;
            }

            debug_list.entry(probe);

            entry += 1;
        }

        debug_list.finish()
    }
}

impl<'set> IntoIterator for &'set NodeSet {
    type Item = NodeRule;
    type IntoIter = NodeSetIter<'set>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        NodeSetIter { set: self, next: 0 }
    }
}

impl FromIterator<NodeRule> for NodeSet {
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = NodeRule>>(iter: I) -> Self {
        let mut result = Self::empty();

        for rule in iter {
            result = result.include(rule)
        }

        result
    }
}

impl NodeSet {
    pub const LIMIT: usize = 16;

    #[inline(always)]
    pub const fn empty() -> Self {
        Self {
            vector: [NON_RULE; Self::LIMIT],
        }
    }

    #[inline(always)]
    pub const fn new(rules: &[NodeRule]) -> Self {
        Self::empty().include_all(rules)
    }

    #[inline(always)]
    pub const fn contains(&self, rule: NodeRule) -> bool {
        if rule == NON_RULE {
            return false;
        }

        let mut entry = 0;

        while entry < Self::LIMIT {
            let probe = self.vector[entry];

            if probe == NON_RULE {
                break;
            }

            if probe == rule {
                return true;
            }

            entry += 1;
        }

        false
    }

    #[inline(always)]
    pub const fn include(mut self, mut rule: NodeRule) -> Self {
        if rule == NON_RULE {
            panic!("Non-rule cannot be inserted into the rule set.");
        }

        let mut entry = 0;

        while entry < Self::LIMIT {
            let mut probe = self.vector[entry];

            if probe == rule {
                return self;
            }

            if probe > rule {
                while entry < Self::LIMIT {
                    probe = self.vector[entry];
                    self.vector[entry] = rule;

                    if probe == NON_RULE {
                        return self;
                    }

                    rule = probe;
                    entry += 1;
                }

                break;
            }

            entry += 1;
        }

        panic!("Too many rules in the rule set.");
    }

    #[inline(always)]
    pub const fn include_all(mut self, rules: &[NodeRule]) -> Self {
        let mut slice_index = 0;

        while slice_index < rules.len() {
            self = self.include(rules[slice_index]);
            slice_index += 1;
        }

        self
    }

    #[inline(always)]
    pub const fn exclude(mut self, rule: NodeRule) -> Self {
        if rule == NON_RULE {
            return self;
        }

        let mut entry = 0;

        while entry < Self::LIMIT {
            let mut probe = self.vector[entry];

            if probe > rule {
                break;
            }

            if probe == rule {
                loop {
                    let next = entry + 1;

                    probe = match next < Self::LIMIT {
                        true => self.vector[next],
                        false => NON_RULE,
                    };

                    self.vector[entry] = probe;

                    if probe == NON_RULE {
                        break;
                    }

                    entry = next;
                }

                break;
            }

            entry += 1;
        }

        self
    }

    #[inline(always)]
    pub const fn exclude_all(mut self, rules: &[NodeRule]) -> Self {
        let mut slice_index = 0;

        while slice_index < rules.len() {
            self = self.exclude(rules[slice_index]);
            slice_index += 1;
        }

        self
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.vector[0] == NON_RULE
    }

    #[inline(always)]
    pub const fn length(&self) -> usize {
        let mut length = 0;

        while length < Self::LIMIT {
            if self.vector[length] == NON_RULE {
                break;
            }

            length += 1;
        }

        length
    }

    #[inline(always)]
    pub fn display<N: Node>(&self) -> impl Display + '_ {
        pub struct DisplayNodeSet<'set, N> {
            set: &'set NodeSet,
            _token: PhantomData<N>,
        }

        impl<'set, N: Node> Display for DisplayNodeSet<'set, N> {
            fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
                let mut vector = Vec::with_capacity(NodeSet::LIMIT);

                for rule in self.set {
                    if let Some(description) = N::name(rule) {
                        vector.push(description);
                    }
                }

                vector.sort();

                formatter.debug_set().entries(vector).finish()
            }
        }

        DisplayNodeSet {
            set: self,
            _token: PhantomData::<N>,
        }
    }
}

pub struct NodeSetIter<'set> {
    set: &'set NodeSet,
    next: usize,
}

impl<'set> Iterator for NodeSetIter<'set> {
    type Item = NodeRule;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == NodeSet::LIMIT {
            return None;
        }

        let probe = self.set.vector[self.next];

        if probe == NON_RULE {
            return None;
        }

        self.next += 1;

        Some(probe)
    }
}

impl<'set> FusedIterator for NodeSetIter<'set> {}
