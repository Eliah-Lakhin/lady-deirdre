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
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

use crate::report::system_panic;

const CHECK_MASK: usize = 1usize;
const REF_MASK: usize = usize::MAX ^ CHECK_MASK;
const REF_MAX: usize = REF_MASK / 2;
const REF_STEP: usize = 1 << 1;

/// A shared boolean flag.
///
/// This object semantically equals `Arc<AtomicBool>`, but is specifically
/// optimized by packing the reference counter and the boolean flag into
/// a single atomic machine word.
///
/// All clones of Trigger refer to the same allocation, which is freed once the
/// last instance of a Trigger is dropped.
///
/// The [constructor](Trigger::new) function creates a new Trigger with
/// the boolean flag set to false ("inactive").
///
/// The [activate](Trigger::activate) function sets the flag to true. This flag
/// cannot be unset later on.
///
/// The [is_active](Trigger::is_active) function checks if the flag set to true
/// by this instance or any of its clones.
///
/// The Trigger interface is particularly interesting for multi-threaded task
/// job graceful shutdown mechanisms, where one instance of Trigger would be
/// a job handle used outside of the job thread, and the job thread would
/// periodically examine another clone of this Trigger for activation, which
/// would be a signal for the worker to interrupt its job.
#[repr(transparent)]
pub struct Trigger {
    data: NonNull<AtomicUsize>,
}

// Safety: Trigger's data access is guarded by the atomic operations.
unsafe impl Send for Trigger {}

// Safety: Trigger's data access is guarded by the atomic operations.
unsafe impl Sync for Trigger {}

impl Default for Trigger {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Trigger {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self.is_active() {
            true => formatter.write_str("Trigger(active)"),
            false => formatter.write_str("Trigger(inactive)"),
        }
    }
}

impl PartialEq for Trigger {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.addr().eq(&other.addr())
    }
}

impl Eq for Trigger {}

impl Hash for Trigger {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.addr().hash(state)
    }
}

impl Clone for Trigger {
    fn clone(&self) -> Self {
        let value = {
            // Safety: Trigger owns a pointer to valid data leaked from the Box.
            let state = unsafe { self.data.as_ref() };

            state.fetch_add(REF_STEP, Ordering::Relaxed)
        };

        if value & REF_MASK > REF_MAX {
            system_panic!("Too many Trigger references.");
        }

        Self { data: self.data }
    }
}

impl Drop for Trigger {
    fn drop(&mut self) {
        let value = {
            // Safety: Trigger owns a pointer to valid data leaked from the Box.
            let state = unsafe { self.data.as_ref() };

            state.fetch_sub(REF_STEP, Ordering::Release)
        };

        if value & REF_MASK == REF_STEP {
            fence(Ordering::Acquire);

            // Safety:
            //   1. Trigger owns a pointer to valid data leaked from the Box.
            //   2. The drop operation is ordered by the Acquire fence.
            let _ = unsafe { Box::from_raw(self.data.as_ptr()) };
        }
    }
}

impl Trigger {
    /// Creates a new inactive Trigger.
    pub fn new() -> Self {
        let data = Box::into_raw(Box::new(AtomicUsize::new(REF_STEP)));

        // Safety: Box leaked pointer is never null.
        let data = unsafe { NonNull::new_unchecked(data) };

        Self { data }
    }

    /// Returns true if this Trigger was activated with the
    /// [activate](Self::activate) function.
    ///
    /// It is not guaranteed that an activation event that occurred within
    /// another clone of this Trigger in another thread concurrently would
    /// always be observed by this instance immediately.
    ///
    /// However, if the Trigger was activated, this function will eventually
    /// observe activation.
    pub fn is_active(&self) -> bool {
        // Safety: Trigger owns a pointer to valid data leaked from the Box.
        let state = unsafe { self.data.as_ref() };

        let value = state.load(Ordering::Relaxed);

        value & CHECK_MASK == CHECK_MASK
    }

    /// Activates this Trigger.
    ///
    /// Already activated triggers cannot be deactivated.
    pub fn activate(&self) {
        // Safety: Trigger owns a pointer to valid data leaked from the Box.
        let state = unsafe { self.data.as_ref() };

        state.fetch_or(CHECK_MASK, Ordering::Release);
    }

    /// Returns the address of the Trigger's allocation.
    #[inline(always)]
    pub fn addr(&self) -> usize {
        self.data.as_ptr() as usize
    }
}

#[cfg(test)]
mod tests {
    use crate::sync::Trigger;

    #[test]
    fn test_trigger() {
        let trigger = Trigger::new();

        assert!(!trigger.is_active());

        let trigger2 = trigger.clone();

        trigger.activate();

        drop(trigger);

        assert!(trigger2.is_active());
    }
}
