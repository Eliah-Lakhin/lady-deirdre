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

use crate::{
    arena::{Entry, EntryIndex, EntryVersion},
    report::{debug_assert, debug_unreachable},
    std::*,
};

/// A mutable versioned data collection.
///
/// The interface provides a way to store, remove, update and mutate items in allocated memory, and
/// to access stored items by weak [versioned references](crate::arena::Entry::Repo).
///
/// All operations performed in "O(1)" constant time.
///
/// Under the hood this data structure holds Rust standard [vector](Vec) with entries. Each entry
/// exists in one of the three states: Occupied, Reserved, or Vacant.
///
/// When an API user adds a data item inside collection, it is either added into the next Vacant
/// entry turning this entry to Occupied state, or on the top of the vector into a new Occupied
/// entry. Vacant entries are managed in queue as a linked list. When the user removes data item,
/// corresponding entry turns into Vacant and is scheduled for the next insertion event in a queue
/// of Vacant entries.
///
/// Each Occupied(or Reserved) entry holds [version number](crate::arena::EntryVersion) of
/// occupied(or possibly occupied) data. And the corresponding Ref object that refers this entry
/// also holds this version value. If an API user removes an item from this collection, and later
/// occupies the entry with a different data item, a new entry will hold a different version value,
/// so the Ref to the old version of item would fail to resolve.
///
/// In other words, references into this collection items are always unique in the history of
/// collection changes.
///
/// Also, an API user can reserve entries inside this collection for late initialization. While
/// the entry is in Reserved state, it does not hold any data, but it could have weak references,
/// and it will not be Occupied by any other data item. These references are not valid for
/// dereferencing until the entry is fully initialized. Once the Reserved entry turns to Occupied
/// it could be dereferenced by initially created reference.
///
/// Collection's interface provides a high-level safe interface, and a lower level unsafe interface
/// that avoids some minor check overhead to benefit performance.
///
/// ```rust
/// use lady_deirdre::arena::{Repository, Entry};
///
/// let mut repo = Repository::<&'static str>::default();
///
/// let string_a_entry: Entry = repo.insert("foo");
/// let string_b_entry: Entry = repo.insert("bar");
///
/// assert_eq!(repo.get(&string_a_entry).unwrap(), &"foo");
/// assert_eq!(repo.get(&string_b_entry).unwrap(), &"bar");
///
/// repo.remove(&string_b_entry);
///
/// assert_eq!(repo.get(&string_a_entry).unwrap(), &"foo");
/// assert!(!repo.contains(&string_b_entry));
///
/// let string_c_entry: Entry = repo.insert("baz");
///
/// assert_eq!(repo.get(&string_a_entry).unwrap(), &"foo");
/// assert!(!repo.contains(&string_b_entry));
/// assert_eq!(repo.get(&string_c_entry).unwrap(), &"baz");
///
/// *(repo.get_mut(&string_a_entry).unwrap()) = "foo2";
///
/// assert_eq!(repo.get(&string_a_entry).unwrap(), &"foo2");
/// assert!(!repo.contains(&string_b_entry));
/// assert_eq!(repo.get(&string_c_entry).unwrap(), &"baz");
/// ```
pub struct Repository<T> {
    entries: Vec<RepositoryEntry<T>>,
    next: EntryIndex,
    revision: EntryVersion,
    modified: bool,
}

impl<T> Default for Repository<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Debug for Repository<T> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_str("Repository")
    }
}

pub type RepositoryIter<'a, T> =
    FilterMap<Iter<'a, RepositoryEntry<T>>, fn(&'a RepositoryEntry<T>) -> Option<&'a T>>;

pub type RepositoryIterMut<'a, T> =
    FilterMap<IterMut<'a, RepositoryEntry<T>>, fn(&'a mut RepositoryEntry<T>) -> Option<&'a mut T>>;

