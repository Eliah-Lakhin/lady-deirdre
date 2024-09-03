////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::fmt::{Debug, Formatter};

/// A [versioned index](Entry) that does not address any value within any
/// possible storage.
///
/// The value of this static equals to the [Entry::nil] value.
pub static NIL_ENTRY: Entry = Entry::nil();

/// A non-versioned index of the entry within the [Repo](crate::arena::Repo).
///
/// A value of [usize::MAX] denotes an invalid index.
pub type EntryIndex = usize;

/// A version of the [Repo](crate::arena::Repo) under which the entry
/// has been added into the repository.
///
/// Note that the valid values of this type start from 1. If the value equals
/// zero, this value denotes an [entry](Entry) that does not belong to any
/// repository.
///
/// A value of [usize::MAX] also denotes an invalid version.
pub type EntryVersion = usize;

/// A versioned index of the entry within the [Repo](crate::arena::Repo).
///
/// This object denotes a unique (within the repository) index of the entry.
/// Each time the user [inserts](crate::arena::Repo::insert) or
/// [reserves](crate::arena::Repo::reserve_entry) an entry in the repo,
/// this entry receive a unique pair of the index and version numbers.
///
/// This object is also intended to address objects outside of a repository
/// (e.g., objects inside simple vectors). If the version value of this object
/// is zero, the [Entry] object denotes a possibly valid value stored somewhere
/// outside of a repository.
///
/// A pair of [usize::MAX] values of the Entry's index and version numbers
/// denotes an Entry which is intentionally invalid; an Entry that does not
/// address any value within any possible type of storage.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entry {
    /// A non-versioned part of the index.
    pub index: EntryIndex,

    /// A version of the repository under which the entry has been added
    /// into the repository.
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
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
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
    /// Returns a versioned index that intentionally does not address any value
    /// within any possible storage.
    ///
    /// If you need just a static reference to the nil Entry, use
    /// the predefined [NIL_ENTRY] static.
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            index: EntryIndex::MAX,
            version: EntryVersion::MAX,
        }
    }

    /// Returns true, if this versioned index intentionally does not address
    /// any value within any possible storage.
    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.index == EntryIndex::MAX && self.version == EntryVersion::MAX
    }
}
