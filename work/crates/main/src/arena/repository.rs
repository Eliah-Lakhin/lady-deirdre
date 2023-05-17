use crate::{
    arena::{Ref, RefIndex, RefVersion},
    report::{debug_assert, debug_unreachable},
    std::*,
};

/// A mutable versioned data collection.
///
/// The interface provides a way to store, remove, update and mutate items in allocated memory, and
/// to access stored items by weak [versioned references](crate::arena::Ref::Repository).
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
/// Each Occupied(or Reserved) entry holds [version number](crate::arena::RefVersion) of
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
/// use lady_deirdre::arena::{Repository, Ref};
///
/// let mut repo = Repository::<&'static str>::default();
///
/// let string_a_ref: Ref = repo.insert("foo");
/// let string_b_ref: Ref = repo.insert("bar");
///
/// assert_eq!(repo.get(&string_a_ref).unwrap(), &"foo");
/// assert_eq!(repo.get(&string_b_ref).unwrap(), &"bar");
///
/// repo.remove(&string_b_ref);
///
/// assert_eq!(repo.get(&string_a_ref).unwrap(), &"foo");
/// assert!(!repo.contains(&string_b_ref));
///
/// let string_c_ref: Ref = repo.insert("baz");
///
/// assert_eq!(repo.get(&string_a_ref).unwrap(), &"foo");
/// assert!(!repo.contains(&string_b_ref));
/// assert_eq!(repo.get(&string_c_ref).unwrap(), &"baz");
///
/// *(repo.get_mut(&string_a_ref).unwrap()) = "foo2";
///
/// assert_eq!(repo.get(&string_a_ref).unwrap(), &"foo2");
/// assert!(!repo.contains(&string_b_ref));
/// assert_eq!(repo.get(&string_c_ref).unwrap(), &"baz");
/// ```
pub struct Repository<T> {
    entries: Vec<RepositoryEntry<T>>,
    next: RefIndex,
    revision: RefVersion,
    modified: bool,
}

impl<T> Default for Repository<T> {
    #[inline]
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next: 0,
            revision: 0,
            modified: false,
        }
    }
}

impl<T> Debug for Repository<T> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str("Repository")
    }
}

pub type RepositoryIterator<'a, T> =
    FilterMap<Iter<'a, RepositoryEntry<T>>, fn(&'a RepositoryEntry<T>) -> Option<&'a T>>;

impl<'a, T> IntoIterator for &'a Repository<T> {
    type Item = &'a T;
    type IntoIter = RepositoryIterator<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter().filter_map(|entry| match entry {
            RepositoryEntry::Occupied { data, .. } => Some(data),
            _ => None,
        })
    }
}

pub type RepositoryIntoIterator<T> =
    FilterMap<IntoIter<RepositoryEntry<T>>, fn(RepositoryEntry<T>) -> Option<T>>;

impl<T> IntoIterator for Repository<T> {
    type Item = T;
    type IntoIter = RepositoryIntoIterator<T>;

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
    /// let item_ref = repo.insert(10);
    ///
    /// assert_eq!(repo.get(&item_ref).unwrap(), &10);
    /// ```
    #[inline]
    pub fn insert(&mut self, data: T) -> Ref {
        let index = self.insert_index(data);

        unsafe { self.make_ref(index) }
    }

