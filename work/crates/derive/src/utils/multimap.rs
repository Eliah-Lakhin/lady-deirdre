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
