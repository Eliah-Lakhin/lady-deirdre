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

use crate::{format::PrintString, report::debug_unreachable, std::*};

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
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        if self.is_nil() {
            return formatter.write_str("Nil");
        }

        formatter.write_str("Id(")?;
        Debug::fmt(&self.inner, formatter)?;

        let name = self.name();

        if name.is_empty() {
            formatter.write_str(", ")?;
            Debug::fmt(&name, formatter)?;
        }

        formatter.write_str(")")
    }
}

impl Display for Id {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
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

    #[inline(always)]
    pub fn name(&self) -> PrintString<'static> {
        if self.is_nil() {
            return PrintString::empty();
        }

        let key = *self;

        IdNames::access(move |map| map.get(&key).cloned().unwrap_or_default())
    }

    #[inline(always)]
    pub fn set_name(&self, name: impl Into<PrintString<'static>>) {
        if self.is_nil() {
            panic!("An attempt to set a name to the Nil identifier.");
        }

        let key = *self;
        let name = name.into();

        IdNames::access(move |map| {
            let _ = match name.is_empty() {
                true => map.remove(&key),
                false => map.insert(key, name),
            };
        });
    }

    #[inline(always)]
    pub fn clear_name(&self) -> bool {
        if self.is_nil() {
            panic!("An attempt to unset a name of the Nil identifier.");
        }

        let key = *self;

        IdNames::access(move |map| map.remove(&key).is_some())
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

struct IdNames {
    lock: AtomicBool,
    data: UnsafeCell<Option<StdMap<Id, PrintString<'static>>>>,
}

// Safety: Access is protected by a mutex.
unsafe impl Send for IdNames {}

// Safety: Access is protected by a mutex.
unsafe impl Sync for IdNames {}

impl Drop for IdNames {
    fn drop(&mut self) {
        self.access_raw(|raw| {
            *raw = None;
        });
    }
}

impl IdNames {
    #[inline(always)]
    const fn new() -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(None),
        }
    }

    #[inline(always)]
    fn access<R>(
        grant: impl FnOnce(&mut StdMap<Id, PrintString<'static>>) -> R + Send + Sync + 'static,
    ) -> R {
        static GLOBAL: IdNames = IdNames::new();

        GLOBAL.access_raw(move |raw| {
            let map = match raw {
                Some(map) => map,
                None => {
                    *raw = Some(StdMap::new());

                    match raw {
                        Some(map) => map,

                        // Safety: The Option initialized above.
                        None => unsafe {
                            debug_unreachable!("IdNames map initialization failure.")
                        },
                    }
                }
            };

            grant(map)
        })
    }

    fn access_raw<R>(
        &self,
        grant: impl FnOnce(&mut Option<StdMap<Id, PrintString<'static>>>) -> R + Send + Sync + 'static,
    ) -> R {
        loop {
            let borrow = self.lock.load(AtomicOrdering::Relaxed);

            if borrow {
                spin_loop();
                continue;
            }

            if self
                .lock
                .compare_exchange_weak(
                    false,
                    true,
                    AtomicOrdering::Acquire,
                    AtomicOrdering::Relaxed,
                )
                .is_err()
            {
                spin_loop();
                continue;
            }

            break;
        }

        let result = {
            // Safety: The data locked atomically.
            let data = unsafe { &mut *self.data.get() };

            grant(data)
        };

        if self
            .lock
            .compare_exchange(
                true,
                false,
                AtomicOrdering::Release,
                AtomicOrdering::Relaxed,
            )
            .is_err()
        {
            // Safety: The data locked atomically.
            unsafe { debug_unreachable!("IdNames mutex release failure.") }
        }

        result
    }
}