    /// Adds an item into this collection returning valid [RefIndex](crate::arena::RefIndex) to
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
    /// let item_ref_index = repo.insert_index(10);
    ///
    /// // This is safe because `insert_index` returns valid index.
    /// unsafe {
    ///     assert_eq!(repo.get_unchecked(item_ref_index), &10);
    /// }
    ///
    /// // This is safe because `insert_index` returns valid index.
    /// let item_ref = unsafe {
    ///     repo.make_ref(item_ref_index)
    /// };
    ///
    /// assert_eq!(repo.get(&item_ref).unwrap(), &10);
    ///
    /// // This is safe because `insert_index` returns valid index, and the item is still
    /// // in the `repo`.
    /// unsafe {
    ///     repo.remove_unchecked(item_ref_index);
    /// };
    ///
    /// // From now on it would be unsafe to call e.g. `repo.get_unchecked(item_reference)`, because
    /// // the item is no longer exists in the `repo`.
    /// ```
    pub fn insert_index(&mut self, data: T) -> RefIndex {
        let index = self.next;

        if self.modified {
            self.revision += 1;
            self.modified = false;
        }

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
    /// user can crate a [Ref](crate::arena::Ref) from received index. This reference will be
    /// considered invalid, but once the entry initializes it will become valid to dereference.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_ref_index = repo.reserve();
    ///
    /// // This is safe because `reserve` returns valid index.
    /// let item_ref = unsafe {
    ///     repo.make_ref(item_ref_index)
    /// };
    ///
    /// // Referred item is not yet initialized, so it cannot be dereferenced, but is it safe
    /// // to try to dereference.
    /// assert!(repo.get(&item_ref).is_none());
    ///
    /// // This is safe because `reserve` returns valid index.
    /// unsafe {
    ///     repo.set_unchecked(item_ref_index, 10);
    /// }
    ///
    /// // Since the item already initialized, from now on it is fine to dereference it.
    /// assert_eq!(repo.get(&item_ref).unwrap(), &10);
    /// ```
    pub fn reserve(&mut self) -> RefIndex {
        let index = self.next;

        if self.modified {
            self.revision += 1;
            self.modified = false;
        }

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
    /// let item_ref = repo.insert(10);
    ///
    /// assert_eq!(repo.get(&item_ref).unwrap(), &10);
    ///
    /// assert_eq!(repo.remove(&item_ref).unwrap(), 10);
    ///
    /// // Referred value no longer exists in the `repo`.
    /// assert!(!repo.contains(&item_ref));
    /// ```
    #[inline]
    pub fn remove(&mut self, reference: &Ref) -> Option<T> {
        match reference {
            Ref::Repository { index, version } => {
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

    /// Forcefully raises repository internal version.
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
    pub fn commit(&mut self) {
        self.revision += 1;
        self.modified = false;
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
    /// let item_ref = repo.insert(10);
    ///
    /// assert!(repo.contains(&item_ref));
    ///
    /// let _ = repo.remove(&item_ref);
    ///
    /// assert!(!repo.contains(&item_ref));
    #[inline]
    pub fn contains(&self, reference: &Ref) -> bool {
        match reference {
            Ref::Repository { index, version } => match self.entries.get(*index) {
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
    /// let item_ref = repo.insert(10);
    ///
    /// assert_eq!(repo.get(&item_ref), Some(&10));
    ///
    /// let _ = repo.remove(&item_ref);
    ///
    /// assert_eq!(repo.get(&item_ref), None);
    #[inline]
    pub fn get(&self, reference: &Ref) -> Option<&T> {
        match reference {
            Ref::Repository { index, version } => match self.entries.get(*index) {
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
    /// let item_ref = repo.insert(10);
    ///
    /// *(repo.get_mut(&item_ref).unwrap()) = 20;
    ///
    /// assert_eq!(repo.get(&item_ref), Some(&20));
    #[inline]
    pub fn get_mut(&mut self, reference: &Ref) -> Option<&mut T> {
        match reference {
            Ref::Repository { index, version } => match self.entries.get_mut(*index) {
                Some(RepositoryEntry::Occupied { data, revision, .. }) if version == revision => {
                    Some(data)
                }
                _ => None,
            },

            _ => None,
        }
    }

    /// Returns item weak reference by internal index.
    ///
    /// This is a low-level API.
    ///
    /// This index could be received, for example, from the [insert_index](Repository::insert_index)
    /// function.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_index = repo.insert_index(10);
    ///
    /// let item_ref = unsafe {
    ///     repo.make_ref(item_index)
    /// };
    ///
    /// assert_eq!(repo.get(&item_ref), Some(&10));
    /// ```
    ///
    /// Note that unlike [Ref](crate::arena::Ref), [RefIndex](crate::arena::RefIndex) is
    /// version-independent "reference" into this collection. An API user should care not to misuse
    /// indices.
    ///
    /// ```rust
    ///
    /// use lady_deirdre::arena::Repository;
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let item_a_index = repo.insert_index(10);
    ///
    /// // This is safe because `insert_index` returns valid index.
    /// let item_a_ref = unsafe {
    ///     repo.make_ref(item_a_index)
    /// };
    ///
    /// assert_eq!(repo.get(&item_a_ref), Some(&10));
    ///
    /// // Removing all items from this collection.
    /// repo.clear();
    ///
    /// // Inserting a new item inside this collection.
    /// let item_b_index = repo.insert_index(20);
    ///
    /// // `item_a_ref` is history-dependent.
    /// // An item previously referred by `item_a_ref` considered to be missing in this collection.
    /// assert!(!repo.contains(&item_a_ref));
    ///
    /// // However, Item B due to prior collection changes has the same index as removed Item A.
    /// assert_eq!(item_a_index, item_b_index);
    ///
    /// // Making a reference from `item_a_index` would return a reference to Item B.
    /// let item_a_ref = unsafe {
    ///     repo.make_ref(item_a_index)
    /// };
    ///
    /// // A new `item_a_ref` actually refers Item B.
    /// assert_eq!(repo.get(&item_a_ref), Some(&20));
    /// ```  
    ///
    /// **Safety:**
    ///   - An entry indexed by `index` exists in this collection either in Occupied, or in Reserved
    ///     state.
    #[inline(always)]
    pub unsafe fn make_ref(&self, index: RefIndex) -> Ref {
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

        Ref::Repository { index, version }
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
    /// let item_index = repo.insert_index(10);
    ///
    /// // This is safe because `insert_item` occupies collection's entry.
    /// assert_eq!(unsafe { repo.get_unchecked(item_index) }, &10);
    /// ```
    ///
    /// **Safety:**
    ///   - An entry indexed by `index` exists in this collection in Occupied state.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: RefIndex) -> &T {
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
    /// let item_index = repo.insert_index(10);
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
    pub unsafe fn get_unchecked_mut(&mut self, index: RefIndex) -> &mut T {
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
    /// let item_index = repo.insert_index(10);
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
    /// let item_ref = unsafe { repo.make_ref(item_index) };
    ///
    /// // Referred item is not initialized yet(is not "Occupied).
    /// assert!(!repo.contains(&item_ref));
    ///
    /// // Initializing reserved entry.
    /// unsafe { repo.set_unchecked(item_index, 10); }
    ///
    /// // From now on referred Item "exists" in this collection.
    /// assert!(repo.contains(&item_ref));
    /// ```
    ///
    /// **Safety:**
    ///   - An entry indexed by `index` exists in this collection in Occupied or Reserved state.
    #[inline(always)]
    pub unsafe fn set_unchecked(&mut self, index: RefIndex, data: T) {
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
    /// let item_index = repo.insert_index(10);
    ///
    /// // This is safe because `insert_index` returns valid index.
    /// let item_ref = unsafe { repo.make_ref(item_index) };
    ///
    /// // This is safe because `insert_item` returns valid index.
    /// unsafe { repo.remove_unchecked(item_index); }
    ///
    /// // From now on referred Item no longer "exists" in this collection.
    /// assert!(!repo.contains(&item_ref));
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
    /// let item_ref = unsafe { repo.make_ref(item_index) };
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
    pub unsafe fn remove_unchecked(&mut self, index: RefIndex) {
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
    /// let item_a_ref = repo.insert(10);
    /// let item_b_ref = repo.insert(20);
    ///
    /// assert!(repo.contains(&item_a_ref));
    /// assert!(repo.contains(&item_b_ref));
    ///
    /// // We do not change the content of referred items, but just re-inserting them.
    /// let item_a_content = repo.remove(&item_a_ref).unwrap();
    /// let item_b_content = repo.remove(&item_b_ref).unwrap();
    /// let item_a_ref_2 = repo.insert(item_a_content);
    /// let item_b_ref_2 = repo.insert(item_b_content);
    ///
    /// // Old weak references no longer valid.
    /// assert!(!repo.contains(&item_a_ref));
    /// assert!(!repo.contains(&item_b_ref));
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
    /// let item_a_index = repo.insert_index(10);
    /// let item_b_index = repo.insert_index(20);
    ///
    /// // This is safe because `insert_index` returns valid index.
    /// let item_a_ref = unsafe { repo.make_ref(item_a_index) };
    /// let item_b_ref = unsafe { repo.make_ref(item_b_index) };
    ///
    /// assert!(repo.contains(&item_a_ref));
    /// assert!(repo.contains(&item_b_ref));
    ///
    /// // Forcefully raises Repository version.
    /// repo.commit();
    ///
    /// // This is safe because the items referred by index are still exist in this repository.
    /// unsafe {
    ///     repo.upgrade(item_a_index);
    ///     repo.upgrade(item_b_index);
    /// }
    ///
    /// // Previously created weak references no longer valid.
    /// assert!(!repo.contains(&item_a_ref));
    /// assert!(!repo.contains(&item_b_ref));
    ///
    /// // We can still create new weak references using these indices.
    /// let item_a_ref_2 = unsafe { repo.make_ref(item_a_index) };
    /// let item_b_ref_2 = unsafe { repo.make_ref(item_b_index) };
    ///
    /// assert!(repo.contains(&item_a_ref_2));
    /// assert!(repo.contains(&item_b_ref_2));
    /// ```
    ///
    /// Note, if an API user misses to call Commit function, it will not lead to undefined behavior,
    /// but in this case the Upgrade function does not guarantee version upgrade.
    ///
    /// **Safety:**
    ///   - An entry indexed by `index` exists in this collection in Occupied or Reserved state.
    #[inline(always)]
    pub unsafe fn upgrade(&mut self, index: RefIndex) {
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
    Vacant(RefIndex),
    Occupied { data: T, revision: RefVersion },
    Reserved { revision: RefVersion },
}
