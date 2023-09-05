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

//TODO cleanup unused reexports.

#[cfg(not(feature = "std"))]
pub use ::alloc::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    collections::{
        btree_set::Iter as StdSetIter,
        BTreeMap as StdMap,
        BTreeSet as StdSet,
        LinkedList,
        VecDeque,
    },
    format,
    rc::{Rc, Weak as SyncWeak},
    string::{String, ToString},
    sync::Arc,
    vec::{IntoIter, Vec},
};
#[cfg(not(feature = "std"))]
pub use ::core::{
    any::{Any, TypeId},
    assert_eq,
    assert_ne,
    borrow::{Borrow, BorrowMut},
    cell::UnsafeCell,
    clone::Clone,
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    column,
    concat,
    convert::{AsRef, From, Into},
    default::Default,
    file,
    fmt::{Arguments as FmtArguments, Debug, Display, Formatter, Result as FmtResult},
    format_args,
    hash::{Hash, Hasher},
    hint::unreachable_unchecked,
    iter::{
        repeat,
        Copied,
        DoubleEndedIterator,
        Enumerate,
        ExactSizeIterator,
        Extend,
        FilterMap,
        FromIterator,
        FusedIterator,
        IntoIterator,
        Iterator,
        Map,
        Peekable,
    },
    line,
    marker::{Copy, PhantomData, Send, Sized, Sync},
    matches,
    mem::{forget, replace, take, transmute, ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ops::{
        AddAssign,
        Deref,
        DerefMut,
        Drop,
        Fn,
        FnMut,
        Index,
        IndexMut,
        Range,
        RangeFrom,
        RangeFull,
        RangeInclusive,
        RangeTo,
        RangeToInclusive,
    },
    option::Option,
    option::Option::*,
    panic,
    ptr::{copy, copy_nonoverlapping, swap, NonNull},
    result::Result,
    result::Result::{Err, Ok},
    slice::Iter,
    str::{from_utf8, from_utf8_unchecked, Chars},
    sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering},
    todo,
    unimplemented,
    unreachable,
};

#[cfg(feature = "std")]
extern crate std;
#[cfg(feature = "std")]
pub use std::{
    any::{Any, TypeId},
    assert_eq,
    assert_ne,
    borrow::{Borrow, BorrowMut, Cow, ToOwned},
    boxed::Box,
    cell::UnsafeCell,
    clone::Clone,
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    collections::{
        hash_set::Iter as StdSetIter,
        HashMap as StdMap,
        HashSet as StdSet,
        LinkedList,
        VecDeque,
    },
    column,
    concat,
    convert::{AsRef, From, Into, TryFrom},
    default::Default,
    file,
    fmt::{
        Arguments as FmtArguments,
        Debug,
        Display,
        Error as FmtError,
        Formatter,
        Result as FmtResult,
    },
    format,
    format_args,
    hash::{Hash, Hasher},
    hint::unreachable_unchecked,
    iter::{
        repeat,
        Copied,
        DoubleEndedIterator,
        Enumerate,
        ExactSizeIterator,
        Extend,
        FilterMap,
        FromIterator,
        FusedIterator,
        IntoIterator,
        Iterator,
        Map,
        Peekable,
    },
    line,
    marker::{Copy, PhantomData, Send, Sized, Sync},
    matches,
    mem::{forget, replace, take, transmute, ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ops::{
        AddAssign,
        Deref,
        DerefMut,
        Drop,
        Fn,
        FnMut,
        Index,
        IndexMut,
        Range,
        RangeFrom,
        RangeFull,
        RangeInclusive,
        RangeTo,
        RangeToInclusive,
    },
    option::Option,
    option::Option::*,
    panic,
    println,
    ptr::{copy, copy_nonoverlapping, swap, NonNull},
    rc::{Rc, Weak as SyncWeak},
    result::Result,
    result::Result::{Err, Ok},
    slice::Iter,
    str::{from_utf8, from_utf8_unchecked, Chars},
    string::{String, ToString},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering},
        Arc,
        Weak as AsyncWeak,
    },
    thread::panicking,
    todo,
    unimplemented,
    unreachable,
    vec::IntoIter,
    vec::Vec,
};

pub(crate) trait StdMapEx<K, V> {
    fn new_std_map(capacity: usize) -> Self;
}

#[cfg(feature = "std")]
impl<K, V> StdMapEx<K, V> for StdMap<K, V> {
    #[inline(always)]
    fn new_std_map(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }
}

#[cfg(not(feature = "std"))]
impl<K, V> StdMapEx<K, V> for StdMap<K, V> {
    #[inline(always)]
    fn new_std_map(_capacity: usize) -> Self {
        Self::new()
    }
}

pub(crate) trait StdSetEx<K> {
    fn new_std_set(capacity: usize) -> Self;
}

#[cfg(feature = "std")]
impl<K> StdSetEx<K> for StdSet<K> {
    #[inline(always)]
    fn new_std_set(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }
}

#[cfg(not(feature = "std"))]
impl<K> StdSetEx<K> for StdMap<K> {
    #[inline(always)]
    fn new_std_set(_capacity: usize) -> Self {
        Self::new()
    }
}
