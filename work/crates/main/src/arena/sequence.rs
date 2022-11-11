use crate::{
    arena::{Ref, RefIndex},
    std::*,
};

/// A convenient wrapper over the FIFO vector.
///
/// This interface wraps a vector of items that supposed to grow as a FIFO stack on initialization,
/// but later on will be used in mostly immutable way during lifetime.
///
/// Sequence interface is compatible with [Ref](crate::arena::Ref) weak references framework.
///
/// In contrast to [Repository](crate::arena::Repository) Sequence does not have a version
/// management mechanism as the collection supposed to be immutable during lifetime. For the sake
/// of simplicity, there are no strict rules to enforce distinction between the initialization
/// and the usage stages, so an API user should utilize this collection with care.
///
/// Since the Sequence collection uses Rust's [Vector](Vec) under the hood, sequential iteration
/// over this collection items does not suffer from the cache misses issue.
///
/// ```rust
/// use lady_deirdre::arena::Sequence;
///
/// let mut sequence = Sequence::<u8>::default();
///
/// sequence.push(10);
/// sequence.push(20);
///
/// let first_item_ref = Sequence::<u8>::make_ref(0);
///
/// assert_eq!(sequence.get(&first_item_ref), Some(&10));
///
/// // Inner function returns a slice of the inner vector data.
/// assert_eq!(&sequence.inner()[1], &20);
/// ```
///
/// Alternatively, an API user can set up a Vector instance and then turn it into Sequence:
///
/// ```rust
/// use lady_deirdre::arena::Sequence;
///
/// let mut sequence = Sequence::<u8>::from(vec![10, 20]);
///
/// let first_item_ref = Sequence::<u8>::make_ref(0);
///
/// assert_eq!(sequence.get(&first_item_ref), Some(&10));
///
/// // Receiving original inner vector from this collection.
/// let original_vector = sequence.into_vec();
///
/// assert_eq!(&original_vector[1], &20);
/// ```
#[repr(transparent)]
pub struct Sequence<T> {
    entries: Vec<T>,
}

impl<T> Default for Sequence<T> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> Debug for Sequence<T> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str("Sequence")
    }
}

impl<T> From<Vec<T>> for Sequence<T> {
    #[inline(always)]
    fn from(entries: Vec<T>) -> Self {
        Self { entries }
    }
}

