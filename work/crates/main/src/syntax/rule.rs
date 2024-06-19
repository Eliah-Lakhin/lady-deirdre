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
    fmt::{Debug, Display, Formatter},
    iter::FusedIterator,
    marker::PhantomData,
};

use crate::syntax::AbstractNode;

/// A numeric type that denotes a syntax parsing rule within the programming
/// language grammar.
///
/// See the [Parse rules](crate::syntax::SyntaxSession#parse-rules) section of
/// the parsing process specification for details.
pub type NodeRule = u16;

/// A static set of the syntax node rules without entries.
///
/// The value of this static equals to the [NodeSet::empty] value.
pub static EMPTY_NODE_SET: NodeSet = NodeSet::empty();

/// Denotes a syntax parse rule of the root node of the syntax tree.
///
/// See the [Parse rules](crate::syntax::SyntaxSession#parse-rules) section of
/// the parsing process specification for details.
pub const ROOT_RULE: NodeRule = 0;

/// Denotes an invalid syntax parse rule.
///
/// This number does not belong to any syntax parse rule of any
/// programming language.
///
/// See the [Parse rules](crate::syntax::SyntaxSession#parse-rules) section of
/// the parsing process specification for details.
pub const NON_RULE: NodeRule = NodeRule::MAX;

/// A set of syntax parse [rules](NodeRule) of fixed size.
///
/// The set stores all entries in place, and the set object has a fixed size.
///
/// The maximum number of rules the object could store is [NodeSet::LIMIT].
///
/// Most methods of this object are the const functions. Some of them take up to
/// `O(LIMIT)` and `O(LIMIT^2)` time to perform.
///
/// The object is assumed to be constructed in a const context as a static value
/// upfront to reduce the runtime overhead.
#[repr(transparent)]
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct NodeSet {
    vector: [NodeRule; Self::LIMIT],
}

impl Debug for NodeSet {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
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
    /// The maximum number of entries this set can address.
    ///
    /// This number may be increased in future minor versions of Lady Deirdre.
    pub const LIMIT: usize = 16;

    /// Creates a node set without entries.
    ///
    /// If you need just a static empty node set, use the predefined
    /// [EMPTY_NODE_SET] static.
    #[inline(always)]
    pub const fn empty() -> Self {
        Self {
            vector: [NON_RULE; Self::LIMIT],
        }
    }

    /// Constructs a node set from the slice of the node rules.
    ///
    /// **Panic**
    ///
    /// Panics if the `rules` parameter has more than the [LIMIT](Self::LIMIT)
    /// number of unique node rules.
    ///
    /// Panics if any value within the `rules` slice is a [NON_RULE].
    #[inline(always)]
    pub const fn new(rules: &[NodeRule]) -> Self {
        Self::empty().include_all(rules)
    }

    /// Returns true if the node set contains the specified node `rule`.
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

    /// Consumes this NodeSet instance and returns a new node set that
    /// includes the `rule` node rule.
    ///
    /// **Panic**
    ///
    /// Panics if the `rule` argument is a [NON_RULE].
    ///
    /// Panics if the node set already has a [LIMIT](Self::LIMIT) number of
    /// unique entries, and the `rule` argument is a new entry within this set.
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

    /// Consumes this NodeSet instance and returns a new node set that
    /// includes all node rules from the `rules` slice.
    ///
    /// **Panic**
    ///
    /// Panics if any rule number within the slice argument is a [NON_RULE].
    ///
    /// Panics if the total number if unique entries within this node set and
    /// the rules from the `rules` slice exceeds [LIMIT](Self::LIMIT).
    #[inline(always)]
    pub const fn include_all(mut self, rules: &[NodeRule]) -> Self {
        let mut slice_index = 0;

        while slice_index < rules.len() {
            self = self.include(rules[slice_index]);
            slice_index += 1;
        }

        self
    }

    /// Consumes this NodeSet instance and returns a new node set without
    /// the specified `rule` node rule.
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

    /// Consumes this NodeSet instance and returns a new node set without
    /// the entries specified in the `rules` node rule slice.
    #[inline(always)]
    pub const fn exclude_all(mut self, rules: &[NodeRule]) -> Self {
        let mut slice_index = 0;

        while slice_index < rules.len() {
            self = self.exclude(rules[slice_index]);
            slice_index += 1;
        }

        self
    }

    /// Returns true if the NodeSet has no entries.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.vector[0] == NON_RULE
    }

    /// Returns the number of entries in this NodeSet instance.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        let mut length = 0;

        while length < Self::LIMIT {
            if self.vector[length] == NON_RULE {
                break;
            }

            length += 1;
        }

        length
    }

    /// Returns an object that displays all entries within this node set.
    ///
    /// The `N` generic parameter specifies the syntax grammar of
    /// the programming language (see [Node](crate::syntax::Node)).
    ///
    /// The underlying displaying algorithm uses
    /// the [rule_name](AbstractNode::rule_name) function to determine
    /// the rules' display names.
    #[inline(always)]
    pub fn display<N: AbstractNode>(&self) -> impl Debug + Display + '_ {
        pub struct DisplayNodeSet<'set, N> {
            set: &'set NodeSet,
            _token: PhantomData<N>,
        }

        impl<'set, N: AbstractNode> Debug for DisplayNodeSet<'set, N> {
            #[inline(always)]
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                Display::fmt(self, formatter)
            }
        }

        impl<'set, N: AbstractNode> Display for DisplayNodeSet<'set, N> {
            fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
                let mut vector = Vec::with_capacity(NodeSet::LIMIT);

                for rule in self.set {
                    if let Some(name) = N::rule_name(rule) {
                        vector.push(name);
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
