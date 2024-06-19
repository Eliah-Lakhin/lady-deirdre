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
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::ManuallyDrop,
    ptr::NonNull,
    sync::{
        atomic,
        atomic::{fence, AtomicUsize},
    },
};

use crate::report::system_panic;

const REF_MAX: usize = usize::MAX - 1;

/// A reference-counting pointer.
///
/// This object is similar to the standard [Arc](std::sync::Arc)
/// reference-counting pointer, such that all clones of the same Shared instance
/// point to the same data allocation and provide read-only access
/// to this allocation. The allocation is freed once the last instance of
/// a Shared is dropped.
///
/// There are two main differences between Arc and Shared:
///
///  1. Shared does not have a [Weak](std::sync::Weak) counterpart. All Shared
///     clones are "strong" references. Therefore, the clone and drop operations
///     are slightly cheaper, and Shared allocates one machine-word less than
///     Arc.
///  2. Shared provides read-access through the [AsRef] implementation rather
///     than [Deref](std::ops::Deref). This makes its API more ergonomic to
///     use as a "builder" because [Shared::get_mut] and [Shared::make_mut]
///     operate on `&mut self`. However, it is less ergonomic for reading
///     because the user has to call `my_shared.as_ref()` explicitly to read
///     the underlying data.
#[repr(transparent)]
pub struct Shared<T: ?Sized> {
    inner: NonNull<SharedInner<T>>,
    _phantom: PhantomData<SharedInner<T>>,
}

// Safety: Shared data access is guarded by the atomic operations.
unsafe impl<T: Send + Sync> Send for Shared<T> {}

// Safety: Shared data access is guarded by the atomic operations.
unsafe impl<T: Send + Sync> Sync for Shared<T> {}

impl<T: Debug> Debug for Shared<T> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.as_ref(), formatter)
    }
}

impl<T: Display> Display for Shared<T> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.as_ref(), formatter)
    }
}

impl<T: Eq> PartialEq<Shared<T>> for Shared<T> {
    #[inline(always)]
    fn eq(&self, other: &Shared<T>) -> bool {
        if self.addr().eq(&other.addr()) {
            return true;
        }

        self.as_ref().eq(other.as_ref())
    }
}

impl<T: Eq> Eq for Shared<T> {}

impl<T: Ord> PartialOrd for Shared<T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> Ord for Shared<T> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        if self.addr().eq(&other.addr()) {
            return Ordering::Equal;
        }

        self.as_ref().cmp(other.as_ref())
    }
}

impl<T: Hash> Hash for Shared<T> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<T: Default> Default for Shared<T> {
    #[inline(always)]
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        let counter = {
            // Safety: Shared owns a pointer to valid data leaked from the Box.
            let inner = unsafe { self.inner.as_ref() };

            inner.counter.fetch_add(1, atomic::Ordering::Relaxed)
        };

        if counter > REF_MAX {
            system_panic!("Too many Shared references.");
        }

        Self {
            inner: self.inner,
            _phantom: Default::default(),
        }
    }
}

impl<T: ?Sized> Drop for Shared<T> {
    fn drop(&mut self) {
        let counter = {
            // Safety: Shared owns a pointer to valid data leaked from the Box.
            let inner = unsafe { self.inner.as_ref() };

            inner.counter.fetch_sub(1, atomic::Ordering::Release)
        };

        if counter == 1 {
            fence(atomic::Ordering::Acquire);

            // Safety:
            //   1. Shared owns a pointer to valid data leaked from the Box.
            //   2. The drop operation is ordered by the Acquire fence.
            let _ = unsafe { Box::from_raw(self.inner.as_ptr()) };
        }
    }
}

impl<T> AsRef<T> for Shared<T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        // Safety:
        //   1. Shared owns a pointer to valid data leaked from the Box.
        //   2. If there are no other instances of Shared, immutable access can be granted.
        //   3. If there are more than one instance, none of them acquire mutable access
        //      because of the inner counter.
        let inner = unsafe { self.inner.as_ref() };

        &inner.data
    }
}

