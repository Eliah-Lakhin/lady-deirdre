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

use std::mem::take;

use crate::{
    lexis::ByteIndex,
    mem::{array_shift, slice_copy_to, slice_shift},
    report::{ld_assert, ld_unreachable},
    units::storage::{
        child::{ChildCount, ChildIndex},
        PAGE_CAP,
        STRING_INLINE,
    },
};

pub(super) struct PageString {
    indices: [ByteIndex; PAGE_CAP],
    bytes: Bytes,
}

impl Default for PageString {
    #[inline(always)]
    fn default() -> Self {
        Self {
            indices: [0; PAGE_CAP],
            bytes: Bytes::default(),
        }
    }
}

impl PageString {
    // Safety:
    //  1. `index < occupied`.
    //  2. `occupied <= PAGE_CAP`
    //  3. `PageString` indices are well-formed.
    #[inline(always)]
    pub(super) unsafe fn byte_slice(&self, occupied: ChildCount, index: ChildIndex) -> &[u8] {
        ld_assert!(index < occupied, "Incorrect index.");
        ld_assert!(occupied <= PAGE_CAP, "Incorrect occupied value.");

        let next = index + 1;

        let start_byte = *unsafe { self.indices.get_unchecked(index) };

        let slice = match (&self.bytes, next < occupied) {
            (Bytes::Inline(inline), true) => {
                let end_byte = *unsafe { self.indices.get_unchecked(next) };
                unsafe { inline.vec.get_unchecked(start_byte..end_byte) }
            }

            (Bytes::Inline(inline), false) => unsafe {
                inline.vec.get_unchecked(start_byte..inline.len)
            },

            (Bytes::Heap(vec), true) => {
                let end_byte = *unsafe { self.indices.get_unchecked(next) };
                unsafe { vec.get_unchecked(start_byte..end_byte) }
            }

            (Bytes::Heap(vec), false) => unsafe { vec.get_unchecked(start_byte..) },
        };

        slice
    }

    // Safety:
    //  1. `index < occupied`.
    //  2. `occupied <= PAGE_CAP`
    //  3. `PageString` indices are well-formed.
    #[inline(always)]
    pub(super) unsafe fn byte_slice_from(&self, occupied: ChildCount, index: ChildIndex) -> &[u8] {
        ld_assert!(index < occupied, "Incorrect index.");
        ld_assert!(occupied <= PAGE_CAP, "Incorrect occupied value.");

        let start_byte = *unsafe { self.indices.get_unchecked(index) };

        let slice = match &self.bytes {
            Bytes::Inline(inline) => unsafe { inline.vec.get_unchecked(start_byte..inline.len) },
            Bytes::Heap(vec) => unsafe { vec.get_unchecked(start_byte..) },
        };

        slice
    }

    #[inline(always)]
    pub(super) unsafe fn bytes(&self) -> &[u8] {
        match &self.bytes {
            Bytes::Inline(inline) => unsafe { inline.vec.get_unchecked(0..inline.len) },
            Bytes::Heap(vec) => vec.as_slice(),
        }
    }

    #[inline(always)]
    // Safety:
    //  1. `source + count <= from_occupied`
    //  2. `from_occupied <= PAGE_CAP`
    //  3. `destination + count <= to_occupied`
    //  4. `to_occupied <= PAGE_CAP`.
    //  5. `count > 0`.
    //  6. All `to` indices except maybe `(destination+1)..(destination+count)`
    //     are well-formed.
    //  7. All `self indices are well-formed.
    //  8. At least first `source + count` in `self` are well-formed.
    pub(super) unsafe fn copy_to(
        &self,
        from_occupied: ChildCount,
        to: &mut Self,
        to_occupied: ChildCount,
        source: ChildIndex,
        destination: ChildIndex,
        count: ChildCount,
    ) {
        ld_assert!(source + count <= from_occupied, "Source range overflow.",);

        ld_assert!(from_occupied <= PAGE_CAP, "Source occupied value overflow.",);

        ld_assert!(
            destination + count <= to_occupied,
            "Destination range overflow.",
        );

        ld_assert!(
            to_occupied <= PAGE_CAP,
            "Destination occupied value overflow.",
        );

        ld_assert!(count > 0, "Empty copy range.");

        let text_indices = match source + count < from_occupied {
            true => self.indices.get_unchecked(source..=(source + count)),
            false => self.indices.get_unchecked(source..(source + count)),
        };

        let text = match &self.bytes {
            Bytes::Inline(inline) => unsafe { inline.vec.get_unchecked(0..inline.len) },
            Bytes::Heap(vec) => vec.as_slice(),
        };

        unsafe { to.rewrite(to_occupied, destination, text, text_indices, count) };
    }

