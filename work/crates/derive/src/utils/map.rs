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

use std::{collections::HashMap, hash::Hash};

use syn::Result;

use crate::utils::{predictable::PredictableHasher, PredictableCollection};

pub type Map<K, V> = HashMap<K, V, PredictableHasher>;

impl<K, V> PredictableCollection for Map<K, V> {
    #[inline(always)]
    fn empty() -> Self {
        Self::with_hasher(PredictableHasher)
    }

    #[inline(always)]
    fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, PredictableHasher)
    }
}

impl<Key, Value> MapImpl for Map<Key, Value> {
    type Key = Key;
    type Value = Value;

    #[inline(always)]
    fn new<const N: usize>(array: [(Self::Key, Self::Value); N]) -> Self
    where
        Self::Key: Eq + Hash,
    {
        Self::from_iter(array)
    }

    #[inline(always)]
    fn append(&mut self, other: Self)
    where
        Self::Key: Eq + Hash,
    {
        for (key, value) in other {
            assert!(
                self.insert(key, value).is_none(),
                "Internal error. Duplicate keys in append."
            );
        }
    }

    #[inline(always)]
    fn single_key(&self) -> Option<&Self::Key> {
        if self.len() != 1 {
            return None;
        }

        self.keys().next()
    }

    #[inline]
    fn for_each(mut self, mut iterator: impl FnMut(&Self::Key, &mut Self::Value)) -> Self {
        for (key, value) in &mut self {
            iterator(key, value);
        }

        self
    }

    fn try_for_each(
        mut self,
        mut iterator: impl FnMut(&Self::Key, &mut Self::Value) -> Result<()>,
    ) -> Result<Self> {
        for (key, value) in &mut self {
            iterator(key, value)?;
        }

        Ok(self)
    }
}

pub trait MapImpl {
    type Key;
    type Value;

    fn new<const N: usize>(array: [(Self::Key, Self::Value); N]) -> Self
    where
        Self::Key: Eq + Hash;

    fn append(&mut self, other: Self)
    where
        Self::Key: Eq + Hash;

    fn single_key(&self) -> Option<&Self::Key>;

    fn for_each(self, iterator: impl FnMut(&Self::Key, &mut Self::Value)) -> Self;

    fn try_for_each(
        self,
        iterator: impl FnMut(&Self::Key, &mut Self::Value) -> Result<()>,
    ) -> Result<Self>
    where
        Self: Sized;
}
