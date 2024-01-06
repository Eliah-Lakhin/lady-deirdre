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

use crate::{report::system_panic, std::*};

const REF_MAX: usize = usize::MAX - 1;

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
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(self.as_ref(), formatter)
    }
}

impl<T: Display> Display for Shared<T> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
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

            inner.counter.fetch_add(1, AtomicOrdering::Relaxed)
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

            inner.counter.fetch_sub(1, AtomicOrdering::Release)
        };

        if counter == 1 {
            fence(AtomicOrdering::Acquire);

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

    #[inline(always)]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        let counter = {
            // Safety: Shared owns a pointer to valid data leaked from the Box.
            let inner = unsafe { self.inner.as_ref() };

            inner.counter.load(AtomicOrdering::Acquire)
        };

        if counter == 1 {
            // Safety:
            //   1. Shared owns a pointer to valid data leaked from the Box.
            //   2. This ownership is unique because of the counter value.
            //   3. No new Shared clones may be created in between,
            //      because this Shared instance is borrowed mutably.
            let inner = unsafe { self.inner.as_mut() };

            return Some(&mut inner.data);
        }

        None
    }

    // Safety: There are no active borrows into the inner data.
    #[inline(always)]
    pub unsafe fn get_mut_unchecked(&mut self) -> &mut T {
        #[cfg(debug_assertions)]
        {
            let counter = {
                // Safety: Shared owns a pointer to valid data leaked from the Box.
                let inner = unsafe { self.inner.as_ref() };

                inner.counter.load(AtomicOrdering::Acquire)
            };

            if counter == 1 {
                panic!("Shared::get_mut_unchecked safety requirements violation.");
            }
        }

        // Safety:
        //   1. Shared owns a pointer to valid data leaked from the Box.
        //   2. Since there are no active borrows into the inner data (upheld by the caller).
        //   3. No new Shared clones may be created in between,
        //      because this Shared instance is borrowed mutably.
        let inner = unsafe { self.inner.as_mut() };

        &mut inner.data
    }

    pub fn make_mut(&mut self) -> &mut T
    where
        T: Clone,
    {
        let unique = {
            // Safety: Shared owns a pointer to valid data leaked from the Box.
            let inner = unsafe { self.inner.as_ref() };

            inner
                .counter
                .compare_exchange(1, 0, AtomicOrdering::Acquire, AtomicOrdering::Relaxed)
                .is_ok()
        };

        match unique {
            true => {
                // Safety: Shared owns a pointer to valid data leaked from the Box.
                let inner = unsafe { self.inner.as_ref() };

                inner.counter.store(1, AtomicOrdering::Release);
            }

            false => {
                *self = Self::new(self.as_ref().clone());
            }
        }

        // Safety:
        //   1. Shared owns a pointer to valid data leaked from the Box.
        //   2. Owner uniqueness ensured above.
        //   3. No new Shared clones may be created in between,
        //      because this Shared instance is borrowed mutably.
        let inner = unsafe { self.inner.as_mut() };

        &mut inner.data
    }

    #[inline(always)]
    pub fn into_inner(self) -> Option<T> {
        let this = ManuallyDrop::new(self);

        let counter = {
            // Safety: Shared owns a pointer to valid data leaked from the Box.
            let inner = unsafe { this.inner.as_ref() };

            inner.counter.fetch_sub(1, AtomicOrdering::Release)
        };

        if counter == 1 {
            fence(AtomicOrdering::Acquire);

            // Safety:
            //   1. Shared owns a pointer to valid data leaked from the Box.
            //   2. The ownership transfer operation is ordered by the Acquire fence.
            //   3. Shared will not be dropped twice because of the `ManuallyDrop` wrapper.
            let inner = unsafe { Box::from_raw(this.inner.as_ptr()) };

            return Some(inner.data);
        }

        None
    }

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

    use crate::{std::*, sync::Shared};

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
