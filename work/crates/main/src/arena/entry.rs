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
/// into the inner array of entries of the [Repository](crate::arena::Repo) collection.
pub type EntryIndex = usize;

/// A revision version of the entry inside [Repository](crate::arena::Repo) collection.
pub type EntryVersion = usize;

/// A generic homogeneous weak reference into the Arena collection items.
///
/// This is a low-level interface. An API user normally does not need to construct or to inspect
/// into this interface manually unless you work on the extension of this Crate.
///
/// The Ref instances initially constructed by the [Repository](crate::arena::Repo) or by
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
pub struct Entry {
    pub index: EntryIndex,
    pub version: EntryVersion,
}

impl Default for Entry {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl Debug for Entry {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        if self.is_nil() {
            return formatter.write_str("Nil");
        }

        if self.version == 0 {
            return formatter.write_fmt(format_args!("{}", self.index));
        }

        formatter.write_fmt(format_args!("{}.{}", self.index, self.version))
    }
}

impl Entry {
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            index: EntryIndex::MAX,
            version: EntryVersion::MAX,
        }
    }

    /// Returns true if the reference enum is a [Entry::Nil] variant.
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.index == EntryIndex::MAX && self.version == EntryVersion::MAX
    }
}
