////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{collections::HashSet, hash::Hash};

use crate::utils::{predictable::PredictableHasher, Map, PredictableCollection};

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
    fn group<K, V>(self, mut division: impl FnMut(Self::Value) -> (K, V)) -> Map<K, Set<V>>
    where
        K: Eq + Hash,
        V: Eq + Hash + Clone,
    {
        let mut multimap = Map::empty();

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

    fn group<K, V>(self, division: impl FnMut(Self::Value) -> (K, V)) -> Map<K, Set<V>>
    where
        K: Eq + Hash,
        V: Eq + Hash + Clone;

    fn as_ref(&self) -> Set<&Self::Value>
    where
        Self::Value: Eq + Hash;
}