impl<T> Shared<T> {
    /// Creates a new Shared.
    #[inline(always)]
    pub fn new(data: T) -> Self {
        let inner = Box::new(SharedInner {
            counter: AtomicUsize::new(1),
            data,
        });

        Self {
            // Safety: Box leaked pointer is never null.
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(inner)) },
            _phantom: PhantomData,
        }
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Returns None if there are other live Shared instance to
    /// the same allocation.
    #[inline(always)]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        let counter = {
            // Safety: Shared owns a pointer to valid data leaked from the Box.
            let inner = unsafe { self.inner.as_ref() };

            inner.counter.load(atomic::Ordering::Acquire)
        };

        if counter == 1 {
            // Safety:
            //   1. Shared owns a pointer to valid data leaked from the Box.
            //   2. This ownership is unique because of the counter value.
            //   3. No new Shared clones may be created in between
            //      because this Shared instance is borrowed mutably.
            let inner = unsafe { self.inner.as_mut() };

            return Some(&mut inner.data);
        }

        None
    }

    /// Returns a mutable reference to the underlying data without additional
    /// checks.
    ///
    /// **Safety**
    ///
    /// There are no other live Shared instance to the same allocation.
    #[inline(always)]
    pub unsafe fn get_mut_unchecked(&mut self) -> &mut T {
        #[cfg(debug_assertions)]
        {
            let counter = {
                // Safety: Shared owns a pointer to valid data leaked from the Box.
                let inner = unsafe { self.inner.as_ref() };

                inner.counter.load(atomic::Ordering::Acquire)
            };

            if counter == 1 {
                panic!("Shared::get_mut_unchecked safety requirements violation.");
            }
        }

        // Safety:
        //   1. Shared owns a pointer to valid data leaked from the Box.
        //   2. Since there are no active borrows into the inner data (upheld by the caller).
        //   3. No new Shared clones may be created in between
        //      because this Shared instance is borrowed mutably.
        let inner = unsafe { self.inner.as_mut() };

        &mut inner.data
    }

    /// Makes a mutable reference to the underlying data.
    ///
    /// If there are no other live Shared instances to the same allocation,
    /// returns a mutable reference to the current allocation
    /// (similarly to [get_mut](Self::get_mut)).
    ///
    /// Otherwise, replaces this Shared instance with a new one by cloning
    /// the underlying data into a new allocation. Then, returns a mutable
    /// reference to this new independent allocation.
    pub fn make_mut(&mut self) -> &mut T
    where
        T: Clone,
    {
        let unique = {
            // Safety: Shared owns a pointer to valid data leaked from the Box.
            let inner = unsafe { self.inner.as_ref() };

            inner
                .counter
                .compare_exchange(1, 0, atomic::Ordering::Acquire, atomic::Ordering::Relaxed)
                .is_ok()
        };

        match unique {
            true => {
                // Safety: Shared owns a pointer to valid data leaked from the Box.
                let inner = unsafe { self.inner.as_ref() };

                inner.counter.store(1, atomic::Ordering::Release);
            }

            false => {
                *self = Self::new(self.as_ref().clone());
            }
        }

        // Safety:
        //   1. Shared owns a pointer to valid data leaked from the Box.
        //   2. Owner uniqueness ensured above.
        //   3. No new Shared clones may be created in between
        //      because this Shared instance is borrowed mutably.
        let inner = unsafe { self.inner.as_mut() };

        &mut inner.data
    }

    /// Takes data from this Shared instance.
    ///
    /// Returns None if there are other live Shared instance to
    /// the same allocation.
    #[inline(always)]
    pub fn into_inner(self) -> Option<T> {
        let this = ManuallyDrop::new(self);

        let counter = {
            // Safety: Shared owns a pointer to valid data leaked from the Box.
            let inner = unsafe { this.inner.as_ref() };

            inner.counter.fetch_sub(1, atomic::Ordering::Release)
        };

        if counter == 1 {
            fence(atomic::Ordering::Acquire);

            // Safety:
            //   1. Shared owns a pointer to valid data leaked from the Box.
            //   2. The ownership transfer operation is ordered by the Acquire fence.
            //   3. Shared will not be dropped twice because of the `ManuallyDrop` wrapper.
            let inner = unsafe { Box::from_raw(this.inner.as_ptr()) };

            return Some(inner.data);
        }

        None
    }

    /// Returns the address of the Shared allocation.
    #[inline(always)]
    pub fn addr(&self) -> usize {
        self.inner.as_ptr() as usize
    }
}

struct SharedInner<T: ?Sized> {
    counter: AtomicUsize,
    data: T,
}

#[cfg(test)]
mod tests {
    use crate::sync::Shared;

    #[test]
    fn test_shared() {
        let mut shared1 = Shared::new(100);

        assert_eq!(shared1.as_ref(), &100);

        *shared1.get_mut().unwrap() += 50;

        assert_eq!(shared1.as_ref(), &150);

        {
            let mut shared2 = shared1.clone();

            assert_eq!(shared2.as_ref(), &150);

            assert!(shared1.get_mut().is_none());
            assert!(shared2.get_mut().is_none());

            *shared2.make_mut() += 50;

            assert_eq!(shared1.as_ref(), &150);
            assert_eq!(shared2.as_ref(), &200);

            *shared1.get_mut().unwrap() += 25;

            assert_eq!(shared2.into_inner(), Some(200));
        }

        assert_eq!(shared1.as_ref(), &175);
        assert!(shared1.get_mut().is_some());
        assert_eq!(shared1.into_inner(), Some(175));
    }
}
