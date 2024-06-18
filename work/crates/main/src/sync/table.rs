////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

///////////////////////////////////////////////////////////////////////////////////////
// A part of this file's source code is an adaptation of the Joel Wejdenstål's       //
// "DashMap" work.                                                                   //
//                                                                                   //
// Joel Wejdenstål's original work available here:                                   //
// https://github.com/xacrimon/dashmap/tree/626b98dab3c124cd9cd4960d0306da5d65918dfc //
//                                                                                   //
// Joel Wejdenstål grants me with a license to his work under the following terms:   //
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

use std::{
    borrow::Borrow,
    collections::{
        hash_map,
        hash_map::{Drain, Entry, OccupiedEntry, VacantEntry},
        HashMap,
    },
    fmt::{Debug, Formatter},
    hash::{BuildHasher, Hash, Hasher, RandomState},
    iter::FusedIterator,
    mem::{size_of, transmute},
    ops::{Deref, DerefMut},
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError},
    vec,
};

use crate::report::ld_unreachable;

/// A sharded read-write lock of the HashMap.
///
/// This object provides concurrent read-write access to the HashMap entries.
///
/// The concurrent read and write access to the **distinct** entries of
/// the Table are likely will not block each other if the entry keys are well
/// distributed by the hasher.
///
/// The underlying implementation achieves this feature by distributing the
/// hash-map entries between the prepared array of elements of fixed
/// size (referred to as the "shards amount") that depends on the available
/// parallelism.
///
/// By default, the shards amount estimated automatically based on the number of
/// CPUs, but could be overridden manually in
/// the [with_capacity_and_hasher_and_shards](Self::with_capacity_and_hasher_and_shards)
/// constructor.
///
/// The `K` generic parameter specifies a type of the entry key. This type
/// is assumed to implement a [Hash] interface.
///
/// The `V` generic parameter specifies a type of the entry value.
///
/// The `S` generic parameter specifies a hasher algorithm. By default,
/// the Table uses standard [RandomState].
///
/// If you are familiar with
/// the [dashmap](https://github.com/xacrimon/dashmap/tree/626b98dab3c124cd9cd4960d0306da5d65918dfc)
/// crate, Lady Deirdre's Table provides almost the same set of features with a
/// few differences:
///
///  - The Table interface allows one-shard configuration, reduces the Table
///    to a simple `RwLock<HashMap<K, V, S>>`. In particular, under
///    the `wasm` targets, the amount of shards is 1.
///  - The Table is fully built on top of the standard library features without
///    any third-party dependencies. In particular, the Table implementation
///    uses [RwLock] instead of the lock_api's RwLock in the DashMap.
///  - There are some opinionated differences in the API between these two
///    implementations, but, in general, both of them are trying to mimic the
///    standard's HashMap API for end-user convenience.
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
            None => unsafe { ld_unreachable!("Empty shards array.") },
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
    /// A default Table constructor.
    #[inline(always)]
    pub fn new() -> Self
    where
        S: Default + Clone,
    {
        Self::with_capacity(0)
    }

    /// A Table constructor with a specified preallocated `capacity` of entries.
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self
    where
        S: Default + Clone,
    {
        Self::with_capacity_and_hasher(capacity, S::default())
    }

    /// A Table constructor with a specified preallocated `capacity` of entries,
    /// and the key `hasher` instance.
    #[inline(always)]
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self
    where
        S: Clone,
    {
        Self::with_capacity_and_hasher_and_shards(capacity, hasher, shards_amount())
    }

    /// A Table constructor with a specified preallocated `capacity` of entries,
    /// the key `hasher` instance, and the amount of `shards`.
    ///
    /// The `shards` amount must me a positive number and a power of two.
    ///
    /// The shards equal to one is a valid argument, which makes the Table
    /// similar to `RwLock<HashMap>`.
    ///
    /// **Panic**
    ///
    /// Panics, if the `shards` value is zero or is not a power of two.
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

    /// Returns true if the Table has an entry with the specified `key`.
    ///
    /// Blocks the current thread if the entry or its shard is locked for write.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let shard = self.shard_of(key);

        let guard = shard.read().unwrap_or_else(|poison| poison.into_inner());

        guard.contains_key(key)
    }

    /// Grants read access to the entry's value by `key`.
    ///
    /// Returns None if the Table does not have an entry with specified key.
    ///
    /// Blocks the current thread if the entry or its shard is locked for write.
    ///
    /// The returning guard object locks the entry's shard for read.
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

    /// Grants read access to the entry's value by `key`.
    ///
    /// Returns None if the Table does not have an entry with specified key.
    ///
    /// Returns None if the entry or its shard is locked for write.
    ///
    /// This function does not block the current thread.
    ///
    /// The returning guard object locks the entry's shard for read.
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

    /// Grants read-write access to the entry's value by `key`.
    ///
    /// Returns None if the Table does not have an entry with specified key.
    ///
    /// Blocks the current thread if the entry or its shard is locked for read
    /// or write.
    ///
    /// The returning guard object locks the entry's shard for write.
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

    /// Grants read-write access to the table entry by `key` for in-place
    /// manipulation.
    ///
    /// The meaning of this function is similar to the [HashMap::entry] function.
    ///
    /// The function blocks the current thread if the shard of entries
    /// determined by `key` is locked for read or write.
    ///
    /// The returning guard object locks the shard for write.
    pub fn entry(&self, key: K) -> TableEntry<K, V, S> {
        let shard = self.shard_of(&key);

        let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

        let entry = guard.entry(key);

        // Safety:
        //   Prolongs reference lifetime to `self` lifetime.
        //   The value will be valid for as long as the guard is held.
        let entry = unsafe { transmute::<Entry<'_, K, V>, Entry<'_, K, V>>(entry) };

        match entry {
            Entry::Occupied(entry) => TableEntry::Occupied(TableOccupiedEntry { entry, guard }),
            Entry::Vacant(entry) => TableEntry::Vacant(TableVacantEntry { entry, guard }),
        }
    }

    /// Inserts a key-value entry into this Table.
    ///
    /// Returns the previous value mapped to the `key`.
    ///
    /// Returns None if there is no entry that belongs to the `key`.
    ///
    /// Blocks the current thread if the entry or its shard is locked for read
    /// or write.
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let shard = self.shard_of(&key);

        let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

        guard.insert(key, value)
    }

    /// Removes a key-value entry from this Table.
    ///
    /// Returns the value of the removed entry.
    ///
    /// Returns None if there is no entry that belongs to the `key`.
    ///
    /// Blocks the current thread if the entry or its shard is locked for read
    /// or write.
    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let shard = self.shard_of(key);

        let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

        guard.remove(key)
    }

    /// Removes a key-value entry from this Table and returns the removed
    /// key-value pair.
    ///
    /// Returns None if there is no entry that belongs to the `key`.
    ///
    /// Blocks the current thread if the entry or its shard is locked for read
    /// or write.
    pub fn remove_entry<Q>(&self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let shard = self.shard_of(key);

        let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

        guard.remove_entry(key)
    }

    /// Clears the Table, returning an iterator over removed
    /// key-value entry pairs.
    ///
    /// This function keeps allocated memory for reuse.
    ///
    /// Under the hood, the function sequentially locks each shard for write,
    /// and calls the [HashMap::drain] function on each of them.
    ///
    /// The returning iterator locks each Table shard one by one for write,
    /// and it **does not unlock** them until the iterator fully consumed or
    /// dropped.
    ///
    /// Hence, concurrent access to the Table will be in sync with `drain`.
    pub fn drain(&self) -> TableDrain<'_, K, V, S> {
        let mut guard = match self.shards.first() {
            Some(shard) => shard.write().unwrap_or_else(|poison| poison.into_inner()),

            // Safety: `shards` array is never empty.
            None => unsafe { ld_unreachable!("Empty shards array.") },
        };

        let mut probes = Vec::with_capacity(self.shards.len());

        let drain = guard.drain();

        // Safety:
        //   Prolongs reference lifetime to `self` lifetime.
        //   The value will be valid for as long as the guard is held.
        let drain = unsafe { transmute::<Drain<'_, K, V>, Drain<'_, K, V>>(drain) };

        probes.push(ProbeDrain {
            drain: Some(drain),
            _guard: guard,
        });

        TableDrain {
            probes,
            table: self,
        }
    }

    /// Retains only the key-value entries specified by predicate.
    ///
    /// The `f` predicate parameter tests each Table key-value pair, and if the
    /// predicate returns false, the retain function removes this entry.
    ///
    /// Under the hood, the function sequentially locks each shard one by one
    /// for write, and calls the [HashMap::retain] function with this predicate
    /// on each of them.
    ///
    /// The retain function **does not unlock** previously locked shards until
    /// finishes.
    ///
    /// Hence, the concurrent access to the Table will be in sync with `retain`.
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

    /// Clears the Table, removing all key-value entries.
    ///
    /// This function keeps allocated memory for reuse.
    ///
    /// Under the hood, the function sequentially locks each shard one by one
    /// for write, and calls the [HashMap::clear] function on each of them.
    ///
    /// The clear function **does not unlock** previously locked shards until
    /// finishes.
    ///
    /// Hence, the concurrent access to the Table will be in sync with `clear`.
    pub fn clear(&self) {
        let mut guards = Vec::with_capacity(self.shards.len());

        for shard in self.shards.iter() {
            let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

            guard.clear();

            guards.push(guard);
        }
    }

    /// Shrinks the capacity of the Table as much as possible by locking each
    /// shard one by one for write and calling the [HashMap::shrink_to_fit]
    /// function on each of them.
    ///
    /// The shrink_to_fit function **unlocks** previously locked shard
    /// immediately after shrinking.
    pub fn shrink_to_fit(&self) {
        for shard in self.shards.iter() {
            let mut guard = shard.write().unwrap_or_else(|poison| poison.into_inner());

            guard.shrink_to_fit();
        }
    }

    /// Provides access to the inner hasher.
    ///
    /// The returning hasher is the hasher used for the shards index
    /// computations, and the hasher of each shard's HashMap.
    pub fn hasher(&self) -> &S {
        &self.hasher
    }

    /// Computes an index of the shard within the [shards](Self::shards) array
    /// for the specified `key`.
    ///
    /// The returning value is **guaranteed** to be within the shards array
    /// bounds.
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
                ld_unreachable!("Table shard index out of bounds.");
            }
        }

        shard
    }

    /// Provides access to the shard by `key`.
    ///
    /// This function does not lock the shard.
    ///
    /// Calling to this function is similar
    /// to `self.shards()[self.shard_index_of(key)]`, but is slightly faster
    /// because the underlying implementation avoids unnecessary checks of the
    /// bounds.
    #[inline(always)]
    pub fn shard_of<Q>(&self, key: &Q) -> &RwLock<HashMap<K, V, S>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        shard_of(self, key)
    }

    /// Provides access to the underlying shards array.
    #[inline(always)]
    pub fn shards(&self) -> &[RwLock<HashMap<K, V, S>>] {
        &self.shards
    }
}