    // Safety:
    //  1. `from + count <= occupied`.
    //  2. `occupied <= PAGE_CAP`.
    //  3. `count > 0`.
    //  4. All `PageString` indices except maybe `(from+1)..(from+count)`
    //     are well-formed.
    //  5. `text` non-empty.
    //  6. `text_indices` are well-formed byte indices into `text`.
    //  7. `text_indices` have at least `count` items.
    #[inline]
    pub(super) unsafe fn rewrite(
        &mut self,
        occupied: ChildCount,
        from: ChildIndex,
        text: &[u8],
        text_indices: &[ByteIndex],
        count: ChildCount,
    ) {
        ld_assert!(from + count <= occupied, "Count overflow.");
        ld_assert!(occupied <= PAGE_CAP, "Occupied value overflow.");
        ld_assert!(count > 0, "Empty count.");
        ld_assert!(!text.is_empty(), "Empty text.");
        ld_assert!(text_indices.len() >= count, "Underflow text_indices.");

        let text_start_byte = *unsafe { text_indices.get_unchecked(0) };

        let text_end_byte = match count < text_indices.len() {
            true => *unsafe { text_indices.get_unchecked(count) },
            false => text.len(),
        };

        let text = unsafe { text.get_unchecked(text_start_byte..text_end_byte) };

        let to = from + count;

        let string_start_byte = *unsafe { self.indices.get_unchecked(from) };

        let string_end_byte = match to < occupied {
            true => *unsafe { self.indices.get_unchecked(to) },
            false => self.bytes.len(),
        };

        let diff = unsafe { self.bytes.write(string_start_byte, string_end_byte, text) };

        let mut index = from;

        loop {
            let text_byte = *unsafe { text_indices.get_unchecked(index - from) };
            let string_byte = unsafe { self.indices.get_unchecked_mut(index) };

            *string_byte = text_byte + string_start_byte - text_start_byte;

            index += 1;

            if index >= from + count {
                break;
            }
        }

        if diff > 0 {
            let diff = diff as usize;

            while index < occupied {
                let string_byte = unsafe { self.indices.get_unchecked_mut(index) };

                *string_byte += diff;

                index += 1;
            }
        } else if diff < 0 {
            let diff = (-diff) as usize;

            while index < occupied {
                let string_byte = unsafe { self.indices.get_unchecked_mut(index) };

                *string_byte -= diff;

                index += 1;
            }
        }
    }

    // Safety:
    //  1. `from <= occupied`.
    //  2. `occupied + count <= PAGE_CAP`.
    //  3. `count > 0`.
    //
    // This operation does not change underlying string bytes.
    // This operation preserves `0..=from` and `(from+count)..(occupied+count)`
    // byte indices well-formed-ness, but does not preserve other indices
    // in the inflated gap well-formed-ness.
    #[inline(always)]
    pub(super) unsafe fn inflate(
        &mut self,
        occupied: ChildCount,
        from: ChildIndex,
        count: ChildCount,
    ) {
        ld_assert!(from <= occupied, "String inflation failure.");
        ld_assert!(occupied + count <= PAGE_CAP, "String inflation failure.",);
        ld_assert!(count > 0, "Empty inflation.");

        if from == occupied {
            // For optimization purposes only the first index in the
            // inflated gap will be well formed.
            *unsafe { self.indices.get_unchecked_mut(from) } = self.bytes.len();
            return;
        };

        unsafe { array_shift(&mut self.indices, from, from + count, occupied - from) }
    }

