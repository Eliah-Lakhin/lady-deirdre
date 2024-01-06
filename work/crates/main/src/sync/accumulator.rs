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

//todo consider removing this module

use crate::{report::debug_unreachable, std::*, sync::Shared};

pub struct Accumulator<T, S = RandomState> {
    inner: Mutex<(bool, Shared<HashSet<T, S>>)>,
}

impl<T: Hash + Eq + Clone, S: BuildHasher + Clone> Accumulator<T, S> {
    #[inline(always)]
    pub fn new() -> Self
    where
        S: Default,
    {
        Self::with_capacity(0)
    }

    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self
    where
        S: Default,
    {
        Self::with_capacity_and_hasher(capacity, S::default())
    }

    #[inline(always)]
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        Self {
            inner: Mutex::new((
                false,
                Shared::new(HashSet::with_capacity_and_hasher(capacity, hasher)),
            )),
        }
    }

    pub fn insert(&self, item: &T) -> bool {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        if guard.1.as_ref().contains(item) {
            return false;
        }

        if !guard.1.make_mut().insert(item.clone()) {
            // Safety: Existence checked above.
            unsafe { debug_unreachable!("Hash set inconsistency.") }
        }

        guard.0 = true;

        true
    }

    pub fn remove(&self, item: &T) -> bool {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        if !guard.1.as_ref().contains(item) {
            return false;
        }

        if !guard.1.make_mut().remove(item) {
            // Safety: Existence checked above.
            unsafe { debug_unreachable!("Hash set inconsistency.") }
        }

        guard.0 = true;

        true
    }

    #[inline(always)]
    pub fn snapshot(&self) -> Shared<HashSet<T, S>> {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        guard.1.clone()
    }

    #[inline(always)]
    pub fn commit(&self) -> bool {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        replace(&mut guard.0, false)
    }
}