/// A RAII guard, that provides read access to the [Table] entry's value.
///
/// Created by the [Table::get] or [Table::try_get] methods.
///
/// The guard keeps the corresponding Table shard locked for read until
/// the guard is dropped.
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

/// A RAII guard, that provides read and write access to the [Table] entry's
/// value.
///
/// Created by the [Table::get_mut] method.
///
/// The guard keeps the corresponding Table shard locked for write until
/// the guard is dropped.
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

/// A RAII guard, which is a view into a single entry in a [Table].
///
/// The entry may either be vacant or occupied.
///
/// Created by the [Table::entry] method.
///
/// An API of this object is similar to the HashMap's [Entry] API.
///
/// The guard keeps the corresponding Table shard locked for write until
/// the guard is dropped.
pub enum TableEntry<'a, K: 'a, V: 'a, S = RandomState> {
    /// An occupied entry.
    Occupied(TableOccupiedEntry<'a, K, V, S>),

    /// A vacant entry.
    Vacant(TableVacantEntry<'a, K, V, S>),
}

impl<'a, K, V: Default, S> TableEntry<'a, K, V, S> {
    /// Ensures a value is in the entry by inserting the default value if empty,
    /// and returns a read-write access guard to the value.
    ///
    /// This function is similar to the [Entry::or_default] function.
    #[inline(always)]
    pub fn or_default(self) -> TableWriteGuard<'a, K, V, S> {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(V::default()),
        }
    }
}

