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
    std::*,
    units::storage::{
        child::{ChildCount, ChildIndex},
        item::Item,
    },
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
