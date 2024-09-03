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