impl<'a, K, V, S> TableEntry<'a, K, V, S> {
    /// Ensures a value is in the entry by inserting the `default` if empty,
    /// and returns a read-write access guard to the value.
    ///
    /// This function is similar to the [Entry::or_insert] function.
    #[inline(always)]
    pub fn or_insert(self, default: V) -> TableWriteGuard<'a, K, V, S> {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result of
    /// the `default` function if empty, and returns a read-write access guard
    /// to the value.
    ///
    /// This function is similar to the [Entry::or_insert_with] function.
    #[inline(always)]
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> TableWriteGuard<'a, K, V, S> {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default()),
        }
    }

    /// Ensures a value is in the entry by inserting the result of
    /// the `default` function if empty, and returns a read-write access guard
    /// to the value.
    ///
    /// The `default` function receives a key of the entry.
    ///
    /// This function is similar to the [Entry::or_insert_with_key] function.
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

    /// Returns a reference to this entry’s key.
    ///
    /// This function is similar to the [Entry::key] function.
    #[inline(always)]
    pub fn key(&self) -> &K {
        match self {
            Self::Occupied(entry) => entry.key(),
            Self::Vacant(entry) => entry.key(),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the Table.
    ///
    /// This function is similar to the [Entry::and_modify] function.
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

/// A RAII guard, which is a view into an occupied entry in a [Table].
///
/// It is part of the [TableEntry] enum.
///
/// An API of this object is similar to the HashMap's [OccupiedEntry] API.
///
/// The guard keeps the corresponding Table shard locked for write until
/// the guard is dropped.
// Safety: Entries order reflects guards drop semantics.
pub struct TableOccupiedEntry<'a, K: 'a, V: 'a, S = RandomState> {
    entry: OccupiedEntry<'a, K, V>,
    guard: RwLockWriteGuard<'a, HashMap<K, V, S>>,
}

impl<K: Debug, V: Debug, S> Debug for TableOccupiedEntry<'_, K, V, S> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.entry, formatter)
    }
}