pub type RepositoryEntriesIter<'a, T> = FilterMap<
    Enumerate<Iter<'a, RepositoryEntry<T>>>,
    fn((usize, &'a RepositoryEntry<T>)) -> Option<Entry>,
>;

pub type RepositoryIntoIter<T> =
    FilterMap<IntoIter<RepositoryEntry<T>>, fn(RepositoryEntry<T>) -> Option<T>>;

impl<'a, T> IntoIterator for &'a Repository<T> {
    type Item = &'a T;
    type IntoIter = RepositoryIter<'a, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Repository<T> {
    type Item = &'a mut T;
    type IntoIter = RepositoryIterMut<'a, T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> IntoIterator for Repository<T> {
    type Item = T;
    type IntoIter = RepositoryIntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter().filter_map(|entry| match entry {
            RepositoryEntry::Occupied { data, .. } => Some(data),
            _ => None,
        })
    }
}

impl<T> FromIterator<T> for Repository<T> {
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let entries = iter
            .into_iter()
            .map(|data| RepositoryEntry::Occupied { data, revision: 0 })
            .collect::<Vec<_>>();

        let next = entries.len();

        Self {
            entries,
            next,
            revision: 0,
            modified: false,
        }
    }
}

impl<T> Repository<T> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            next: 0,
            revision: 0,
            modified: false,
        }
    }

    /// Creates a new collection instance with pre-allocated memory for at least `capacity` items
    /// to be stored in.
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            next: 0,
            revision: 0,
            modified: false,
        }
    }

    /// Adds an item into this collection returning valid weak reference to the item.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_entry = repo.insert(10);
    ///
    /// assert_eq!(repo.get(&item_entry).unwrap(), &10);
    /// ```
    #[inline]
    pub fn insert(&mut self, data: T) -> Entry {
        let index = self.insert_raw(data);

        unsafe { self.entry_of(index) }
    }

    /// Adds an item into this collection returning valid [RefIndex](crate::arena::EntryIndex) to
    /// access corresponding item from the inner array of this Repository.
    ///
    /// This is a low-level API.
    ///
    /// An API user can utilize this index with care to perform low-level unsafe operations with
    /// lesser overhead.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_entry_index = repo.insert_raw(10);
    ///
    /// // This is safe because `insert_index` returns valid index.
    /// unsafe {
    ///     assert_eq!(repo.get_unchecked(item_entry_index), &10);
    /// }
    ///
    /// // This is safe because `insert_index` returns valid index.
    /// let item_entry = unsafe {
    ///     repo.entry_of(item_entry_index)
    /// };
    ///
    /// assert_eq!(repo.get(&item_entry).unwrap(), &10);
    ///
    /// // This is safe because `insert_index` returns valid index, and the item is still
    /// // in the `repo`.
    /// unsafe {
    ///     repo.remove_unchecked(item_entry_index);
    /// };
    ///
    /// // From now on it would be unsafe to call e.g. `repo.get_unchecked(item_reference)`, because
    /// // the item is no longer exists in the `repo`.
    /// ```
    pub fn insert_raw(&mut self, data: T) -> EntryIndex {
        let index = self.next;

        self.commit(false);

        match self.entries.get_mut(self.next) {
            None => {
                self.entries.push(RepositoryEntry::Occupied {
                    data,
                    revision: self.revision,
                });

                self.next += 1;
            }

            Some(vacant) => {
                debug_assert!(
                    matches!(vacant, RepositoryEntry::Vacant(..)),
                    "Occupied entry in the next position.",
                );

                self.next = match replace(
                    vacant,
                    RepositoryEntry::Occupied {
                        data,
                        revision: self.revision,
                    },
                ) {
                    RepositoryEntry::Vacant(next) => next,
                    _ => unsafe { unreachable_unchecked() },
                }
            }
        }

        index
    }

    /// Reserves an entry inside this collection for late initialization.
    ///
    /// This is a low-level API.
    ///
    /// An API user can utilize low-level API to initialize referred entry later. In particular, the
    /// user can crate a [Ref](crate::arena::Entry) from received index. This reference will be
    /// considered invalid, but once the entry initializes it will become valid to dereference.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_entry_index = repo.reserve();
    ///
    /// // This is safe because `reserve` returns valid index.
    /// let item_entry = unsafe {
    ///     repo.entry_of(item_entry_index)
    /// };
    ///
    /// // Referred item is not yet initialized, so it cannot be dereferenced, but is it safe
    /// // to try to dereference.
    /// assert!(repo.get(&item_entry).is_none());
    ///
    /// // This is safe because `reserve` returns valid index.
    /// unsafe {
    ///     repo.set_unchecked(item_entry_index, 10);
    /// }
    ///
    /// // Since the item already initialized, from now on it is fine to dereference it.
    /// assert_eq!(repo.get(&item_entry).unwrap(), &10);
    /// ```
    pub fn reserve(&mut self) -> EntryIndex {
        let index = self.next;

        self.commit(false);

        match self.entries.get_mut(self.next) {
            None => {
                self.entries.push(RepositoryEntry::Reserved {
                    revision: self.revision,
                });

                self.next += 1;
            }

            Some(vacant) => {
                debug_assert!(
                    matches!(vacant, RepositoryEntry::Vacant(..)),
                    "Occupied entry in the next position.",
                );

                self.next = match replace(
                    vacant,
                    RepositoryEntry::Reserved {
                        revision: self.revision,
                    },
                ) {
                    RepositoryEntry::Vacant(next) => next,
                    _ => unsafe { unreachable_unchecked() },
                }
            }
        }

        index
    }

    /// Removes an item from this collection by reference.
    ///
    /// If referred item exists, returns the value. Otherwise returns [None].
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_entry = repo.insert(10);
    ///
    /// assert_eq!(repo.get(&item_entry).unwrap(), &10);
    ///
    /// assert_eq!(repo.remove(&item_entry).unwrap(), 10);
    ///
    /// // Referred value no longer exists in the `repo`.
    /// assert!(!repo.contains(&item_entry));
    /// ```
    #[inline]
    pub fn remove(&mut self, entry: &Entry) -> Option<T> {
        match entry {
            Entry::Repo { index, version } => {
                let entry = self.entries.get_mut(*index)?;

                match entry {
                    RepositoryEntry::Occupied { revision, .. } if revision == version => (),

                    _ => return None,
                }

                let occupied = replace(entry, RepositoryEntry::Vacant(self.next));

                let data = match occupied {
                    RepositoryEntry::Occupied { data, .. } => {
                        self.modified = true;
                        data
                    }
                    _ => unsafe { unreachable_unchecked() },
                };

                self.next = *index;

                Some(data)
            }

            _ => None,
        }
    }

    #[inline(always)]
    pub fn revision(&self) -> EntryVersion {
        self.revision
    }

    /// Raises repository internal version if the repository contain
    /// uncommitted changes, or if the `force` flag is true.
    ///
    /// This is a low-level API. Normally an API user does not need to call this function manually,
    /// as the versions are managed automatically.
    ///
    /// This function is supposed to be used together with "upgrade" function.
    /// See [Upgrade function documentation](Repository::upgrade) for details.
    ///
    /// Note that raising of the Repository version does not affect exist entries. It only
    /// affects a newly inserted items, or the items upgraded by the Upgrade function.
    #[inline(always)]
    pub fn commit(&mut self, force: bool) {
        if force || self.modified {
            self.revision += 1;
            self.modified = false;
        }
    }

    /// Removes all items from this collection preserving allocated memory.
    ///
    /// All references belong to this collection are implicitly turn to invalid.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.modified = true;
        self.next = 0;
        self.entries.clear();
    }

    /// Returns `true` if referred item exists in this collection in the Occupied entry.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_entry = repo.insert(10);
    ///
    /// assert!(repo.contains(&item_entry));
    ///
    /// let _ = repo.remove(&item_entry);
    ///
    /// assert!(!repo.contains(&item_entry));
    #[inline]
    pub fn contains(&self, entry: &Entry) -> bool {
        match entry {
            Entry::Repo { index, version } => match self.entries.get(*index) {
                Some(RepositoryEntry::Occupied { revision, .. }) => version == revision,
                _ => false,
            },

            _ => false,
        }
    }

    /// Tries to dereference referred item.
    ///
    /// Returns [None] if referred item does not exist in this collection in the Occupied entry.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_entry = repo.insert(10);
    ///
    /// assert_eq!(repo.get(&item_entry), Some(&10));
    ///
    /// let _ = repo.remove(&item_entry);
    ///
    /// assert_eq!(repo.get(&item_entry), None);
    #[inline]
    pub fn get(&self, entry: &Entry) -> Option<&T> {
        match entry {
            Entry::Repo { index, version } => match self.entries.get(*index) {
                Some(RepositoryEntry::Occupied { data, revision, .. }) if version == revision => {
                    Some(data)
                }
                _ => None,
            },

            _ => None,
        }
    }

    /// Tries to mutably dereference referred item.
    ///
    /// Returns [None] if referred item does not exist in this collection in the Occupied entry.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_entry = repo.insert(10);
    ///
    /// *(repo.get_mut(&item_entry).unwrap()) = 20;
    ///
    /// assert_eq!(repo.get(&item_entry), Some(&20));
    #[inline]
    pub fn get_mut(&mut self, entry: &Entry) -> Option<&mut T> {
        match entry {
            Entry::Repo { index, version } => match self.entries.get_mut(*index) {
                Some(RepositoryEntry::Occupied { data, revision, .. }) if version == revision => {
                    Some(data)
                }
                _ => None,
            },

            _ => None,
        }
    }

    #[inline(always)]
    pub fn iter(&self) -> RepositoryIter<T> {
        self.entries.iter().filter_map(|entry| match entry {
            RepositoryEntry::Occupied { data, .. } => Some(data),
            _ => None,
        })
    }

    #[inline(always)]
    pub fn iter_mut(&mut self) -> RepositoryIterMut<T> {
        self.entries.iter_mut().filter_map(|entry| match entry {
            RepositoryEntry::Occupied { data, .. } => Some(data),
            _ => None,
        })
    }

    #[inline(always)]
    pub fn entries(&self) -> RepositoryEntriesIter<T> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| match entry {
                RepositoryEntry::Occupied { revision, .. } => Some(Entry::Repo {
                    index,
                    version: *revision,
                }),
                _ => None,
            })
    }

    /// Returns item weak reference by internal index.
    ///
    /// This is a low-level API.
    ///
    /// This index could be received, for example, from the [insert_index](Repository::insert_raw)
    /// function.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_index = repo.insert_raw(10);
    ///
    /// let item_entry = unsafe {
    ///     repo.entry_of(item_index)
    /// };
    ///
    /// assert_eq!(repo.get(&item_entry), Some(&10));
    /// ```
    ///
    /// Note that unlike [Ref](crate::arena::Entry), [RefIndex](crate::arena::EntryIndex) is
    /// version-independent "reference" into this collection. An API user should care not to misuse
    /// indices.
    ///
    /// ```rust
    ///
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_a_index = repo.insert_raw(10);
    ///
    /// // This is safe because `insert_index` returns valid index.
    /// let item_a_entry = unsafe {
    ///     repo.entry_of(item_a_index)
    /// };
    ///
    /// assert_eq!(repo.get(&item_a_entry), Some(&10));
    ///
    /// // Removing all items from this collection.
    /// repo.clear();
    ///
    /// // Inserting a new item inside this collection.
    /// let item_b_index = repo.insert_raw(20);
    ///
    /// // `item_a_entry` is history-dependent.
    /// // An item previously referred by `item_a_entry` considered to be missing in this collection.
    /// assert!(!repo.contains(&item_a_entry));
    ///
    /// // However, Item B due to prior collection changes has the same index as removed Item A.
    /// assert_eq!(item_a_index, item_b_index);
    ///
    /// // Making a reference from `item_a_index` would return a reference to Item B.
    /// let item_a_entry = unsafe {
    ///     repo.entry_of(item_a_index)
    /// };
    ///
    /// // A new `item_a_entry` actually refers Item B.
    /// assert_eq!(repo.get(&item_a_entry), Some(&20));
    /// ```  
    ///
    /// **Safety:**
    ///   - An entry indexed by `index` exists in this collection either in Occupied, or in Reserved
    ///     state.
    #[inline(always)]
    pub unsafe fn entry_of(&self, index: EntryIndex) -> Entry {
        debug_assert!(index < self.entries.len(), "Index out of bounds.");

        #[allow(unreachable_code)]
        let entry = unsafe { self.entries.get_unchecked(index) };

        let version = match entry {
            RepositoryEntry::Occupied { revision, .. }
            | RepositoryEntry::Reserved { revision, .. } => *revision,

            // Safety: Upheld by the caller.
            RepositoryEntry::Vacant(..) => unsafe {
                debug_unreachable!(
                    "Internal error. An attempt to make a reference from index \
                    pointing to vacant entry."
                );
            },
        };

        Entry::Repo { index, version }
    }

    /// Immutably derefers collection's item by internal index.
    ///
    /// This is a low-level API.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_index = repo.insert_raw(10);
    ///
    /// // This is safe because `insert_item` occupies collection's entry.
    /// assert_eq!(unsafe { repo.get_unchecked(item_index) }, &10);
    /// ```
    ///
    /// **Safety:**
    ///   - An entry indexed by `index` exists in this collection in Occupied state.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: EntryIndex) -> &T {
        debug_assert!(index < self.entries.len(), "Index out of bounds.");

        let entry = unsafe { self.entries.get_unchecked(index) };

        match entry {
            RepositoryEntry::Occupied { data, .. } => data,

            // Safety: Upheld by the caller.
            _ => unsafe { debug_unreachable!("An attempt to index into non-occupied entry.") },
        }
    }

    /// Mutably derefers collection's item by internal index.
    ///
    /// This is a low-level API.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_index = repo.insert_raw(10);
    ///
    /// // This is safe because `insert_item` occupies collection's entry.
    /// unsafe { *repo.get_unchecked_mut(item_index) = 20; }
    ///
    /// assert_eq!(unsafe { repo.get_unchecked(item_index) }, &20);
    /// ```
    ///
    /// **Safety:**
    ///   - An entry indexed by `index` exists in this collection in Occupied state.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, index: EntryIndex) -> &mut T {
        debug_assert!(index < self.entries.len(), "Index out of bounds.");

        let entry = unsafe { self.entries.get_unchecked_mut(index) };

        match entry {
            RepositoryEntry::Occupied { data, .. } => data,

            // Safety: Upheld by the caller.
            _ => unsafe { debug_unreachable!("An attempt to index into non-occupied entry.") },
        }
    }

    /// Replaces Occupied item value by collection's internal index, or initializes
    /// Reserved item by index.
    ///
    /// This is a low-level API.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_index = repo.insert_raw(10);
    ///
    /// // This is safe because `insert_item` occupies collection's entry.
    /// unsafe { repo.set_unchecked(item_index, 20); }
    ///
    /// assert_eq!(unsafe { repo.get_unchecked(item_index) }, &20);
    /// ```
    ///
    /// If the indexed entry is a Reserved entry, this function initializes this item turning entry
    /// state to Occupied.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_index = repo.reserve();
    ///
    /// // This is safe because `reserve` returns valid index.
    /// let item_entry = unsafe { repo.entry_of(item_index) };
    ///
    /// // Referred item is not initialized yet(is not "Occupied).
    /// assert!(!repo.contains(&item_entry));
    ///
    /// // Initializing reserved entry.
    /// unsafe { repo.set_unchecked(item_index, 10); }
    ///
    /// // From now on referred Item "exists" in this collection.
    /// assert!(repo.contains(&item_entry));
    /// ```
    ///
    /// **Safety:**
    ///   - An entry indexed by `index` exists in this collection in Occupied or Reserved state.
    #[inline(always)]
    pub unsafe fn set_unchecked(&mut self, index: EntryIndex, data: T) {
        debug_assert!(index < self.entries.len(), "Index out of bounds.");

        let entry = unsafe { self.entries.get_unchecked_mut(index) };

        let revision = match entry {
            RepositoryEntry::Reserved { revision } | RepositoryEntry::Occupied { revision, .. } => {
                *revision
            }

            // Safety: Upheld by the caller.
            RepositoryEntry::Vacant(..) => unsafe {
                debug_unreachable!("An attempt to write into vacant entry.")
            },
        };

        *entry = RepositoryEntry::Occupied { data, revision };
    }

    /// Removes collection's Occupied or Reserved entry by internal index.
    ///
    /// This is a low-level API.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_index = repo.insert_raw(10);
    ///
    /// // This is safe because `insert_index` returns valid index.
    /// let item_entry = unsafe { repo.entry_of(item_index) };
    ///
    /// // This is safe because `insert_item` returns valid index.
    /// unsafe { repo.remove_unchecked(item_index); }
    ///
    /// // From now on referred Item no longer "exists" in this collection.
    /// assert!(!repo.contains(&item_entry));
    /// ```
    ///
    /// An API user can utilize this function to remove Reserved entry without initialization.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_index = repo.reserve();
    ///
    /// // This is safe because `reserve` returns valid index.
    /// let item_entry = unsafe { repo.entry_of(item_index) };
    ///
    /// // This is safe because `reserve` returns valid index, and the Item's Entry exists in this
    /// // collection in Reserved state.
    /// unsafe { repo.remove_unchecked(item_index); }
    ///
    /// // From now on referred Entry no longer "exists" in this collection.
    /// // An API user cannot initialize this item by `item_index`.
    /// ```
    ///
    /// **Safety:**
    ///   - An entry indexed by `index` exists in this collection in Occupied or Reserved state.
    #[inline(always)]
    pub unsafe fn remove_unchecked(&mut self, index: EntryIndex) {
        debug_assert!(index < self.entries.len(), "Index out of bounds.");

        let entry = unsafe { self.entries.get_unchecked_mut(index) };

        let occupied = replace(entry, RepositoryEntry::Vacant(self.next));

        self.modified = true;

        match occupied {
            RepositoryEntry::Occupied { .. } | RepositoryEntry::Reserved { .. } => (),

            // Safety: Upheld by the caller.
            RepositoryEntry::Vacant { .. } => unsafe {
                debug_unreachable!("An attempt to remove vacant entry.")
            },
        };

        self.next = index;
    }

    /// Upgrades collection's Occupied or Reserved entry version without changing of their content.
    ///
    /// This is a low-level API that allows bulk "re-insertion" of several existing item in a more
    /// efficient way than the series of independent removes and inserts.
    ///
    /// If an API user wants to preserve some entries content, but needs to obsolete their weak
    /// references, a trivial way to do so is just to remove and then re-insert them:
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_a_entry = repo.insert(10);
    /// let item_b_entry = repo.insert(20);
    ///
    /// assert!(repo.contains(&item_a_entry));
    /// assert!(repo.contains(&item_b_entry));
    ///
    /// // We do not change the content of referred items, but just re-inserting them.
    /// let item_a_content = repo.remove(&item_a_entry).unwrap();
    /// let item_b_content = repo.remove(&item_b_entry).unwrap();
    /// let item_a_entry_2 = repo.insert(item_a_content);
    /// let item_b_entry_2 = repo.insert(item_b_content);
    ///
    /// // Old weak references no longer valid.
    /// assert!(!repo.contains(&item_a_entry));
    /// assert!(!repo.contains(&item_b_entry));
    /// ```
    ///
    /// This is safe approach, however this approach involves certain performance overhead that
    /// could be critical when performing bulk operations. In addition to that this approach does
    /// not preserve entries indices(which is also important in certain situations).
    ///
    /// If an API user confident about indices integrity, an alternative way would be using a
    /// [Commit](crate::arena::Repository::commit) function and series of Upgrade functions instead.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_a_index = repo.insert_raw(10);
    /// let item_b_index = repo.insert_raw(20);
    ///
    /// // This is safe because `insert_raw` returns valid index.
    /// let item_a_entry = unsafe { repo.entry_of(item_a_index) };
    /// let item_b_entry = unsafe { repo.entry_of(item_b_index) };
    ///
    /// assert!(repo.contains(&item_a_entry));
    /// assert!(repo.contains(&item_b_entry));
    ///
    /// // Forcefully raises Repository version.
    /// repo.commit(true);
    ///
    /// // This is safe because the items referred by index are still exist in this repository.
    /// unsafe {
    ///     repo.upgrade(item_a_index);
    ///     repo.upgrade(item_b_index);
    /// }
    ///
    /// // Previously created weak references no longer valid.
    /// assert!(!repo.contains(&item_a_entry));
    /// assert!(!repo.contains(&item_b_entry));
    ///
    /// // We can still create new weak references using these indices.
    /// let item_a_entry_2 = unsafe { repo.entry_of(item_a_index) };
    /// let item_b_entry_2 = unsafe { repo.entry_of(item_b_index) };
    ///
    /// assert!(repo.contains(&item_a_entry_2));
    /// assert!(repo.contains(&item_b_entry_2));
    /// ```
    ///
    /// Note, if an API user misses to call Commit function, it will not lead to undefined behavior,
    /// but in this case the Upgrade function does not guarantee version upgrade.
    ///
    /// **Safety:**
    ///   - An entry indexed by `index` exists in this collection in Occupied or Reserved state.
    #[inline(always)]
    pub unsafe fn upgrade(&mut self, index: EntryIndex) {
        debug_assert!(index < self.entries.len(), "Index out of bounds.");

        let entry = unsafe { self.entries.get_unchecked_mut(index) };

        match entry {
            RepositoryEntry::Occupied { revision, .. } | RepositoryEntry::Reserved { revision } => {
                *revision = self.revision;
            }

            // Safety: Upheld by the caller.
            RepositoryEntry::Vacant { .. } => unsafe {
                debug_unreachable!("An attempt to update revision of vacant entry.")
            },
        };
    }
}

#[doc(hidden)]
pub enum RepositoryEntry<T> {
    Vacant(EntryIndex),
    Occupied { data: T, revision: EntryVersion },
    Reserved { revision: EntryVersion },
}
