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

///////////////////////////////////////////////////////////////////////////////////////
// A part of this file's source code is an adaptation of the Joel Wejdenstål's       //
// "DashMap" work.                                                                   //
//                                                                                   //
// Joel Wejdenstål's original work available here:                                   //
// https://github.com/xacrimon/dashmap/tree/626b98dab3c124cd9cd4960d0306da5d65918dfc //
//                                                                                   //
// Joel Wejdenstål provided his work under the following terms:                      //
//                                                                                   //
//   MIT License                                                                     //
//                                                                                   //
//   Copyright (c) 2019 Acrimon                                                      //
//                                                                                   //
//   Permission is hereby granted, free of charge, to any person obtaining a copy    //
//   of this software and associated documentation files (the "Software"), to deal   //
//   in the Software without restriction, including without limitation the rights    //
//   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell       //
//   copies of the Software, and to permit persons to whom the Software is           //
//   furnished to do so, subject to the following conditions:                        //
//                                                                                   //
//   The above copyright notice and this permission notice shall be included in all  //
//   copies or substantial portions of the Software.                                 //
//                                                                                   //
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR      //
//   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,        //
//   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE     //
//   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER          //
//   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,   //
//   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE   //
//   SOFTWARE.                                                                       //
//                                                                                   //
// Kindly be advised that the terms governing the distribution of my work are        //
// distinct from those pertaining to the original work of Joel Wejdenstål.           //
///////////////////////////////////////////////////////////////////////////////////////

use crate::{report::debug_unreachable, std::*};

pub struct Table<K, V, S = RandomState> {
    shift: usize,
    shards: Box<[RwLock<HashMap<K, V, S>>]>,
    hasher: S,
}

impl<K, V, S> IntoIterator for Table<K, V, S> {
    type Item = (K, V);
    type IntoIter = TableIntoIter<K, V, S>;

    fn into_iter(self) -> Self::IntoIter {
        let mut shards = Vec::from(self.shards).into_iter();

        let probe = match shards.next() {
            Some(probe) => probe
                .into_inner()
                .unwrap_or_else(|poison| poison.into_inner())
                .into_iter(),

            // Safety: `shards` array is never empty.
            None => unsafe { debug_unreachable!("Empty shards array.") },
        };

        TableIntoIter { probe, shards }
    }
}

impl<K: Hash + Eq, V, S: BuildHasher + Default + Clone> Default for Table<K, V, S> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> Table<K, V, S> {
    #[inline(always)]
    pub fn new() -> Self
    where
        S: Default + Clone,
    {
        Self::with_capacity(0)
    }

    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self
    where
        S: Default + Clone,
    {
        Self::with_capacity_and_hasher(capacity, S::default())
    }

    #[inline(always)]
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self
    where
        S: Clone,
    {
        Self::with_capacity_and_hasher_and_shards(capacity, hasher, shards_amount())
    }

    pub fn with_capacity_and_hasher_and_shards(capacity: usize, hasher: S, shards: usize) -> Self
    where
        S: Clone,
    {
        if !shards.is_power_of_two() {
            panic!("Table shards amount {shards} is not a power of two.");
        }

        let shard_capacity = ((capacity + shards - 1) & !(shards - 1)) / shards;

        let shift = match shards > 1 {
            true => size_of::<usize>() * 8 - shards.trailing_zeros() as usize,
            false => 0,
        };

        let shards = (0..shards)
            .map(|_| {
                RwLock::new(HashMap::with_capacity_and_hasher(
                    shard_capacity,
                    hasher.clone(),
                ))
            })
            .collect();

        Self {
            shift,
            shards,
            hasher,
        }
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let shard = self.shard_of(key);

        let guard = shard.read().unwrap_or_else(|poison| poison.into_inner());

        guard.contains_key(key)
    }

    pub fn get<Q>(&self, key: &Q) -> Option<TableReadGuard<K, V, S>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let shard = self.shard_of(key);

        let guard = shard.read().unwrap_or_else(|poison| poison.into_inner());

        let value = guard.get(key)?;