impl<'a, K, V, S> TableOccupiedEntry<'a, K, V, S> {
    /// Returns a reference to the entry's key.
    ///
    /// This function is similar to the [OccupiedEntry::key] function.
    #[inline(always)]
    pub fn key(&self) -> &K {
        self.entry.key()
    }

    /// Takes the ownership of the key-value pair of the entry from the Table.
    ///
    /// This function is similar to the [OccupiedEntry::remove_entry] function.
    #[inline(always)]
    pub fn remove_entry(self) -> (K, V) {
        self.entry.remove_entry()
    }

    /// Returns a reference to the entry's value.
    ///
    /// This function is similar to the [OccupiedEntry::get] function.
    #[inline(always)]
    pub fn get(&self) -> &V {
        self.entry.get()
    }

    /// Returns a mutable reference to the entry's value.
    ///
    /// This function is similar to the [OccupiedEntry::get_mut] function.
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut V {
        self.entry.get_mut()
    }

    /// Converts this RAII guard into a [TableWriteGuard] RAII guard that grants
    /// read-write access to the entry value.
    ///
    /// This function keeps the corresponding shard locked.
    ///
    /// This function is similar to the [OccupiedEntry::into_mut] function.
    #[inline(always)]
    pub fn into_mut(self) -> TableWriteGuard<'a, K, V, S> {
        let value = self.entry.into_mut();

