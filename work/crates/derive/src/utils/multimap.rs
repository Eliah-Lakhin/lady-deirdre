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

use crate::utils::{predictable::PredictableHasher, PredictableCollection, Set, SetImpl};

pub type Multimap<K, V> = HashMap<K, Set<V>, PredictableHasher>;

impl<Key, Value> MultimapImpl for Multimap<Key, Value> {
    type Key = Key;
    type Value = Value;

    fn fold<K>(self, mut map: impl FnMut(Self::Key) -> K) -> Multimap<K, Self::Value>
    where
        K: Eq + Hash,
        Self::Key: Eq + Hash,
        Self::Value: Eq + Hash + Clone,
    {
        let mut multimap = Multimap::empty();

        for (key, value) in self {
            let key = map(key);

            match multimap.get_mut(&key) {
                None => {
                    let _ = multimap.insert(key, value);
                }

                Some(accumulator) => accumulator.append(value),
            }
        }

        multimap
    }

    fn join<V>(self, mut join: impl FnMut(Self::Key, Self::Value) -> V) -> Set<V>
    where
        V: Eq + Hash,
        Self::Key: Eq + Hash + Clone,
    {
        let mut set = Set::empty();

        for (key, subset) in self {
            for value in subset {
                let _ = set.insert(join(key.clone(), value));
            }
        }

        set
    }
}

pub trait MultimapImpl {
    type Key;
    type Value;

    fn fold<K>(self, map: impl FnMut(Self::Key) -> K) -> Multimap<K, Self::Value>
    where
        K: Eq + Hash,
        Self::Key: Eq + Hash,
        Self::Value: Eq + Hash + Clone;

    fn join<V>(self, concatenation: impl FnMut(Self::Key, Self::Value) -> V) -> Set<V>
    where
        V: Eq + Hash,
        Self::Key: Eq + Hash + Clone;
}
