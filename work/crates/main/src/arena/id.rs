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
    borrow::Borrow,
    fmt::{Debug, Display, Formatter},
    ops::Deref,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::sync::{Lazy, Table};

/// A globally unique identifier of a compilation unit (or a similar object).
///
/// An instance created with the [Id::new] function is globally unique
/// within the current process, such that each two instances which are not
/// copies of each other are always not equal.
///
/// Under the hood, Id is a simple wrapper of a [u64] value. The constructor
/// function assigns a non-zero number to the Id instance atomically increasing
/// its inner id counter.
///
/// The [nil](Id::nil) ids are special identifiers backed by the zero number,
/// which denote invalid identifiers; objects that do not identify any
/// compilation unit.
///
/// Normally, the compilation units should always by identifiable by the unique
/// instances of Id. In particular, all Lady Deirdre's compilation units and
/// similar objects (e.g. [Documents](crate::units::Document) and
/// [TokenBuffers](crate::lexis::TokenBuffer)) satisfy this rule. However,
/// for the third-party compilation unit types, it is the implementor's
/// responsibility to follow this requirement.
///
/// The related trait [Identifiable] is assumed to be implemented for each
/// compilation unit and the types that address things
/// (e.g. [NodeRefs](crate::syntax::NodeRef)) related to these units.
///
/// This trait helps an API user to look up for the compilation units by ids.
///
/// Finally, the Id object provides a possibility to associate this identifier
/// instance (and any copy of it) with a custom and possibly non-unique string
/// name. This name is used in the [Debug] and [Display] implementations of Id
/// and helps you visually distinguish between compilation unit identifiers in
/// a multi-unit compiler. For example, you can assign a file name to a
/// [Document](crate::units::Document) using this feature.
///
/// It is important to note that Lady Deirdre does not have a built-in names
/// cleanup mechanism. The Id object is a [Copy] type, and it is your
/// responsibility as the author of the compilation unit to remove the name
/// from the Id of the last available copy of that Id to avoid memory leaks.
///
/// However, all built-in compilation unit types within this crate automatically
/// clean up their Id names.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Id {
    inner: u64,
}

impl Identifiable for Id {
    #[inline(always)]
    fn id(&self) -> Id {
        *self
    }
}

impl Debug for Id {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        if self.is_nil() {
            return formatter.write_str("Nil");
        }

        formatter.write_str("Id(")?;
        Debug::fmt(&self.inner, formatter)?;

        let name = self.name();

        if !name.is_empty() {
            formatter.write_str(", ")?;
            Debug::fmt(&name, formatter)?;
        }

        formatter.write_str(")")
    }
}

impl Display for Id {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        if self.is_nil() {
            return formatter.write_str("Nil");
        }

        let name = self.name();

        match name.is_empty() {
            true => Display::fmt(&self.inner, formatter),
            false => Debug::fmt(&name, formatter),
        }
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
    /// Creates a new identifier.
    ///
    /// **Panic**
    ///
    /// Panics if this function has been called more than `u64::MAX / 2` times.
    /// In other words, Lady Deirdre currently allows no more than this number
    /// of unique identifiers per a single process.
    #[inline(always)]
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        const HALF: u64 = u64::MAX / 2;

        let next = COUNTER.fetch_add(1, Ordering::SeqCst);

        if next > HALF {
            COUNTER.fetch_sub(1, Ordering::SeqCst);

            panic!("Id internal counter overflow.");
        }

        Self { inner: next }
    }

    /// Returns an identifier that intentionally does not address
    /// any compilation unit.
    #[inline(always)]
    pub const fn nil() -> Self {
        Id { inner: 0 }
    }

    /// Returns true if this identifier intentionally does not address any
    /// compilation unit.
    #[inline(always)]
    pub const fn is_nil(self) -> bool {
        self.inner == 0
    }

    /// Returns an inner number that denotes this identifier.
    ///
    /// If the returning value is zero, this identifier is [nil](Self::nil).
    #[inline(always)]
    pub const fn into_inner(self) -> u64 {
        self.inner
    }

    /// Returns a clone of a name of this identifier.
    ///
    /// The returning string is empty if there is no name associated with this
    /// identifier.
    #[inline(always)]
    pub fn name(&self) -> String {
        if self.is_nil() {
            return String::new();
        }

        ID_NAMES
            .get(self)
            .map(|name_guard| name_guard.deref().clone())
            .unwrap_or_default()
    }

    /// Associates this identifier (and all copies of it) with a user-facing
    /// string.
    ///
    /// If the `name` parameter is an empty string, this function removes
    /// a name from this identifier (and from all copies of it). The behavior
    /// is similar to the [clear_name](Self::clear_name) function.
    ///
    /// Note that the identifier names is a subject to manual cleanup to avoid
    /// memory leaks. See the [Id] specification for details.
    ///
    /// **Panic**
    ///
    /// This function panics if the id is [nil](Self::nil).
    #[inline(always)]
    pub fn set_name(&self, name: impl Into<String>) {
        if self.is_nil() {
            panic!("An attempt to set a name to the Nil identifier.");
        }

        let name = name.into();

        let _ = match name.is_empty() {
            true => ID_NAMES.remove(self),
            false => ID_NAMES.insert(*self, name),
        };
    }

    /// Removes the name (and frees its memory) associated with this identifier.
    ///
    /// If the identifier does not have a name, this function does nothing.
    ///
    /// **Panic**
    ///
    /// This function panics if the id is [nil](Self::nil).
    #[inline(always)]
    pub fn clear_name(&self) -> bool {
        if self.is_nil() {
            panic!("An attempt to unset a name of the Nil identifier.");
        }

        ID_NAMES.remove(self).is_some()
    }
}

/// A helper trait that denotes a compilation unit to which this object belongs.
///
/// This trait helps an API user to look up for the compilation units by calling
/// the [id](Identifiable::id) function that returns an [identifier](Id) of
/// the related compilation unit.
///
/// The trait is assumed to be implemented on all types of compilation units
/// (and similar objects). In this scenario, an API user uses the id function
/// to get the unit identifier. For instance, you can use this identifier as
/// a key of a hash map of the compiler's units.
///
/// Also, this trait should be implemented on all types that belong to a specific
/// compilation unit. An API user would use the id function to reveal
/// corresponding unit by looking up the unit by its identifier.
pub trait Identifiable {
    /// Returns the globally unique identifier of the compilation unit to which
    /// this object belongs.
    fn id(&self) -> Id;
}

static ID_NAMES: Lazy<Table<Id, String>> = Lazy::new(|| Table::new());

pub(crate) enum SubId {
    Own(Id),
    Fork(Id),
}

impl PartialEq for SubId {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id().eq(&other.id())
    }
}

impl Eq for SubId {}

impl Identifiable for SubId {
    #[inline(always)]
    fn id(&self) -> Id {
        match self {
            SubId::Own(id) => *id,
            SubId::Fork(id) => *id,
        }
    }
}

impl Drop for SubId {
    fn drop(&mut self) {
        let Self::Own(id) = self else {
            return;
        };

        id.clear_name();
    }
}

impl SubId {
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self::Own(Id::new())
    }

    #[inline(always)]
    pub(crate) fn fork(id: Id) -> Self {
        Self::Fork(id)
    }
}
