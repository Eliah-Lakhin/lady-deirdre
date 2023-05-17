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
    incremental::storage::{
        child::{ChildCount, ChildIndex},
        item::Item,
    },
    report::{debug_assert, debug_assert_ne},
    std::*,
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
        if total <= capacity(I::BRANCHING) {
            return Spread {
                head: 1,
                tail: 0,
                items: total,
                next: 0,
            };
        }

        let branch_count = total / I::BRANCHING;
        let reminder = total - branch_count * I::BRANCHING;
        let reminder_spread = reminder / branch_count;
        let items = I::BRANCHING + reminder_spread;
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

//Safety:
// 1. `from` and `to` are two distinct arrays.
// 2. The `from` data within `source..(source + count)` range is within N bounds.
// 3. The `to` data within `destination..(destination + count)` range is is within N bounds.
#[inline(always)]
pub(super) unsafe fn array_copy_to<const N: usize, T: Sized>(
    from: &mut [T; N],
    to: &mut [T; N],
    source: ChildCount,
    destination: ChildCount,
    count: ChildCount,
) {
    debug_assert_ne!(
        from.as_mut_ptr(),
        to.as_mut_ptr(),
        "Array copy overlapping."
    );
    debug_assert!(source + count <= N, "Source range exceeds capacity.");
    debug_assert!(
        destination + count <= N,
        "Destination range exceeds capacity.",
    );

    let from = unsafe { from.as_mut_ptr().offset(source as isize) };
    let to = unsafe { to.as_mut_ptr().offset(destination as isize) };

    unsafe { copy_nonoverlapping(from, to, count) };
}

//Safety:
// 1. `from + count <= N`.
// 1. `from + to <= N`.
// 2. `count > 0`.
#[inline(always)]
pub(super) unsafe fn array_shift<const N: usize, T: Sized>(
    array: &mut [T; N],
    from: ChildCount,
    to: ChildCount,
    count: ChildCount,
) {
    debug_assert!(from + count <= N, "Shift with overflow.");
    debug_assert!(to + count <= N, "Shift with overflow.");
    debug_assert!(count > 0, "Empty shift range.");

    let array_ptr = array.as_mut_ptr();
    let source = unsafe { array_ptr.offset(from as isize) };
    let destination = unsafe { array_ptr.offset(to as isize) };

    match from + count <= to || to + count <= from {
        false => unsafe { copy(source, destination, count) },
        true => unsafe { copy_nonoverlapping(source, destination, count) },
    }
}
