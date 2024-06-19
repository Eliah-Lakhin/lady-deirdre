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

use std::{
    fmt::{Debug, Formatter},
    iter::{Enumerate, FilterMap},
    mem::replace,
    slice::{Iter, IterMut},
    vec::IntoIter,
};

use crate::{
    arena::{Entry, EntryIndex, EntryVersion},
    report::ld_unreachable,
};

/// A versioned storage of arbitrary objects.
///
/// The Repo (short from "repository") is a vector of entries. Each entry
/// is either vacant or occupied by an object of `T` type.
///
/// The repository manages a linked list of vacant entries within the inner
/// vector of entries. When the user inserts a new object into the repository,
/// the underlying implementation puts this value into the next vacant entry,
/// turning this entry into occupied. When the user removes the entry,
/// the repository turns this entry into vacant and links it to the top of the
/// previous vacant entry, making it the next candidate for insertion.
///
/// Accessing, inserting, and removing values happening through the indices
/// within the inner vector, which is a fast O(1) operation.
///
/// The repository has a content version, which is a number that starts
/// from 1 and always increases when the user inserts a new entry.
/// Each occupied entry within this repository stores a version number under
/// which the value has been inserted.
///
/// As such, each occupied entry within the repository is uniquely identifiable
/// by the insertion version and the index in the inner vector. The next time
/// when the vacant entry will be reoccupied by a new value, it will receive
/// a new version number.
///
/// This compound pair of the version and index numbers is called
/// "versioned index", addressed by the [Entry] object, and serves as a key
/// of the value within the Repo.
///
/// Additionally, the repository allows late initializations of the entries.
///
/// The third kind of entry is a reserved entry, which is treated as occupied
/// but does not have a value yet.
pub struct Repo<T> {
    entries: Vec<RepoEntry<T>>,
    next: EntryIndex,
    version: EntryVersion,
    modified: bool,
}

impl<T> Default for Repo<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Debug for Repo<T> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("Repository")
    }
}

/// A type of iterator over the borrowed occupied values in the [Repo].
///
/// Crated by the [Repo::iter] function.
pub type RepoIter<'a, T> = FilterMap<Iter<'a, RepoEntry<T>>, fn(&'a RepoEntry<T>) -> Option<&'a T>>;

/// A type of iterator over the mutably borrowed occupied values in the [Repo].
///
/// Crated by the [Repo::iter_mut] function.
pub type RepoIterMut<'a, T> =
    FilterMap<IterMut<'a, RepoEntry<T>>, fn(&'a mut RepoEntry<T>) -> Option<&'a mut T>>;

/// A type of iterator over the indexed keys of the occupied entries
/// in the [Repo].
///
/// Crated by the [Repo::entries] function.
pub type RepoEntriesIter<'a, T> =
    FilterMap<Enumerate<Iter<'a, RepoEntry<T>>>, fn((usize, &'a RepoEntry<T>)) -> Option<Entry>>;

/// A type of the owning iterator over the indexed keys of the occupied entries
/// in the [Repo].
///
/// Crated by the [Repo::into_entries] function.
pub type RepoEntriesIntoIter<T> =
    FilterMap<Enumerate<IntoIter<RepoEntry<T>>>, fn((usize, RepoEntry<T>)) -> Option<Entry>>;

/// A type of the owning iterator over the occupied values in the [Repo].
///
/// Crated by the [into_iter](IntoIter::into_iter) function of the Repo.
pub type RepoIntoIter<T> = FilterMap<IntoIter<RepoEntry<T>>, fn(RepoEntry<T>) -> Option<T>>;

impl<'a, T> IntoIterator for &'a Repo<T> {
    type Item = &'a T;
    type IntoIter = RepoIter<'a, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Repo<T> {
    type Item = &'a mut T;
    type IntoIter = RepoIterMut<'a, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> IntoIterator for Repo<T> {
    type Item = T;
    type IntoIter = RepoIntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter().filter_map(|entry| match entry {
            RepoEntry::Occupied { data, .. } => Some(data),
            _ => None,
        })
    }
}

impl<T> FromIterator<T> for Repo<T> {
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let entries = iter
            .into_iter()
            .map(|data| RepoEntry::Occupied { data, version: 1 })
            .collect::<Vec<_>>();

        let next = entries.len();

