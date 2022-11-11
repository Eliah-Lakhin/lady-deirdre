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

use std::{collections::HashSet, hash::Hash};

use crate::utils::{predictable::PredictableHasher, Multimap, PredictableCollection};

pub type Set<V> = HashSet<V, PredictableHasher>;

impl<V> PredictableCollection for Set<V> {
    #[inline(always)]
    fn empty() -> Self {
        Self::with_hasher(PredictableHasher)
    }

    #[inline(always)]
    fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, PredictableHasher)
    }
}

impl<Value> SetImpl for Set<Value> {
    type Value = Value;

    #[inline(always)]
    fn new<const N: usize>(array: [Self::Value; N]) -> Self
    where
        Self::Value: Eq + Hash,
    {
        Self::from_iter(array)
    }

    #[inline(always)]
    fn append(&mut self, other: Self)
    where
        Self::Value: Eq + Hash + Clone,
    {
        *self = HashSet::union(self, &other).cloned().collect()
    }

    #[inline(always)]
    fn merge(self, other: Self) -> Self
    where
        Self::Value: Eq + Hash + Clone,
    {
        HashSet::union(&self, &other).cloned().collect()
    }

    #[inline(always)]
    fn is_single(&self) -> bool {
        self.len() == 1
    }

    #[inline]
    fn single(&self) -> Option<Self::Value>
    where
        Self::Value: Clone,
    {
        if self.len() != 1 {
            return None;
        }

        self.iter().next().cloned()
    }

    #[inline]
    fn group<K, V>(self, mut division: impl FnMut(Self::Value) -> (K, V)) -> Multimap<K, V>
    where
        K: Eq + Hash,
        V: Eq + Hash + Clone,
    {
        let mut multimap = Multimap::empty();

        for value in self {
            let (key, value) = division(value);

            multimap
                .entry(key)
                .and_modify(|values: &mut Set<V>| {
                    let _ = values.insert(value.clone());
                })
                .or_insert_with(|| Set::new([value.clone()]));
        }

        multimap
    }

    #[inline]
    fn as_ref(&self) -> Set<&Self::Value>
    where
        Self::Value: Eq + Hash,
    {
        self.iter().collect()
    }
}

pub trait SetImpl {
    type Value;

    fn new<const N: usize>(array: [Self::Value; N]) -> Self
    where
        Self::Value: Eq + Hash;

    fn append(&mut self, other: Self)
    where
        Self::Value: Eq + Hash + Clone;

    fn merge(self, other: Self) -> Self
    where
        Self::Value: Eq + Hash + Clone;

    fn is_single(&self) -> bool;

    fn single(&self) -> Option<Self::Value>
    where
        Self::Value: Clone;

    fn group<K, V>(self, division: impl FnMut(Self::Value) -> (K, V)) -> Multimap<K, V>
    where
        K: Eq + Hash,
        V: Eq + Hash + Clone;

    fn as_ref(&self) -> Set<&Self::Value>
    where
        Self::Value: Eq + Hash;
}
