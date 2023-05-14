use crate::std::*;

/// A globally unique identifier of the data container.
///
/// ```rust
/// use lady_deirdre::arena::Id;
///
/// let id_a = Id::new();
/// let id_b = Id::new();
///
/// // Id is equals to itself.
/// assert_eq!(id_a, id_a);
///
/// // Id is equals to its copy.
/// assert_eq!(id_a, *(&id_a));
///
/// // Id is never equals to another Id.
/// assert_ne!(id_a, id_b);
///
/// // Id is never equals the Nil Id.
/// assert_ne!(&id_a, Id::nil());
///
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Id {
    inner: u64,
}

impl Ord for Id {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl PartialOrd for Id {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Debug for Id {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&self.inner, formatter)
    }
}

impl Display for Id {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.inner, formatter)
    }
}

impl AsRef<u64> for Id {
    #[inline(always)]
    fn as_ref(&self) -> &u64 {
        &self.inner
    }
}

impl Borrow<u64> for Id {
    #[inline(always)]
    fn borrow(&self) -> &u64 {
        &self.inner
    }
}

impl Id {
    /// Returns the next non-nil instance of [Id].
    ///
    /// There could be up to a half of [u64::MAX] unique instances of [Id] per process.
    /// An attempt to allocate more instances will panic.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Id;
    ///
    /// assert!(!Id::new().is_nil());
    ///
    /// ```
    #[inline(always)]
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        const HALF: u64 = u64::MAX / 2;

        let next = COUNTER.fetch_add(1, AtomicOrdering::SeqCst);

        if next > HALF {
            COUNTER.fetch_sub(1, AtomicOrdering::SeqCst);

            panic!("Id internal counter overflow.");
        }

        Self { inner: next }
    }

    /// Returns a static reference to the [Id] instance that considered to be invalid.
    ///
    /// Nil identifiers normally don't refer valid data.
    ///
    /// ```rust
    /// use lady_deirdre::arena::Id;
    ///
    /// assert!(Id::nil().is_nil());
    ///
    /// ```
    #[inline(always)]
    pub const fn nil() -> Self {
        Id { inner: 0 }
    }

    /// Returns `true` if the [Id] instance refers invalid data.
    #[inline(always)]
    pub const fn is_nil(self) -> bool {
        self.inner == 0
    }

    /// Returns [u64] inner representation of [Id].
    ///
    /// A zero value corresponds to the [nil](Id::nil) identifier.
    #[inline(always)]
    pub const fn into_inner(self) -> u64 {
        self.inner
    }
}

/// A convenient interface for objects that persist or refer globally unique data.
///
/// This interface normally should be implemented for collections of globally unique data, and for
/// weak references into such collections.
pub trait Identifiable {
    /// Returns a reference to a globally unique identifier of the data container this object
    /// belongs to.  
    fn id(&self) -> Id;
}