        Self {
            entries,
            next,
            version: 1,
            modified: false,
        }
    }
}

impl<T> Repo<T> {
    /// Creates a new empty repository.
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            next: 0,
            version: 1,
            modified: false,
        }
    }

    /// Creates a new empty repository capable of storing at least `capacity`
    /// number of entries without reallocation.
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            next: 0,
            version: 1,
            modified: false,
        }
    }

    /// Inserts a new value into the repository.
    ///
    /// Returns a versioned index of the Occupied entry.
    #[inline]
    pub fn insert(&mut self, data: T) -> Entry {
        let index = self.insert_raw(data);

        unsafe { self.entry_of_unchecked(index) }
    }

    /// Inserts a new value into the repository.
    ///
    /// Returns an index of the Occupied entry.
    ///
    /// This function is slightly faster than [insert](Self::insert) for
    /// bulk loading.
    pub fn insert_raw(&mut self, data: T) -> EntryIndex {
        let index = self.next;

        self.commit(false);

        let Some(vacant) = self.entries.get_mut(self.next) else {
            self.entries.push(RepoEntry::Occupied {
                data,
                version: self.version,
            });

            self.next += 1;

            return index;
        };

        self.next = match replace(
            vacant,
            RepoEntry::Occupied {
                data,
                version: self.version,
            },
        ) {
            RepoEntry::Vacant(next) => next,

            // Safety: `next` always refers Vacant entry.
            _ => unsafe { ld_unreachable!("Wrong discriminant.") },
        };

        index
    }

    /// Inserts a Reserved entry into the repository for late initialization.
    ///
    /// Returns an index of the reserved entry.
    ///
    /// The [set_unchecked](Self::set_unchecked) function completes entry
    /// initialization, turning it into Occupied.
    ///
    /// It is safe to remove the Reserved entry by index without initialization
    /// using the [remove_unchecked](Self::remove_unchecked) function
    pub fn reserve_entry(&mut self) -> EntryIndex {
        let index = self.next;

        self.commit(false);

        let Some(vacant) = self.entries.get_mut(self.next) else {
            self.entries.push(RepoEntry::Reserved {
                version: self.version,
            });

            self.next += 1;

            return index;
        };

        self.next = match replace(
            vacant,
            RepoEntry::Reserved {
                version: self.version,
            },
        ) {
            RepoEntry::Vacant(next) => next,

            // Safety: `next` always refers Vacant item.
            _ => unsafe { ld_unreachable!("Wrong discriminant.") },
        };

        index
    }

    /// Removes an Occupied entry from the repository by
    /// the [versioned index](Entry).
    ///
    /// Returns the value of the entry if this Occupied entry exists in
    /// the repository; otherwise, returns None.
    #[inline]
    pub fn remove(&mut self, entry: &Entry) -> Option<T> {
        let repo_entry = self.entries.get_mut(entry.index)?;

        match repo_entry {
            RepoEntry::Occupied { version, .. } if version == &entry.version => (),

            _ => return None,
        }

        let occupied = replace(repo_entry, RepoEntry::Vacant(self.next));

        let RepoEntry::Occupied { data, .. } = occupied else {
            // Safety: `discriminant` checked above.
            unsafe { ld_unreachable!("Wrong discriminant.") }
        };

        self.modified = true;
        self.next = entry.index;

        Some(data)
    }

    /// Returns the current version of this repository.
    ///
    /// Note that this number is always positive, because repository versions
    /// start from 1.
    #[inline(always)]
    pub fn version(&self) -> EntryVersion {
        self.version
    }

    /// Increases the repository version.
    ///
    /// If the `force` flag is false, the version increase is up to
    /// the repository's decision.
    #[inline(always)]
    pub fn commit(&mut self, force: bool) {
        if force || self.modified {
            self.version += 1;
            self.modified = false;
        }
    }

    /// Removes all entries from the repository while preserving allocated
    /// memory and the current version of the repository.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.modified = true;
        self.next = 0;
        self.entries.clear();
    }

    /// Returns true if the repository contains an Occupied entry addressed
    /// by the specified [versioned index](Entry).
    #[inline]
    pub fn contains(&self, entry: &Entry) -> bool {
        let Some(RepoEntry::Occupied { version, .. }) = self.entries.get(entry.index) else {
            return false;
        };

        *version == entry.version
    }

    /// Creates a [versioned index](Entry) of the Occupied or Reserved entry
    /// within this repository by the (non-versioned) `index` of the entry.
    ///
    /// Returns [nil](Entry::nil) versioned index, if there is no such entry.
    #[inline(always)]
    pub fn entry_of(&self, index: EntryIndex) -> Entry {
        let Some(entry) = self.entries.get(index) else {
            return Entry::nil();
        };

        let (RepoEntry::Occupied { version, .. } | RepoEntry::Reserved { version, .. }) = entry
        else {
            return Entry::nil();
        };

        Entry {
            index,
            version: *version,
        }
    }

    /// Immutably borrows a value of the Occupied entry by
    /// the [versioned index](Entry).
    ///
    /// Returns None if there is no such entry.
    #[inline]
    pub fn get(&self, entry: &Entry) -> Option<&T> {
        let Some(RepoEntry::Occupied { data, version }) = self.entries.get(entry.index) else {
            return None;
        };

        if version != &entry.version {
            return None;
        }

        Some(data)
    }

    /// Mutably borrows a value of the Occupied entry by
    /// the [versioned index](Entry).
    ///
    /// Returns None if there is no such entry.
    #[inline]
    pub fn get_mut(&mut self, entry: &Entry) -> Option<&mut T> {
        let Some(RepoEntry::Occupied { data, version }) = self.entries.get_mut(entry.index) else {
            return None;
        };

        if version != &entry.version {
            return None;
        }

        Some(data)
    }

    /// Returns an iterator over immutable references of the values of
    /// all Occupied entries within the repository.
    #[inline(always)]
    pub fn iter(&self) -> RepoIter<T> {
        self.entries.iter().filter_map(|entry| match entry {
            RepoEntry::Occupied { data, .. } => Some(data),
            _ => None,
        })
    }

    /// Returns an iterator over mutable references of the values of
    /// all Occupied entries within the repository.
    #[inline(always)]
    pub fn iter_mut(&mut self) -> RepoIterMut<T> {
        self.entries.iter_mut().filter_map(|entry| match entry {
            RepoEntry::Occupied { data, .. } => Some(data),
            _ => None,
        })
    }

    /// Returns an iterator that yields the [versioned indices](Entry) of all
    /// Occupied entries within the repository.
    ///
    /// This function does not consume the repository instance.
    #[inline(always)]
    pub fn entries(&self) -> RepoEntriesIter<T> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| match entry {
                RepoEntry::Occupied { version, .. } => Some(Entry {
                    index,
                    version: *version,
                }),
                _ => None,
            })
    }

    /// Returns an iterator that yields the [versioned indices](Entry) of all
    /// Occupied entries within the repository.
    ///
    /// This function consume the repository instance.
    #[inline(always)]
    pub fn into_entries(self) -> RepoEntriesIntoIter<T> {
        self.entries
            .into_iter()
            .enumerate()
            .filter_map(|(index, entry)| match entry {
                RepoEntry::Occupied { version, .. } => Some(Entry { index, version }),
                _ => None,
            })
    }

    /// Creates a [versioned index](Entry) of the Occupied or Reserved entry
    /// within this repository by the (non-versioned) `index` of the entry
    /// without extra checks.
    ///
    /// **Safety**
    ///
    /// An Occupied or Reserved entry addressed by the `index` parameter exists
    /// in the repository.
    #[inline(always)]
    pub unsafe fn entry_of_unchecked(&self, index: EntryIndex) -> Entry {
        let Some(entry) = self.entries.get(index) else {
            unsafe { ld_unreachable!("Index out of bounds.") }
        };

        let (RepoEntry::Occupied { version, .. } | RepoEntry::Reserved { version, .. }) = entry
        else {
            unsafe {
                ld_unreachable!(
                    "An attempt to make a reference from index pointing to vacant entry."
                )
            }
        };

        Entry {
            index,
            version: *version,
        }
    }

    /// Immutably borrows a value of the Occupied entry by
    /// the non-versioned index without extra checks.
    ///
    /// **Safety**
    ///
    /// An Occupied entry addressed by the `index` parameter exists in
    /// the repository.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: EntryIndex) -> &T {
        let Some(entry) = self.entries.get(index) else {
            unsafe { ld_unreachable!("Index out of bounds.") }
        };

        let RepoEntry::Occupied { data, .. } = entry else {
            unsafe { ld_unreachable!("An attempt to index into non-occupied entry.") }
        };

        data
    }

    /// Mutably borrows a value of the Occupied entry by
    /// the non-versioned index without extra checks.
    ///
    /// **Safety**
    ///
    /// An Occupied entry addressed by the `index` parameter exists in
    /// the repository.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, index: EntryIndex) -> &mut T {
        let Some(entry) = self.entries.get_mut(index) else {
            unsafe { ld_unreachable!("Index out of bounds.") }
        };

        let RepoEntry::Occupied { data, .. } = entry else {
            unsafe { ld_unreachable!("An attempt to index into non-occupied entry.") }
        };

        data
    }

    /// Sets the value of the Occupied or Reserved entry by the non-versioned
    /// index without extra checks.
    ///
    /// If the entry addressed by `index` is an Occupied entry, this
    /// function replaces the previous value with the new one and drops
    /// the previous value.
    ///
    /// If the entry addressed by `index` is a Reserved entry, this function
    /// initializes this entry turning it into Occupied.
    ///
    /// **Safety**
    ///
    /// An Occupied or Reserved entry addressed by the `index` parameter exists
    /// in the repository.
    #[inline(always)]
    pub unsafe fn set_unchecked(&mut self, index: EntryIndex, data: T) {
        let Some(entry) = self.entries.get_mut(index) else {
            unsafe { ld_unreachable!("Index out of bounds.") }
        };

        let (RepoEntry::Occupied { version, .. } | RepoEntry::Reserved { version, .. }) = entry
        else {
            unsafe { ld_unreachable!("An attempt to write into vacant entry.") }
        };

        *entry = RepoEntry::Occupied {
            data,
            version: *version,
        };
    }

    /// Removes Occupied or Reserved entry by the non-versioned index
    /// without extra checks.
    ///
    /// If the entry addressed by `index` is an Occupied entry, this
    /// function drops this value, turning the entry into Vacant.
    ///
    /// If the entry addressed by `index` is a Reserved (not yet initialized)
    /// entry, this function just turns the entry into Vacant.
    ///
    /// **Safety**
    ///
    /// An Occupied or Reserved entry addressed by the `index` parameter exists
    /// in the repository.
    #[inline(always)]
    pub unsafe fn remove_unchecked(&mut self, index: EntryIndex) -> Entry {
        let Some(entry) = self.entries.get_mut(index) else {
            unsafe { ld_unreachable!("Index out of bounds.") }
        };

        let occupied = replace(entry, RepoEntry::Vacant(self.next));

        let (RepoEntry::Occupied { version, .. } | RepoEntry::Reserved { version, .. }) = occupied
        else {
            unsafe { ld_unreachable!("An attempt to remove vacant entry.") }
        };

        self.modified = true;
        self.next = index;

        Entry { index, version }
    }

    /// Assigns the current [version](Self::version) of the repository to
    /// the Occupied or Reserved entry by the non-versioned index without
    /// extra checks.
    ///
    /// This function is useful whenever you need to dissociate already existing
    /// value from their [versioned indices](Entry) but wants to preserve their
    /// inner indices.
    ///
    /// For example, if you want to bulk update several entries, using this
    /// function would be more efficient than removing and re-inserting
    /// corresponding values.
    ///
    /// In this case, you may also want to call the `repo.commit(true)` function
    /// before the (bulk) update to ensure that the repository receives a new
    /// version.
    ///
    /// **Safety**
    ///
    /// An Occupied or Reserved entry addressed by the `index` parameter exists
    /// in the repository.
    #[inline(always)]
    pub unsafe fn upgrade(&mut self, index: EntryIndex) {
        let Some(entry) = self.entries.get_mut(index) else {
            unsafe { ld_unreachable!("Index out of bounds.") }
        };

        let (RepoEntry::Occupied { version, .. } | RepoEntry::Reserved { version, .. }) = entry
        else {
            unsafe { ld_unreachable!("An attempt to upgrade revision of vacant entry.") }
        };

        *version = self.version;
    }
}

#[doc(hidden)]
pub enum RepoEntry<T> {
    Vacant(EntryIndex),
    Occupied { data: T, version: EntryVersion },
    Reserved { version: EntryVersion },
}