    // Safety:
    //  1. `occupied <= PAGE_CAP`
    //  2. `from + count <= occupied`.
    //  3. `count > 0`.
    //  4. `PageString` indices are well-formed.
    #[inline]
    pub(super) unsafe fn deflate(
        &mut self,
        mut occupied: ChildCount,
        mut from: ChildIndex,
        count: ChildCount,
    ) {
        ld_assert!(occupied <= PAGE_CAP, "Incorrect occupied value.");
        ld_assert!(from + count <= occupied, "String deflation failure");
        ld_assert!(count > 0, "Empty string deflation.");

        let to = from + count;
        let start_byte = *unsafe { self.indices.get_unchecked(from) };
        let diff;

        match &mut self.bytes {
            Bytes::Inline(inline) => {
                match to < occupied {
                    true => {
                        let end_byte = *unsafe { self.indices.get_unchecked(to) };

                        diff = end_byte - start_byte;

                        unsafe {
                            slice_shift(
                                &mut inline.vec,
                                end_byte,
                                start_byte,
                                inline.len - end_byte,
                            )
                        };

                        inline.len -= diff;
                    }

                    false => {
                        diff = inline.len - start_byte;

                        inline.len = start_byte;
                    }
                };
            }

            Bytes::Heap(vec) => {
                match to < occupied {
                    true => {
                        let len = vec.len();
                        let end_byte = *unsafe { self.indices.get_unchecked(to) };

                        diff = end_byte - start_byte;

                        unsafe {
                            slice_shift(vec.as_mut_slice(), end_byte, start_byte, len - end_byte)
                        };

                        unsafe { vec.set_len(len - diff) };
                    }

                    false => {
                        diff = vec.len() - start_byte;

                        unsafe { vec.set_len(start_byte) };
                    }
                };

                unsafe { self.bytes.try_shrink() };
            }
        }

        if to < occupied {
            unsafe { array_shift(&mut self.indices, to, from, occupied - to) };

            occupied -= count;

            loop {
                *unsafe { self.indices.get_unchecked_mut(from) } -= diff;

                from += 1;

                if from >= occupied {
                    break;
                }
            }
        }
    }

    #[inline]
    pub(super) fn append(&mut self, string: &str) {
        let destination;
        let vec = match &mut self.bytes {
            Bytes::Inline(inline) => {
                if inline.len + string.len() <= STRING_INLINE {
                    unsafe {
                        slice_copy_to(
                            string.as_bytes(),
                            &mut inline.vec,
                            0,
                            inline.len,
                            string.len(),
                        )
                    };

                    inline.len += string.len();

                    return;
                }

                destination = inline.len;
                unsafe { self.bytes.as_heap(string.len()) }
            }

            Bytes::Heap(vec) => {
                destination = vec.len();

                vec.reserve(string.len());

                unsafe { vec.set_len(destination + string.len()) };

                vec
            }
        };

        unsafe {
            slice_copy_to(
                string.as_bytes(),
                vec.as_mut_slice(),
                0,
                destination,
                string.len(),
            )
        }
    }

    // Safety:
    //   1. `index < PAGE_CAP`.
    #[inline(always)]
    pub(super) unsafe fn get_byte_index(&self, index: ChildIndex) -> ByteIndex {
        ld_assert!(index < PAGE_CAP, "Index overflow.");

        *unsafe { self.indices.get_unchecked(index) }
    }

    // Safety:
    //   1. `index < PAGE_CAP`.
    #[inline(always)]
    pub(super) unsafe fn set_byte_index(&mut self, index: ChildIndex, byte_index: ByteIndex) {
        ld_assert!(index < PAGE_CAP, "Index overflow.");

        *unsafe { self.indices.get_unchecked_mut(index) } = byte_index;
    }
}

enum Bytes {
    Inline(Inline),
    Heap(Vec<u8>),
}

impl Drop for Bytes {
    fn drop(&mut self) {
        match self {
            Self::Heap(vec) => unsafe { vec.set_len(0) },
            _ => (),
        }
    }
}

impl Default for Bytes {
    #[inline(always)]
    fn default() -> Self {
        Self::Inline(Inline::default())
    }
}

