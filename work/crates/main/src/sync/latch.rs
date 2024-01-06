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

use crate::{report::system_panic, std::*};

const CHECK_MASK: usize = 1usize;
const REF_MASK: usize = usize::MAX ^ CHECK_MASK;
const REF_MAX: usize = REF_MASK / 2;
const REF_STEP: usize = 1 << 1;

#[repr(transparent)]
pub struct Latch {
    data: NonNull<AtomicUsize>,
}

// Safety: Latch's data access is guarded by the atomic operations.
unsafe impl Send for Latch {}

// Safety: Latch's data access is guarded by the atomic operations.
unsafe impl Sync for Latch {}

impl Debug for Latch {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&self.get(), formatter)
    }
}

impl PartialEq for Latch {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.addr().eq(&other.addr())
    }
}

impl Eq for Latch {}

impl Hash for Latch {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.addr().hash(state)
    }
}

impl Clone for Latch {
    fn clone(&self) -> Self {
        let value = {
            // Safety: Latch owns a pointer to valid data leaked from the Box.
            let state = unsafe { self.data.as_ref() };

            state.fetch_add(REF_STEP, AtomicOrdering::Relaxed)
        };

        if value & REF_MASK > REF_MAX {
            system_panic!("Too many Latch references.");
        }

        Self { data: self.data }
    }
}

impl Drop for Latch {
    fn drop(&mut self) {
        let value = {
            // Safety: Latch owns a pointer to valid data leaked from the Box.
            let state = unsafe { self.data.as_ref() };

            state.fetch_sub(REF_STEP, AtomicOrdering::Release)
        };

        if value & REF_MASK == REF_STEP {
            fence(AtomicOrdering::Acquire);

            // Safety:
            //   1. Latch owns a pointer to valid data leaked from the Box.
            //   2. The drop operation is ordered by the Acquire fence.
            let _ = unsafe { Box::from_raw(self.data.as_ptr()) };
        }
    }
}

impl Latch {
    pub fn new() -> Self {
        let data = Box::into_raw(Box::new(AtomicUsize::new(REF_STEP)));

        // Safety: Box leaked pointer is never null.
        let data = unsafe { NonNull::new_unchecked(data) };

        Self { data }
    }

    //todo consider renaming
    pub fn get(&self) -> bool {
        // Safety: Latch owns a pointer to valid data leaked from the Box.
        let state = unsafe { self.data.as_ref() };

        let value = state.load(AtomicOrdering::Acquire);

        value & CHECK_MASK == CHECK_MASK
    }

    //todo consider renaming
    pub fn get_relaxed(&self) -> bool {
        // Safety: Latch owns a pointer to valid data leaked from the Box.
        let state = unsafe { self.data.as_ref() };

        let value = state.load(AtomicOrdering::Relaxed);

        value & CHECK_MASK == CHECK_MASK
    }

    //todo consider renaming
    pub fn set(&self) {
        // Safety: Latch owns a pointer to valid data leaked from the Box.
        let state = unsafe { self.data.as_ref() };

        state.fetch_or(CHECK_MASK, AtomicOrdering::Release);
    }

    #[inline(always)]
    pub fn addr(&self) -> usize {
        self.data.as_ptr() as usize
    }
}

#[cfg(test)]
mod tests {
    use crate::{std::*, sync::Latch};

    #[test]
    fn test_latch() {
        let latch = Latch::new();

        assert!(!latch.get());

        let latch2 = latch.clone();

        latch.set();

        drop(latch);

        assert!(latch2.get());
    }
}
