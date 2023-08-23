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
    lexis::{Token, TokenRule, TokenSet, EOI},
    std::*,
    syntax::SyntaxSession,
};

pub static UNLIMITED_RECOVERY: Recovery = Recovery::unlimited();

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct Recovery {
    groups: [(TokenRule, TokenRule); Self::GROUPS_LIMIT as usize],
    groups_len: u8,
    unexpected: TokenSet,
}

impl Recovery {
    pub const GROUPS_LIMIT: u8 = 4;

    #[inline(always)]
    pub const fn unlimited() -> Self {
        Self {
            groups: [(0, 0); Self::GROUPS_LIMIT as usize],
            groups_len: 0,
            unexpected: TokenSet::empty(),
        }
    }

    #[inline(always)]
    pub const fn group(mut self, open: TokenRule, close: TokenRule) -> Self {
        if open == close {
            panic!("Group open and close tokens must be different.");
        }

        let mut group_id = 0;
        while group_id < self.groups_len {
            let (other_open, other_close) = self.groups[group_id as usize];

            if other_open == open {
                panic!("Duplicate group open token.")
            }

            if other_close == open {
                panic!("Duplicate group open token.")
            }

            if other_open == close {
                panic!("Duplicate group close token.")
            }

            if other_close == close {
                panic!("Duplicate group close token.")
            }

            group_id += 1;
        }

        if self.groups_len >= Self::GROUPS_LIMIT {
            panic!("Groups limit exceeded.");
        }

        self.groups[self.groups_len as usize] = (open, close);
        self.groups_len += 1;

        self
    }

    #[inline(always)]
    pub const fn unexpected(mut self, rule: TokenRule) -> Self {
        self.unexpected = self.unexpected.include(rule);

        self
    }

    #[inline(always)]
    pub const fn unexpected_set(mut self, set: TokenSet) -> Self {
        self.unexpected = self.unexpected.union(set);

        self
    }

    #[inline]
    pub fn recover<'code>(
        &self,
        session: &mut impl SyntaxSession<'code>,
        expectations: &TokenSet,
    ) -> bool {
        let mut stack = GroupStack::new();

        loop {
            let rule = session.token(0).rule();

            if expectations.contains(rule) {
                return true;
            }

            if self.unexpected.contains(rule) {
                return false;
            }

            let mut group_id = 0u8;
            while group_id < self.groups_len {
                let open = self.groups[group_id as usize].0;

                if open == rule {
                    self.try_skip_group(session, &mut stack, group_id);
                }

                group_id += 1;
            }

            if !session.advance() {
                return false;
            }
        }
    }

    #[inline(always)]
    fn try_skip_group<'code>(
        &self,
        session: &mut impl SyntaxSession<'code>,
        stack: &mut GroupStack,
        mut group_id: u8,
    ) {
        stack.clear();
        stack.push(group_id);

        let mut distance = 0;

        'outer: loop {
            distance += 1;

            let rule = session.token(distance).rule();

            if rule == EOI {
                break;
            }

            group_id = 0;
            while group_id < self.groups_len {
                let (open, close) = self.groups[group_id as usize];

                if open == rule {
                    stack.push(group_id);
                    break;
                }

                if close == rule {
                    let id = match stack.pop() {
                        None => break 'outer,
                        Some(id) => id,
                    };

                    if id != group_id {
                        stack.push(id);
                        break 'outer;
                    }

                    if stack.is_empty() {
                        break 'outer;
                    }

                    break;
                }

                group_id += 1;
            }
        }

        if stack.is_empty() {
            session.skip(distance);
        }
    }
}

enum GroupStack {
    Inline {
        vec: [u8; Self::INLINE],
        length: usize,
    },
    Heap {
        vec: Vec<u8>,
    },
}

impl GroupStack {
    const INLINE: usize = 8;

    #[inline(always)]
    fn new() -> Self {
        Self::Inline {
            vec: [0; Self::INLINE],
            length: 0,
        }
    }

    #[inline(always)]
    fn push(&mut self, index: u8) {
        match self {
            Self::Inline { vec, length } => {
                if *length < Self::INLINE {
                    vec[*length] = index;
                    *length += 1;
                    return;
                }

                let mut vec = Vec::from(*vec);
                vec.push(index);

                *self = Self::Heap { vec }
            }

            Self::Heap { vec } => vec.push(index),
        }
    }

    #[inline(always)]
    fn pop(&mut self) -> Option<u8> {
        match self {
            Self::Inline { vec, length } => match *length > 0 {
                true => {
                    *length -= 1;

                    let item = vec[*length];

                    Some(item)
                }

                false => None,
            },

            Self::Heap { vec } => vec.pop(),
        }
    }

    #[inline(always)]
    fn clear(&mut self) {
        match self {
            Self::Inline { length, .. } => *length = 0,
            Self::Heap { vec } => vec.clear(),
        }
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        match self {
            Self::Inline { length, .. } => *length == 0,
            Self::Heap { vec } => vec.is_empty(),
        }
    }
}