        // Safety:
        //   Prolongs reference lifetime to `self` lifetime.
        //   The value will be valid for as long as the guard is held.
        let value = unsafe { transmute::<&V, &V>(value) };

        Some(TableReadGuard {
            value,
            _guard: guard,
        })
    }

    pub fn try_get<Q>(&self, key: &Q) -> Option<TableReadGuard<K, V, S>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let shard = self.shard_of(key);

        let guard = match shard.try_read() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(poison)) => poison.into_inner(),
            Err(TryLockError::WouldBlock) => return None,
        };

        let value = guard.get(key)?;

        // Safety:
        //   Prolongs reference lifetime to `self` lifetime.
        //   The value will be valid for as long as the guard is held.
        let value = unsafe { transmute::<&V, &V>(value) };

        Some(TableReadGuard {
            value,
            _guard: guard,
        })
    }

    pub fn get_mut<Q>(&self, key: &Q) -> Option<TableWriteGuard<K, V, S>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let shard = self.shard_of(key);

        let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

        let value = guard.get_mut(key)?;

        // Safety:
        //   Prolongs reference lifetime to `self` lifetime.
        //   The value will be valid for as long as the guard is held.
        let value = unsafe { transmute::<&mut V, &mut V>(value) };

        Some(TableWriteGuard {
            value,
            _guard: guard,
        })
    }

    pub fn entry(&self, key: K) -> TableEntry<K, V, S> {
        let shard = self.shard_of(&key);

        let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

        let entry = guard.entry(key);

        // Safety:
        //   Prolongs reference lifetime to `self` lifetime.
        //   The value will be valid for as long as the guard is held.
        let entry = unsafe { transmute::<HashMapEntry<'_, K, V>, HashMapEntry<'_, K, V>>(entry) };

        match entry {
            HashMapEntry::Occupied(entry) => {
                TableEntry::Occupied(TableOccupiedEntry { entry, guard })
            }
            HashMapEntry::Vacant(entry) => TableEntry::Vacant(TableVacantEntry { entry, guard }),
        }
    }

    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let shard = self.shard_of(&key);

        let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

        guard.insert(key, value)
    }

    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let shard = self.shard_of(key);

        let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

        guard.remove(key)
    }

    pub fn remove_entry<Q>(&self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let shard = self.shard_of(key);

        let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

        guard.remove_entry(key)
    }

    pub fn drain(&self) -> TableDrain<'_, K, V, S> {
        let mut guard = match self.shards.first() {
            Some(shard) => shard.write().unwrap_or_else(|poison| poison.into_inner()),

            // Safety: `shards` array is never empty.
            None => unsafe { debug_unreachable!("Empty shards array.") },
        };

        let mut probes = Vec::with_capacity(self.shards.len());

        let drain = guard.drain();

        // Safety:
        //   Prolongs reference lifetime to `self` lifetime.
        //   The value will be valid for as long as the guard is held.
        let drain = unsafe { transmute::<Drain<'_, K, V>, Drain<'_, K, V>>(drain) };

        probes.push(ProbeDrain {
            drain,
            _guard: guard,
        });

        TableDrain {
            probes,
            table: self,
        }
    }

    pub fn retain<F>(&self, mut f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let mut guards = Vec::with_capacity(self.shards.len());

        for shard in self.shards.iter() {
            let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

            guard.retain(&mut f);

            guards.push(guard);
        }
    }

    pub fn clear(&self) {
        let mut guards = Vec::with_capacity(self.shards.len());

        for shard in self.shards.iter() {
            let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

            guard.clear();

            guards.push(guard);
        }
    }

    pub fn shrink_to_fit(&self) {
        for shard in self.shards.iter() {
            let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

            guard.shrink_to_fit();
        }
    }

    pub fn hasher(&self) -> &S {
        &self.hasher
    }

    #[inline(always)]
    pub fn shard_index_of<Q>(&self, key: &Q) -> usize
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.shards.len() == 1 {
            return 0;
        }

        let mut hasher = self.hasher.build_hasher();

        key.hash(&mut hasher);

        let hash = hasher.finish() as usize;

        let shard = (hash << 7) >> self.shift;

        if shard >= self.shards.len() {
            // Safety: Hash is uniform in the shards space.
            unsafe {
                debug_unreachable!("Table shard index out of bounds.");
            }
        }

        shard
    }

    #[inline(always)]
    pub fn shard_of<Q>(&self, key: &Q) -> &RwLock<HashMap<K, V, S>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        shard_of(self, key)
    }

    #[inline(always)]
    pub fn shards(&self) -> &[RwLock<HashMap<K, V, S>>] {
        &self.shards
    }
}

