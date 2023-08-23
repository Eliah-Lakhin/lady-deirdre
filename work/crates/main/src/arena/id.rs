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
/// assert_ne!(id_a, Id::nil());
///
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Id {
    inner: u64,
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
