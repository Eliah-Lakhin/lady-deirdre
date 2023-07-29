use crate::std::*;

/// An index into the inner array of items of the [Sequence](crate::arena::Sequence) collection, or
/// into the inner array of entries of the [Repository](crate::arena::Repository) collection.
pub type RefIndex = usize;

/// A revision version of the entry inside [Repository](crate::arena::Repository) collection.
pub type RefVersion = usize;

/// A generic homogeneous weak reference into the Arena collection items.
///
/// This is a low-level interface. An API user normally does not need to construct or to inspect
/// into this interface manually unless you work on the extension of this Crate.
///
/// The Ref instances initially constructed by the [Repository](crate::arena::Repository) or by
/// the [Sequence](crate::arena::Sequence), or by a top-level API.
///
/// The reference considered to be either valid or invalid. The integrity of references is not
/// guaranteed by underlying collections or by the wrapper containers. For example, a Repository
/// collection can produce a valid reference to the item inside that collection, but later on the
/// data could obsolete(e.g. by removing an item from the collection). In this case the Ref instance
/// becomes invalid, and it could not be dereferenced to a valid item from that collection. In this
/// sense Ref is a "weak" reference.
///
/// The Ref instance is collection-independent, as such it could be interpreted in different ways
/// depending on applied collection, and in case of misinterpretation it could be dereferenced to a
/// wrong Item. Misinterpretation of the Ref instance(within the safe API) is a logical error, not
/// an undefined behavior.
///
/// See [module documentation](crate::arena) for details on how to avoid this problem in the end
/// API design.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ref {
    /// Indicates invalid reference.
    ///
    /// This type of reference cannot be dereferenced.
    Nil,

    /// Indicates a reference to a single data item that resides outside of the main collection.
    ///
    /// Such references considered to be always valid. This type of variants cannot be
    /// dereferenced by collection functions of the [arena](crate::arena) module. They supposed to be
    /// dereferenced by the top-level wrapper container functions.
    ///
    /// Primary variant is a helper variant to refer an inner single selected item that logically
    /// belongs to specified top-level wrapper container, but resides outside of the container's
    /// main collection.
    ///
    /// An example of such container is a [Cluster](crate::syntax::Cluster) container that has a
    /// [Cluster::primary](crate::syntax::Cluster::primary) field that resides near the
    /// [Cluster::nodes](crate::syntax::Cluster::nodes) collection field. A [Ref::Primary] variant
    /// would refer the "primary" field value of the Cluster instance in this case.
    Primary,

    /// Indicates a references to the Item inside the [Sequence](crate::arena::Sequence) collection.
    Sequence {
        /// An index into the inner array of the [Sequence](crate::arena::Sequence) collection.
        ///
        /// If the index is outside of array's bounds, the reference considered to be invalid, and
        /// is interpreted as a [Ref::Nil] variant. Otherwise the reference considered to be valid.
        index: RefIndex,
    },

    /// Indicates a references to the Item inside the [Repository](crate::arena::Repository)
    /// collection.
    ///
    /// The reference valid if and only if it refers Occupied entry inside corresponding Repository,
    /// and the version of the reference equals to the version of the indexed entry.
    ///
    /// For details see [Repository documentation](crate::arena::Repository).
    Repository {
        /// An index into the inner array of entries inside the
        /// [Repository](crate::arena::Repository) collection.
        ///
        /// If the index is outside of the Repository inner array bounds, the reference considered
        /// to be invalid, and is interpreted as a [Ref::Nil] variant.
        index: RefIndex,

        /// A version of the entry indexed by this variant into the inner array of entries inside
        /// the [Repository](crate::arena::Repository) collection.
        ///
        /// If the version held by this variant differs from the version of occupied entry in
        /// specified Repository instance, the reference considered to be invalid, and is
        /// interpreted as a [Ref::Nil] variant.
        version: RefVersion,
    },
}

impl Debug for Ref {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Ref::Nil => formatter.write_str("Nil"),
            Ref::Primary => formatter.write_str("Primary"),
            Ref::Sequence { index } => formatter.write_fmt(format_args!("Ref({index})")),
            Ref::Repository { index, version } => {
                formatter.write_fmt(format_args!("Ref({index}:{version})"))
            }
        }
    }
}

impl Ref {
    /// Returns true if the reference enum is a [Ref::Nil] variant.
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        match &self {
            Ref::Nil => true,
            _ => false,
        }
    }
}