        TableWriteGuard {
            value,
            _guard: self.guard,
        }
    }

    /// Sets the value of the entry, and returns the entry’s old value.
    ///
    /// This function is similar to the [OccupiedEntry::insert] function.
    #[inline(always)]
    pub fn insert(&mut self, value: V) -> V {
        self.entry.insert(value)
    }

    /// Takes the value out of the entry, and returns it.
    ///
    /// This function is similar to the [OccupiedEntry::remove] function.
    #[inline(always)]
    pub fn remove(self) -> V {
        self.entry.remove()
    }
}

/// A RAII guard, which is a view into a vacant entry in a [Table].
///
/// It is part of the [TableEntry] enum.
///
/// An API of this object is similar to the HashMap's [VacantEntry] API.
///
/// The guard keeps the corresponding Table shard locked for write until
/// the guard is dropped.
// Safety: Entries order reflects guards drop semantics.
pub struct TableVacantEntry<'a, K: 'a, V: 'a, S = RandomState> {
    entry: VacantEntry<'a, K, V>,
    guard: RwLockWriteGuard<'a, HashMap<K, V, S>>,
}

impl<K: Debug, V: Debug, S> Debug for TableVacantEntry<'_, K, V, S> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.entry, formatter)
    }
}

impl<'a, K: 'a, V: 'a, S> TableVacantEntry<'a, K, V, S> {
    /// Returns a reference to the key that would be used when inserting a value
    /// through the TableVacantEntry.
    ///
    /// This function is similar to the [VacantEntry::key] function.
    #[inline(always)]
    pub fn key(&self) -> &K {
        self.entry.key()
    }

    /// Takes ownership of the key.
    ///
    /// This function is similar to the [VacantEntry::into_key] function.
    #[inline(always)]
    pub fn into_key(self) -> K {
        self.entry.into_key()
    }

    /// Sets the value of the entry with the VacantEntry’s key, and returns
    /// a [TableWriteGuard] RAII guard that grants read-write access to
    /// the entry value.
    ///
    /// This function is similar to the [VacantEntry::insert] function.
    #[inline(always)]
    pub fn insert(self, value: V) -> TableWriteGuard<'a, K, V, S> {
        let value = self.entry.insert(value);

        TableWriteGuard {
            value,
            _guard: self.guard,
        }
    }
}

/// A draining iterator over the entries of a [Table].
///
/// Created by the [Table::drain] method.
///
/// This object behaves similarly to the HashMap's [Drain] iterator.
pub struct TableDrain<'a, K: 'a, V: 'a, S = RandomState> {
    probes: Vec<ProbeDrain<'a, K, V, S>>,
    table: &'a Table<K, V, S>,
}

impl<'a, K, V, S> Drop for TableDrain<'a, K, V, S> {
    fn drop(&mut self) {
        loop {
            let index = self.probes.len();

            let mut guard = match self.table.shards.get(index) {
                Some(lock) => lock.write().unwrap_or_else(|poison| poison.into_inner()),
                None => break,
            };

            guard.clear();

            self.probes.push(ProbeDrain {
                drain: None,
                _guard: guard,
            });
        }
    }
}

impl<'a, K, V, S> FusedIterator for TableDrain<'a, K, V, S> {}

impl<'a, K, V, S> Iterator for TableDrain<'a, K, V, S> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(probe) = self.probes.last_mut() {
                if let Some(drain) = &mut probe.drain {
                    if let Some(key_value) = drain.next() {
                        return Some(key_value);
                    }
                }
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
                drain: Some(drain),
                _guard: guard,
            });
        }
    }
}

/// An owning iterator over the entries of a [Table].
pub struct TableIntoIter<K, V, S = RandomState> {
    probe: hash_map::IntoIter<K, V>,
    shards: vec::IntoIter<RwLock<HashMap<K, V, S>>>,
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
    drain: Option<Drain<'a, K, V>>,
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
        None => unsafe { ld_unreachable!("Table shard index out of bounds.") },
    }
}

#[inline(always)]
fn shards_amount() -> usize {
    #[cfg(not(target_family = "wasm"))]
    {
        std::thread::available_parallelism()
            .map_or(1usize, |parallelism| 4 * usize::from(parallelism))
            .next_power_of_two()
    }

    #[cfg(target_family = "wasm")]
    {
        1
    }
}
