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

use std::{ops::Deref, sync::OnceLock};

/// A value which is initialized on the first access.
///
/// Lazy is thread-safe and can be used in statics.
///
/// Any dereferencing access will block the thread if another thread is currently
/// initializes this Lazy.
///
/// The first generic parameter `T` is required and specifies the underlying
/// data type.
///
/// The second generic parameter is inferred by the compiler based on the
/// constructor's callback:
///
/// ```
/// use std::ops::Deref;
/// use lady_deirdre::sync::Lazy;
///
/// static FOO: Lazy<usize> = Lazy::new(|| 10 + 20);
///
/// let a: &'static usize = FOO.deref(); // first access implies initialization
///
/// assert_eq!(*a, 30);
/// ```
///
/// If you are familiar with
/// the [once_cell](https://github.com/matklad/once_cell/tree/c48d3c2c01de926228aea2ac1d03672b4ce160c1)
/// crate, Lady Deirdre's Lazy implements a similar object as
/// `once_cell::sync::Lazy`, but it is fully built on the standard library
/// features without any third-party dependencies.
pub struct Lazy<T: Send + Sync + 'static, F = fn() -> T> {
    cell: OnceLock<T>,
    init: F,
}

impl<T: Send + Sync + 'static> Deref for Lazy<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.cell.get_or_init(self.init)
    }
}

impl<T: Send + Sync + 'static> Lazy<T> {
    /// A constructor of the object.
    ///
    /// The constructor is a const function, but the `init` function, which
    /// initializes the Lazy instance on the first dereferencing, is not
    /// required to be const function.
    ///
    /// The `init` constructor should not dereference not-yet-initialized self
    /// Lazy directly or indirectly. The exact behavior of recurrent
    /// referencing is not specified, but usually leads to runtime deadlocks and
    /// may panic on some platforms.
    #[inline(always)]
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            cell: OnceLock::new(),
            init,
        }
    }
}