// Safety: Entries order reflects guards drop semantics.
pub struct TableReadGuard<'a, K, V, S = RandomState> {
    value: &'a V,
    _guard: RwLockReadGuard<'a, HashMap<K, V, S>>,
}

impl<'a, K, V, S> Deref for TableReadGuard<'a, K, V, S> {
    type Target = V;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

// Safety: Entries order reflects guards drop semantics.
pub struct TableWriteGuard<'a, K, V, S = RandomState> {
    value: &'a mut V,
    _guard: RwLockWriteGuard<'a, HashMap<K, V, S>>,
}

impl<'a, K, V, S> Deref for TableWriteGuard<'a, K, V, S> {
    type Target = V;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, K, V, S> DerefMut for TableWriteGuard<'a, K, V, S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

pub enum TableEntry<'a, K: 'a, V: 'a, S = RandomState> {
    Occupied(TableOccupiedEntry<'a, K, V, S>),
    Vacant(TableVacantEntry<'a, K, V, S>),
}

impl<'a, K, V: Default, S> TableEntry<'a, K, V, S> {
    #[inline(always)]
    pub fn or_default(self) -> TableWriteGuard<'a, K, V, S> {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(V::default()),
        }
    }
}

impl<'a, K, V, S> TableEntry<'a, K, V, S> {
    #[inline(always)]
    pub fn or_insert(self, default: V) -> TableWriteGuard<'a, K, V, S> {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default),
        }
    }

    #[inline(always)]
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> TableWriteGuard<'a, K, V, S> {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default()),
        }
    }

    #[inline(always)]
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(
        self,
        default: F,
    ) -> TableWriteGuard<'a, K, V, S> {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => {
                let value = default(entry.key());
                entry.insert(value)
            }
        }
    }

    #[inline(always)]
    pub fn key(&self) -> &K {
        match self {
            Self::Occupied(entry) => entry.key(),
            Self::Vacant(entry) => entry.key(),
        }
    }

    #[inline(always)]
    pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Self {
        match self {
            Self::Occupied(mut entry) => {
                f(entry.get_mut());
                Self::Occupied(entry)
            }
            Self::Vacant(entry) => Self::Vacant(entry),
        }
    }
}

// Safety: Entries order reflects guards drop semantics.
pub struct TableOccupiedEntry<'a, K: 'a, V: 'a, S = RandomState> {
    entry: OccupiedEntry<'a, K, V>,
    guard: RwLockWriteGuard<'a, HashMap<K, V, S>>,
}

impl<K: Debug, V: Debug, S> Debug for TableOccupiedEntry<'_, K, V, S> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&self.entry, formatter)
    }
}

impl<'a, K, V, S> TableOccupiedEntry<'a, K, V, S> {
    #[inline(always)]
    pub fn key(&self) -> &K {
        self.entry.key()
    }

    #[inline(always)]
    pub fn remove_entry(self) -> (K, V) {
        self.entry.remove_entry()
    }

    #[inline(always)]
    pub fn get(&self) -> &V {
        self.entry.get()
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut V {
        self.entry.get_mut()
    }

    #[inline(always)]
    pub fn into_mut(self) -> TableWriteGuard<'a, K, V, S> {
        let value = self.entry.into_mut();

        TableWriteGuard {
            value,
            _guard: self.guard,
        }
    }

    #[inline(always)]
    pub fn insert(&mut self, value: V) -> V {
        self.entry.insert(value)
    }

    #[inline(always)]
    pub fn remove(self) -> V {
        self.entry.remove()
    }
}

