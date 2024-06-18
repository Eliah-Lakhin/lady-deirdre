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

use crate::units::storage::{
    child::{ChildCount, ChildIndex},
    item::Item,
};

#[derive(Debug)]
pub(super) struct Spread {
    pub(super) head: ChildCount,
    pub(super) tail: ChildCount,
    pub(super) items: ChildCount,
    next: ChildIndex,
}

impl Spread {
    #[inline(always)]
    pub(super) const fn new<I: Item>(total: ChildCount) -> Spread {
        if total <= I::CAP {
            return Spread {
                head: 1,
                tail: 0,
                items: total,
                next: 0,
            };
        }

        let branch_count = total / I::B;
        let reminder = total - branch_count * I::B;
        let reminder_spread = reminder / branch_count;
        let items = I::B + reminder_spread;
        let tail = reminder - reminder_spread * branch_count;

        Spread {
            head: branch_count - tail,
            tail,
            items,
            next: 0,
        }
    }

    #[inline(always)]
    pub(super) const fn layer_size(&self) -> ChildCount {
        self.head + self.tail
    }

    #[inline(always)]
    pub(super) const fn total_items(&self) -> ChildCount {
        self.head * self.items + self.tail * (self.items + 1)
    }

    #[inline(always)]
    pub(super) fn advance(&mut self) -> ChildIndex {
        if self.next < self.items {
            self.next += 1;

            return self.next - 1;
        }

        self.next = 1;

        if self.head > 0 {
            self.head -= 1;

            if self.head == 0 {
                if self.tail == 0 {
                    return ChildIndex::MAX;
                }

                self.items += 1;
            }
        } else {
            self.tail -= 1;

            if self.tail == 0 {
                return ChildIndex::MAX;
            }
        }

        0
    }
}

#[inline(always)]
pub(super) const fn capacity(branching: ChildCount) -> ChildCount {
    branching * 2 - 1
}
