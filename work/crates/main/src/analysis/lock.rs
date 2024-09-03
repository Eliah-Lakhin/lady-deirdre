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

use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::{Condvar, Mutex},
    time::{Duration, Instant},
};

use crate::{
    analysis::{AnalysisError, AnalysisResult},
    report::ld_assert,
};

const UNLOCK_MASK: usize = 0;
const READ_MASK: usize = !0 ^ 1;
const READ_BIT: usize = 1 << 1;
const WRITE_MASK: usize = 1;

pub(super) struct TimeoutRwLock<T: 'static> {
    state: Mutex<usize>,
    state_changed: Condvar,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send + Sync + 'static> Send for TimeoutRwLock<T> {}

unsafe impl<T: Send + Sync + 'static> Sync for TimeoutRwLock<T> {}

impl<T: 'static> TimeoutRwLock<T> {
    #[inline(always)]
    pub(super) fn new(data: T) -> Self {
        Self {
            state: Mutex::new(UNLOCK_MASK),
            state_changed: Condvar::new(),
            data: UnsafeCell::new(data),
        }
    }

    #[cfg(not(target_family = "wasm"))]
    pub(super) fn read(&self, timeout: &Duration) -> AnalysisResult<TimeoutRwLockReadGuard<T>> {
        let mut state_guard = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        let mut cooldown = 1;
        let time = Instant::now();

        loop {
            if *state_guard & WRITE_MASK == 0 {
                *state_guard += READ_BIT;
                return Ok(TimeoutRwLockReadGuard { record: self });
            }

            (state_guard, _) = self
                .state_changed
                .wait_timeout(state_guard, Duration::from_millis(cooldown))
                .unwrap_or_else(|poison| poison.into_inner());

            if &time.elapsed() > timeout {
                return Err(AnalysisError::Timeout);
            }

            cooldown <<= 1;
        }
    }

    #[cfg(target_family = "wasm")]
    pub(super) fn read(&self, timeout: &Duration) -> AnalysisResult<TimeoutRwLockReadGuard<T>> {
        let mut state_guard = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        if *state_guard & WRITE_MASK == 0 {
            *state_guard += READ_BIT;
            return Ok(TimeoutRwLockReadGuard { record: self });
        }

        Err(AnalysisError::Timeout)
    }

    #[cfg(not(target_family = "wasm"))]
    pub(super) fn write(&self, timeout: &Duration) -> AnalysisResult<TimeoutRwLockWriteGuard<T>> {
        let mut state_guard = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        let mut culldown = 1;
        let time = Instant::now();

        loop {
            if *state_guard == UNLOCK_MASK {
                *state_guard = WRITE_MASK;
                return Ok(TimeoutRwLockWriteGuard { record: self });
            }

            (state_guard, _) = self
                .state_changed
                .wait_timeout(state_guard, Duration::from_millis(culldown))
                .unwrap_or_else(|poison| poison.into_inner());

            if &time.elapsed() > timeout {
                return Err(AnalysisError::Timeout);
            }

            culldown <<= 1;
        }
    }

    #[cfg(target_family = "wasm")]
    pub(super) fn write(&self, timeout: &Duration) -> AnalysisResult<TimeoutRwLockWriteGuard<T>> {
        let mut state_guard = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        if *state_guard == UNLOCK_MASK {
            *state_guard = WRITE_MASK;
            return Ok(TimeoutRwLockWriteGuard { record: self });
        }

        Err(AnalysisError::Timeout)
    }
}

pub(super) struct TimeoutRwLockReadGuard<'a, T: 'static> {
    record: &'a TimeoutRwLock<T>,
}

impl<'a, T: 'static> Drop for TimeoutRwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        let mut state_guard = self
            .record
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        ld_assert!(*state_guard & WRITE_MASK == 0, "Invalid lock state.");
        ld_assert!(*state_guard & READ_MASK > 0, "Invalid lock state.");

        *state_guard -= READ_BIT;

        if *state_guard == UNLOCK_MASK {
            drop(state_guard);
            self.record.state_changed.notify_one();
        }
    }
}

impl<'a, T: 'static> Deref for TimeoutRwLockReadGuard<'a, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.record.data.get() }
    }
}

pub(super) struct TimeoutRwLockWriteGuard<'a, T: 'static> {
    record: &'a TimeoutRwLock<T>,
}

impl<'a, T: 'static> Drop for TimeoutRwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        let mut state_guard = self
            .record
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        ld_assert!(*state_guard & WRITE_MASK > 0, "Invalid lock state.");
        ld_assert!(*state_guard & READ_MASK == 0, "Invalid lock state.");

        *state_guard = UNLOCK_MASK;

        drop(state_guard);

        self.record.state_changed.notify_all();
    }
}

impl<'a, T: 'static> Deref for TimeoutRwLockWriteGuard<'a, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.record.data.get() }
    }
}

impl<'a, T: 'static> DerefMut for TimeoutRwLockWriteGuard<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.record.data.get() }
    }
}