impl Bytes {
    // Safety:
    //  1. `from` and `to` are valid byte indices into underlying string.
    //  2. `from <= to`.
    //  3. `text` is is valid Utd8 sequence.
    #[inline(always)]
    unsafe fn write(&mut self, from: ByteIndex, to: ByteIndex, text: &[u8]) -> isize {
        ld_assert!(from <= to, "Invalid byte range.");

        let text_diff = text.len();
        let string_diff = to - from;

        match self {
            Self::Inline(inline) => {
                if text_diff < string_diff {
                    let shrink = string_diff - text_diff;

                    if to < inline.len {
                        unsafe { slice_shift(&mut inline.vec, to, to - shrink, inline.len - to) };
                    }

                    inline.len -= shrink;

                    unsafe { slice_copy_to(text, &mut inline.vec, 0, from, text_diff) };

                    -(shrink as isize)
                } else if text_diff > string_diff {
                    let grow = text_diff - string_diff;

                    if inline.len + grow <= STRING_INLINE {
                        if to < inline.len {
                            unsafe { slice_shift(&mut inline.vec, to, to + grow, inline.len - to) };
                        }

                        inline.len += grow;

                        unsafe { slice_copy_to(text, &mut inline.vec, 0, from, text_diff) };
                    } else {
                        let len = inline.len;
                        let vec = unsafe { self.as_heap(grow) };

                        if to < len {
                            unsafe { slice_shift(vec, to, to + grow, len - to) };
                        }

                        unsafe { slice_copy_to(text, vec, 0, from, text_diff) };
                    }

                    grow as isize
                } else {
                    unsafe { slice_copy_to(text, &mut inline.vec, 0, from, text_diff) };

                    0
                }
            }

            Self::Heap(vec) => {
                if text_diff < string_diff {
                    let len = vec.len();
                    let shrink = string_diff - text_diff;

                    if to < len {
                        unsafe { slice_shift(vec, to, to - shrink, len - to) };
                    }

                    unsafe { vec.set_len(len - shrink) };

                    unsafe { slice_copy_to(text, vec, 0, from, text_diff) };

                    unsafe { self.try_shrink() };

                    -(shrink as isize)
                } else if text_diff > string_diff {
                    let len = vec.len();
                    let grow = text_diff - string_diff;

                    vec.reserve(grow);

                    unsafe { vec.set_len(len + grow) };

                    if to < len {
                        unsafe { slice_shift(vec, to, to + grow, len - to) };
                    }

                    unsafe { slice_copy_to(text, vec, 0, from, text_diff) };

                    grow as isize
                } else {
                    unsafe { slice_copy_to(text, vec, 0, from, text_diff) };

                    0
                }
            }
        }
    }

    // Safety: Bytes are currently inlined.
    #[inline(always)]
    unsafe fn as_heap(&mut self, grow: ByteIndex) -> &mut Vec<u8> {
        match self {
            Self::Inline(inline) => {
                let mut vec =
                    Vec::with_capacity(usize::max(STRING_INLINE * 3 / 2, inline.len + grow));

                {
                    unsafe { vec.set_len(inline.len + grow) };

                    unsafe { slice_copy_to(&inline.vec, vec.as_mut_slice(), 0, 0, inline.len) };
                }

                *self = Self::Heap(vec);

                match self {
                    Bytes::Heap(vec) => vec,

                    Bytes::Inline(_) => unsafe {
                        ld_unreachable!("Bytes transfer to heap failure.")
                    },
                }
            }

            Self::Heap(_) => unsafe { ld_unreachable!("Bytes already on heap.") },
        }
    }

    // Safety: Bytes are currently on heap.
    #[inline(always)]
    unsafe fn try_shrink(&mut self) {
        match self {
            Self::Heap(vec) => {
                let len = vec.len();

                if len > STRING_INLINE / 2 {
                    return;
                }

                let mut vec = take(vec);

                *self = Self::Inline(Inline {
                    vec: [0; STRING_INLINE],
                    len,
                });

                match self {
                    Bytes::Inline(inline) => {
                        unsafe { slice_copy_to(vec.as_slice(), &mut inline.vec, 0, 0, len) };

                        unsafe { vec.set_len(0) };
                    }

                    Bytes::Heap(_) => unsafe { ld_unreachable!("Bytes inlining failure.") },
                }
            }

            Self::Inline(..) => unsafe { ld_unreachable!("Bytes already inlined.") },
        }
    }

    #[inline(always)]
    fn len(&self) -> ByteIndex {
        match self {
            Bytes::Inline(inline) => inline.len,
            Bytes::Heap(vec) => vec.len(),
        }
    }
}

struct Inline {
    vec: [u8; STRING_INLINE],
    len: ByteIndex,
}

impl Default for Inline {
    #[inline(always)]
    fn default() -> Self {
        Self {
            vec: [0; STRING_INLINE],
            len: 0,
        }
    }
}
