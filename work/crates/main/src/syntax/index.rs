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
pub type RuleIndex = u16;

pub static EMPTY_RULE_SET: RuleSet = RuleSet::empty();

/// A syntax grammar entry rule.
///
/// See [`syntax parser algorithm specification`](crate::syntax::Node::parse) for details.
pub static ROOT_RULE: RuleIndex = 0;

pub(crate) static NON_ROOT_RULE: RuleIndex = 1;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct RuleSet {
    vector: [RuleIndex; Self::LIMIT],
    occupied: usize,
}

impl Debug for RuleSet {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&self.vector[0..self.occupied], formatter)
    }
}

impl<'set> IntoIterator for &'set RuleSet {
    type Item = RuleIndex;
    type IntoIter = Copied<Iter<'set, RuleIndex>>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.vector[0..self.occupied].into_iter().copied()
    }
}

impl FromIterator<RuleIndex> for RuleSet {
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = RuleIndex>>(iter: I) -> Self {
        let mut result = Self::empty();

        for rule in iter {
            result = result.include(rule)
        }

        result
    }
}

impl RuleSet {
    pub const LIMIT: usize = 16;

    #[inline(always)]
    pub const fn empty() -> Self {
        Self {
            vector: [0; Self::LIMIT],
            occupied: 0,
        }
    }

    #[inline(always)]
    pub const fn new(rules: &[RuleIndex]) -> Self {
        Self::empty().include_all(rules)
    }

    #[inline(always)]
    pub const fn contains(&self, rule: RuleIndex) -> bool {
        let mut entry = 0;

        while entry < self.occupied {
            if self.vector[entry] == rule {
                return true;
            }

            entry += 1;
        }

        false
    }

    #[inline(always)]
    pub const fn include(mut self, rule: RuleIndex) -> Self {
        if self.contains(rule) {
            return self;
        }

        if self.occupied == Self::LIMIT {
            panic!("Too many rules in the rule set.");
        }

        self.vector[self.occupied] = rule;
        self.occupied += 1;

        self
    }

    #[inline(always)]
    pub const fn include_all(mut self, rules: &[RuleIndex]) -> Self {
        let mut slice_index = 0;

        while slice_index < rules.len() {
            self = self.include(rules[slice_index]);
            slice_index += 1;
        }

        self
    }

    #[inline(always)]
    pub const fn exclude(mut self, rule: RuleIndex) -> Self {
        let mut entry = 0;

        while entry < self.occupied {
            if self.vector[entry] == rule {
                entry += 1;
                while entry < self.occupied {
                    self.vector[entry - 1] = self.vector[entry];
                    entry += 1;
                }

                self.occupied -= 1;

                break;
            }

            entry += 1;
        }

        self
    }

    #[inline(always)]
    pub const fn exclude_all(mut self, rules: &[RuleIndex]) -> Self {
        let mut slice_index = 0;

        while slice_index < rules.len() {
            self = self.exclude(rules[slice_index]);
            slice_index += 1;
        }

        self
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.occupied == 0
    }

    #[inline(always)]
    pub const fn length(&self) -> usize {
        self.occupied
    }

    #[inline(always)]
    pub fn display<N: Node>(&self) -> impl Display + '_ {
        pub struct DisplayRuleSet<'set, N> {
            set: &'set RuleSet,
            _token: PhantomData<N>,
        }

        impl<'set, N: Node> Display for DisplayRuleSet<'set, N> {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                let mut vector = Vec::with_capacity(RuleSet::LIMIT);

                for rule in self.set {
                    if let Some(description) = N::describe(rule) {
                        vector.push(description);
                    }
                }

                vector.sort();

                formatter.debug_set().entries(vector).finish()
            }
        }

        DisplayRuleSet {
            set: self,
            _token: PhantomData::<N>,
        }
    }
}