impl<T> Sequence<T> {
    /// Creates a new collection instance with pre-allocated memory for at least `capacity` items
    /// to be stored in.
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
        }
    }

    /// Pushes an item on the top of the Sequence inner FIFO vector.
    ///
    /// This function is supposed to be used on the instance initialization stage only.
    ///
    /// Returns valid reference index to refer added item. This index can be used to create valid
    /// [Ref](crate::arena::Ref) instance using [make_ref](crate::arena::Sequence::make_ref)
    /// function.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Sequence;
    ///
    /// let mut sequence = Sequence::<u8>::default();
    ///
    /// let item_index = sequence.push(10);
    /// let item_ref = Sequence::<u8>::make_ref(item_index);
    ///
    /// assert_eq!(sequence.get(&item_ref), Some(&10));
    /// ```
    #[inline(always)]
    pub fn push(&mut self, data: T) -> RefIndex {
        let index = self.entries.len();

        self.entries.push(data);

        index
    }

    /// Removes an item from the top of the Sequence inner FIFO vector.
    ///
    /// This function is supposed to be used on the instance initialization stage only.
    ///
    /// Returns removed item if the Sequence is not empty. Otherwise returns [None].
    ///
    /// ```rust
    /// use lady_deirdre::arena::Sequence;
    ///
    /// let mut sequence = Sequence::<u8>::default();
    ///
    /// let _ = sequence.push(10);
    /// let _ = sequence.push(20);
    ///
    /// assert_eq!(sequence.pop(), Some(20));
    /// assert_eq!(sequence.pop(), Some(10));
    /// assert_eq!(sequence.pop(), None);
    /// ```
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        self.entries.pop()
    }

    /// Reserves capacity to for at least `additional` items to be inserted on top of this
    /// collection.
    #[inline(always)]
    pub fn reserve(&mut self, additional: usize) {
        self.entries.reserve(additional)
    }

    /// Returns `true` if referred item exists in this collection.
    ///
    /// ```rust
    /// use lady_deirdre::arena::{Repository, Sequence};
    ///
    /// let mut repo = Repository::<u8>::default();
    ///
    /// let repo_item_ref = repo.insert(10);
    ///
    /// let mut seq = Sequence::<u8>::default();
    ///
    /// let seq_item_index = seq.push(20);
    /// let seq_item_ref = Sequence::<u8>::make_ref(seq_item_index);
    ///
    /// // Repository item reference is invalid to the Sequence collection.
    /// assert!(!seq.contains(&repo_item_ref));
    ///
    /// // Inserted Sequence item reference is a valid reference for this Sequence collection.
    /// assert!(seq.contains(&seq_item_ref));
    #[inline]
    pub fn contains(&self, reference: &Ref) -> bool {
        match reference {
            Ref::Sequence { index } if self.entries.len() > *index => true,

            _ => false,
        }
    }

    /// Tries to dereference referred item.
    ///
    /// Returns [None] if referred item does not exist in this collection.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Sequence;
    ///
    /// let mut seq = Sequence::<u8>::default();
    ///
    /// let item_index = seq.push(10);
    /// let item_ref = Sequence::<u8>::make_ref(item_index);
    ///
    /// assert_eq!(seq.get(&item_ref), Some(&10));
    ///
    /// let _ = seq.pop();
    ///
    /// // Referred item no longer exists in this collection.
    /// assert_eq!(seq.get(&item_ref), None);
    #[inline]
    pub fn get(&self, reference: &Ref) -> Option<&T> {
        match reference {
            Ref::Sequence { index } => self.entries.get(*index),

            _ => None,
        }
    }

    /// Tries to mutably dereference referred item.
    ///
    /// Returns [None] if referred item does not exist in this collection.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Sequence;
    ///
    /// let mut seq = Sequence::<u8>::default();
    ///
    /// let item_index = seq.push(10);
    /// let item_ref = Sequence::<u8>::make_ref(item_index);
    ///
    /// *(seq.get_mut(&item_ref).unwrap()) = 20;
    ///
    /// assert_eq!(seq.get(&item_ref), Some(&20));
    #[inline]
    pub fn get_mut(&mut self, reference: &Ref) -> Option<&mut T> {
        match reference {
            Ref::Sequence { index } => self.entries.get_mut(*index),

            _ => None,
        }
    }

    /// Removes all items from this collection preserving allocated memory.
    ///
    /// All references belong to this collection are implicitly turn to invalid. However, if an API
    /// user inserts new items later on, previously created references would become valid again as
    /// the Sequence collection does not manage versions.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Moves inner vector of items out of this collection.
    #[inline(always)]
    pub fn into_vec(self) -> Vec<T> {
        self.entries
    }

    /// Returns item weak reference by internal index.
    ///
    /// This index could be received, for example, from the [push](Sequence::push) function.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Sequence;
    ///
    /// let mut seq = Sequence::<u8>::default();
    ///
    /// let item_index = seq.push(10);
    /// let item_ref = Sequence::<u8>::make_ref(item_index);
    ///
    /// assert_eq!(seq.get(&item_ref), Some(&10));
    ///
    /// let _ = seq.pop();
    ///
    /// // Referred item no longer exists in this collection.
    /// assert_eq!(seq.get(&item_ref), None);
    ///
    /// // Note that however Sequence collection does not manage versions, as such inserting a new
    /// // item inside this collection would turn previously created weak reference to a valid
    /// // reference again, and that old reference would refer a new item instance.
    ///
    /// let _ = seq.push(20);
    /// assert_eq!(seq.get(&item_ref), Some(&20));
    #[inline(always)]
    pub fn make_ref(index: RefIndex) -> Ref {
        Ref::Sequence { index }
    }

    /// Returns an immutable slice of all items inside this collection.
    ///
    /// Returned data slice is indexable by indices received from the [push](Sequence::push)
    /// function.
    #[inline(always)]
    pub fn inner(&self) -> &[T] {
        &self.entries[..]
    }

    /// Returns a mutable slice of all items inside this collection.
    ///
    /// Returned data slice is indexable by indices received from the [push](Sequence::push)
    /// function.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut [T] {
        &mut self.entries[..]
    }
}
