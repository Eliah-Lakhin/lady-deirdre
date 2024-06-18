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

use std::ptr::{copy, copy_nonoverlapping};

use crate::report::{ld_assert, ld_assert_ne};

//Safety:
// 1. `from` and `to` are two distinct memory allocations.
// 2. `source..(source + count)` is within `from` bounds.
// 3. `destination..(destination + count)` is within `to` bounds.
#[inline(always)]
pub(crate) unsafe fn array_copy_to<const N: usize, T: Sized>(
    from: &[T; N],
    to: &mut [T; N],
    source: usize,
    destination: usize,
    count: usize,
) {
    ld_assert_ne!(from.as_ptr(), to.as_mut_ptr(), "Array copy overlapping.");
    ld_assert!(source + count <= N, "Source range exceeds capacity.");
    ld_assert!(
        destination + count <= N,
        "Destination range exceeds capacity.",
    );

    let from = unsafe { from.as_ptr().offset(source as isize) };
    let to = unsafe { to.as_mut_ptr().offset(destination as isize) };

    unsafe { copy_nonoverlapping(from, to, count) };
}

//Safety:
// 1. `from` and `to` are two distinct memory allocations.
// 2. `source..(source + count)` is within `from` bounds.
// 3. `destination..(destination + count)` is within `to` bounds.
#[inline(always)]
pub(crate) unsafe fn slice_copy_to<T: Sized>(
    from: &[T],
    to: &mut [T],
    source: usize,
    destination: usize,
    count: usize,
) {
    ld_assert_ne!(from.as_ptr(), to.as_mut_ptr(), "Slice copy overlapping.");
    ld_assert!(
        source + count <= from.len(),
        "Source range exceeds capacity."
    );
    ld_assert!(
        destination + count <= to.len(),
        "Destination range exceeds capacity.",
    );

    let from = unsafe { from.as_ptr().offset(source as isize) };
    let to = unsafe { to.as_mut_ptr().offset(destination as isize) };

    unsafe { copy_nonoverlapping(from, to, count) };
}

//Safety:
// 1. `from + count <= N`.
// 1. `to + count <= N`.
// 2. `count > 0`.
#[inline(always)]
pub(crate) unsafe fn array_shift<const N: usize, T: Sized>(
    array: &mut [T; N],
    from: usize,
    to: usize,
    count: usize,
) {
    ld_assert!(from + count <= N, "Shift with overflow.");
    ld_assert!(to + count <= N, "Shift with overflow.");
    ld_assert!(count > 0, "Empty shift range.");

    let array_ptr = array.as_mut_ptr();
    let source = unsafe { array_ptr.offset(from as isize) };
    let destination = unsafe { array_ptr.offset(to as isize) };

    match from + count <= to || to + count <= from {
        false => unsafe { copy(source, destination, count) },
        true => unsafe { copy_nonoverlapping(source, destination, count) },
    }
}

//Safety:
// 1. `from + count <= slice.len()`.
// 1. `to + count <= slice.len()`.
// 2. `count > 0`.
#[inline(always)]
pub(crate) unsafe fn slice_shift<T: Sized>(slice: &mut [T], from: usize, to: usize, count: usize) {
    ld_assert!(from + count <= slice.len(), "Shift with overflow.");
    ld_assert!(to + count <= slice.len(), "Shift with overflow.");
    ld_assert!(count > 0, "Empty shift range.");

    let array_ptr = slice.as_mut_ptr();
    let source = unsafe { array_ptr.offset(from as isize) };
    let destination = unsafe { array_ptr.offset(to as isize) };

    match from + count <= to || to + count <= from {
        false => unsafe { copy(source, destination, count) },
        true => unsafe { copy_nonoverlapping(source, destination, count) },
    }
}

#[cfg(test)]
mod tests {
    use crate::mem::{array_copy_to, array_shift, slice_copy_to, slice_shift};

    #[test]
    fn test_array_copy_to() {
        let from = [1, 2, 3, 4, 5, 6, 7];
        let mut to = [-1, -2, -3, -4, -5, -6, -7];

        unsafe { array_copy_to(&from, &mut to, 3, 1, 3) };

        assert_eq!(to, [-1, 4, 5, 6, -5, -6, -7]);
    }

    #[test]
    fn test_slice_copy_to() {
        let from = [1, 2, 3, 4, 5, 6, 7];
        let mut to = [-1, -2, -3, -4, -5, -6, -7];

        unsafe { slice_copy_to(&from, &mut to, 3, 1, 3) };

        assert_eq!(to, [-1, 4, 5, 6, -5, -6, -7]);
    }

    #[test]
    fn test_array_shift_no_overlap() {
        let mut array = [1, 2, 3, 4, 5, 6, 7];

        unsafe { array_shift(&mut array, 1, 4, 2) };

        assert_eq!(array, [1, 2, 3, 4, 2, 3, 7]);
    }

    #[test]
    fn test_array_shift_overlap() {
        let mut array = [1, 2, 3, 4, 5, 6, 7];

        unsafe { array_shift(&mut array, 4, 3, 2) };

        assert_eq!(array, [1, 2, 3, 5, 6, 6, 7]);
    }

    #[test]
    fn test_slice_shift_no_overlap() {
        let mut slice = [1, 2, 3, 4, 5, 6, 7];

        unsafe { slice_shift(&mut slice, 1, 4, 2) };

        assert_eq!(slice, [1, 2, 3, 4, 2, 3, 7]);
    }

    #[test]
    fn test_slice_shift_overlap() {
        let mut slice = [1, 2, 3, 4, 5, 6, 7];

        unsafe { slice_shift(&mut slice, 4, 3, 2) };

        assert_eq!(slice, [1, 2, 3, 5, 6, 6, 7]);
    }
}
