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

use crate::std::*;

/// An index into the inner array of items of the [Sequence](crate::arena::Sequence) collection, or
/// into the inner array of entries of the [Repository](crate::arena::Repository) collection.
pub type EntryIndex = usize;

/// A revision version of the entry inside [Repository](crate::arena::Repository) collection.
pub type EntryVersion = usize;

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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Entry {
    /// Indicates invalid reference.
    ///
    /// This type of reference cannot be dereferenced.
    #[default]
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
    /// [Cluster::nodes](crate::syntax::Cluster::nodes) collection field. A [Entry::Primary] variant
    /// would refer the "primary" field value of the Cluster instance in this case.
    Primary,

    /// Indicates a references to the Item inside the [Sequence](crate::arena::Sequence) collection.
    Seq {
        /// An index into the inner array of the [Sequence](crate::arena::Sequence) collection.
        ///
        /// If the index is outside of array's bounds, the reference considered to be invalid, and
        /// is interpreted as a [Ref::Nil] variant. Otherwise the reference considered to be valid.
        index: EntryIndex,
    },

    /// Indicates a references to the Item inside the [Repository](crate::arena::Repository)
    /// collection.
    ///
    /// The reference valid if and only if it refers Occupied entry inside corresponding Repository,
    /// and the version of the reference equals to the version of the indexed entry.
    ///
    /// For details see [Repository documentation](crate::arena::Repository).
    Repo {
        /// An index into the inner array of entries inside the
        /// [Repository](crate::arena::Repository) collection.
        ///
        /// If the index is outside of the Repository inner array bounds, the reference considered
        /// to be invalid, and is interpreted as a [Ref::Nil] variant.
        index: EntryIndex,

        /// A version of the entry indexed by this variant into the inner array of entries inside
        /// the [Repository](crate::arena::Repository) collection.
        ///
        /// If the version held by this variant differs from the version of occupied entry in
        /// specified Repository instance, the reference considered to be invalid, and is
        /// interpreted as a [Ref::Nil] variant.
        version: EntryVersion,
    },
}

impl Debug for Entry {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        match self {
            Entry::Nil => formatter.write_str("Nil"),
            Entry::Primary => formatter.write_str("Primary"),
            Entry::Seq { index } => formatter.write_fmt(format_args!("Entry({index})")),
            Entry::Repo { index, version } => {
                formatter.write_fmt(format_args!("Entry({index}:{version})"))
            }
        }
    }
}

impl Entry {
    /// Returns true if the reference enum is a [Entry::Nil] variant.
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        match &self {
            Entry::Nil => true,
            _ => false,
        }
    }
}