// Safety: Entries order reflects guards drop semantics.
pub struct TableVacantEntry<'a, K: 'a, V: 'a, S = RandomState> {
    entry: VacantEntry<'a, K, V>,
    guard: RwLockWriteGuard<'a, HashMap<K, V, S>>,
}

impl<K: Debug, V: Debug, S> Debug for TableVacantEntry<'_, K, V, S> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&self.entry, formatter)
    }
}

impl<'a, K: 'a, V: 'a, S> TableVacantEntry<'a, K, V, S> {
    #[inline(always)]
    pub fn key(&self) -> &K {
        self.entry.key()
    }

    #[inline(always)]
    pub fn into_key(self) -> K {
        self.entry.into_key()
    }

    #[inline(always)]
    pub fn insert(self, value: V) -> TableWriteGuard<'a, K, V, S> {
        let value = self.entry.insert(value);

        TableWriteGuard {
            value,
            _guard: self.guard,
        }
    }
}

pub struct TableDrain<'a, K: 'a, V: 'a, S = RandomState> {
    probes: Vec<ProbeDrain<'a, K, V, S>>,
    table: &'a Table<K, V, S>,
}

impl<'a, K, V, S> FusedIterator for TableDrain<'a, K, V, S> {}

impl<'a, K, V, S> Iterator for TableDrain<'a, K, V, S> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let last = match self.probes.last_mut() {
                Some(guard) => guard,

                // Safety:
                //   1. `probes` vector initialized as non-empty.
                //   2. `probes` vector is grow-only.
                None => unsafe { debug_unreachable!("Empty managed vector.") },
            };

            if let Some(key_value) = last.drain.next() {
                return Some(key_value);
            }

            let index = self.probes.len();

            let mut guard = self
                .table
                .shards
                .get(index)?
                .write()
                .unwrap_or_else(|poison| poison.into_inner());

            let drain = guard.drain();

            // Safety:
            //   Prolongs reference lifetime to `self` lifetime.
            //   The value will be valid for as long as the guard is held.
            let drain = unsafe { transmute::<Drain<'_, K, V>, Drain<'_, K, V>>(drain) };

            self.probes.push(ProbeDrain {
                drain,
                _guard: guard,
            });
        }
    }
}

pub struct TableIntoIter<K, V, S = RandomState> {
    probe: HashMapIntoIter<K, V>,
    shards: IntoIter<RwLock<HashMap<K, V, S>>>,
}

impl<K, V, S> FusedIterator for TableIntoIter<K, V, S> {}

impl<K, V, S> Iterator for TableIntoIter<K, V, S> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(next) = self.probe.next() {
                return Some(next);
            }

            self.probe = self
                .shards
                .next()?
                .into_inner()
                .unwrap_or_else(|poison| poison.into_inner())
                .into_iter();
        }
    }
}

// Safety: Entries order reflects guards drop semantics.
struct ProbeDrain<'a, K: 'a, V: 'a, S> {
    drain: Drain<'a, K, V>,
    _guard: RwLockWriteGuard<'a, HashMap<K, V, S>>,
}

#[inline(always)]
fn shard_of<'a, K, V, S, Q>(table: &'a Table<K, V, S>, key: &Q) -> &'a RwLock<HashMap<K, V, S>>
where
    K: Hash + Eq + Borrow<Q>,
    Q: Hash + Eq + ?Sized,
    S: BuildHasher,
{
    let shard_index = table.shard_index_of(key);

    match table.shards.get(shard_index) {
        Some(shard) => shard,

        // Safety:
        //   1. `shard_index_of` always returns a shard index
        //       within a `shards` length.
        //   2. `shards` is never empty.
        None => unsafe { debug_unreachable!("Table shard index out of bounds.") },
    }
}

#[inline(always)]
fn shards_amount() -> usize {
    #[cfg(not(target_family = "wasm"))]
    {
        available_parallelism()
            .map_or(1usize, |parallelism| 4 * usize::from(parallelism))
            .next_power_of_two()
    }

    #[cfg(target_family = "wasm")]
    {
        1
    }
}
