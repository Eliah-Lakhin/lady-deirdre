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
extern crate alloc;
#[cfg(not(feature = "std"))]
extern crate core;

#[cfg(not(feature = "std"))]
pub(crate) use alloc::{
    borrow::Cow,
    boxed::Box,
    collections::{btree_set::Iter as StdSetIter, BTreeMap, BTreeSet, VecDeque},
    format,
    string::{String, ToString},
    vec::{IntoIter, Vec},
};
#[cfg(not(feature = "std"))]
pub(crate) use core::{
    any::{Any, TypeId},
    assert_eq,
    assert_ne,
    borrow::Borrow,
    cell::UnsafeCell,
    clone::Clone,
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    column,
    concat,
    convert::{AsMut, AsRef, From, Into, TryFrom},
    default::Default,
    error::Error,
    file,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    format_args,
    hash::{BuildHasher, Hash, Hasher},
    hint::{spin_loop, unreachable_unchecked},
    iter::{
        repeat,
        DoubleEndedIterator,
        Enumerate,
        ExactSizeIterator,
        Extend,
        FilterMap,
        FromIterator,
        FusedIterator,
        IntoIterator,
        Iterator,
        Peekable,
        Take,
    },
    iter::{Flatten, Map},
    line,
    marker::{Copy, PhantomData, Send, Sized, Sync},
    matches,
    mem::{replace, take, transmute, MaybeUninit},
    ops::{
        AddAssign,
        Deref,
        DerefMut,
        Drop,
        Fn,
        FnMut,
        FnOnce,
        Index,
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
    ptr::{copy, copy_nonoverlapping, NonNull},
    result::Result::{Err, Ok},
    slice::{Iter, IterMut},
    str::{from_utf8, from_utf8_unchecked, Chars},
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering as AtomicOrdering},
    todo,
    unimplemented,
    unreachable,
};

#[cfg(feature = "std")]
extern crate std;
#[cfg(feature = "std")]
pub(crate) use std::{
    any::Any,
    any::TypeId,
    assert_eq,
    assert_ne,
    borrow::ToOwned,
    borrow::{Borrow, Cow},
    boxed::Box,
    cell::UnsafeCell,
    clone::Clone,
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    collections::{
        hash_map::{
            Drain,
            Entry as HashMapEntry,
            IntoIter as HashMapIntoIter,
            OccupiedEntry,
            RandomState,
            VacantEntry,
        },
        hash_set::Iter as StdSetIter,
        HashMap,
        HashSet,
        VecDeque,
    },
    column,
    concat,
    convert::{AsMut, AsRef, From, Into, TryFrom},
    default::Default,
    error::Error,
    file,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    format,
    format_args,
    hash::{BuildHasher, Hash, Hasher},
    hint::{spin_loop, unreachable_unchecked},
    iter::{
        repeat,
        DoubleEndedIterator,
        Enumerate,
        ExactSizeIterator,
        Extend,
        FilterMap,
        FromIterator,
        FusedIterator,
        IntoIterator,
        Iterator,
        Peekable,
    },
    iter::{Flatten, Map, Take},
    line,
    marker::{Copy, PhantomData, Send, Sized, Sync},
    matches,
    mem::{drop, replace, size_of, take, transmute, ManuallyDrop, MaybeUninit},
    ops::{
        AddAssign,
        Deref,
        DerefMut,
        Drop,
        Fn,
        FnMut,
        FnOnce,
        Index,
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
    ptr::{copy, copy_nonoverlapping, NonNull},
    result::Result,
    result::Result::{Err, Ok},
    slice::{Iter, IterMut},
    str::{from_utf8, from_utf8_unchecked, Chars},
    string::{String, ToString},
    sync::{
        atomic::{fence, AtomicBool, AtomicU64, AtomicUsize, Ordering as AtomicOrdering},
        Arc,
        Condvar,
        Mutex,
        MutexGuard,
        OnceLock,
        RwLock,
        RwLockReadGuard,
        RwLockWriteGuard,
        TryLockError,
        Weak,
    },
    thread::{available_parallelism, panicking, spawn},
    thread_local,
    todo,
    unimplemented,
    unreachable,
    vec::IntoIter,
    vec::Vec,
};

#[cfg(feature = "std")]
pub(crate) type StdMap<K, V> = HashMap<K, V>;

#[cfg(not(feature = "std"))]
pub(crate) type StdMap<K, V> = BTreeMap<K, V>;

pub(crate) trait StdMapEx<K, V> {
    fn new_std_map() -> Self;

    fn new_std_map_with_capacity(capacity: usize) -> Self;
}

#[cfg(feature = "std")]
impl<K, V> StdMapEx<K, V> for StdMap<K, V> {
    #[inline(always)]
    fn new_std_map() -> Self {
        Self::new()
    }

    #[inline(always)]
    fn new_std_map_with_capacity(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }
}

#[cfg(not(feature = "std"))]
impl<K, V> StdMapEx<K, V> for StdMap<K, V> {
    #[inline(always)]
    fn new_std_map() -> Self {
        Self::new()
    }

    #[inline(always)]
    fn new_std_map_with_capacity(_capacity: usize) -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
pub(crate) type StdSet<T> = std::collections::HashSet<T>;

#[cfg(not(feature = "std"))]
pub(crate) type StdSet<T> = BTreeSet<T>;

pub(crate) trait StdSetEx<K> {
    fn new_std_set() -> Self;

    fn new_std_set_with_capacity(capacity: usize) -> Self;

    fn std_set_drain(&mut self, capacity: usize) -> Vec<K>
    where
        K: Eq + Hash;
}

#[cfg(feature = "std")]
impl<K> StdSetEx<K> for StdSet<K> {
    fn new_std_set() -> Self {
        Self::new()
    }

    #[inline(always)]
    fn new_std_set_with_capacity(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    #[inline(always)]
    fn std_set_drain(&mut self, capacity: usize) -> Vec<K>
    where
        K: Eq + Hash,
    {
        let result = self.drain().collect();

        self.shrink_to(capacity);

        result
    }
}

#[cfg(not(feature = "std"))]
impl<K> StdSetEx<K> for StdSet<K> {
    #[inline(always)]
    fn new_std_set() -> Self {
        Self::new()
    }

    #[inline(always)]
    fn new_std_set_with_capacity(_capacity: usize) -> Self {
        Self::new()
    }

    #[inline(always)]
    fn std_set_drain(&mut self, _capacity: usize) -> Vec<K> {
        replace(self, Self::new_std_set()).into_iter().collect()
    }
}
