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
        btree_map::Iter as BTreeMapIter,
        btree_map::Range as BTreeMapRange,
        btree_map::RangeMut as BTreeMapRangeMut,
        BTreeMap,
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
    borrow::{Borrow, BorrowMut},
    cell::UnsafeCell,
    clone::Clone,
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    convert::{AsRef, From, Into},
    debug_assert,
    debug_assert_eq,
    debug_assert_ne,
    default::Default,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    hint::unreachable_unchecked,
    iter::{
        DoubleEndedIterator,
        Enumerate,
        ExactSizeIterator,
        FilterMap,
        FromIterator,
        FusedIterator,
        IntoIterator,
        Iterator,
        Map,
    },
    marker::{Copy, PhantomData, Send, Sized, Sync},
    matches,
    mem::{forget, replace, take, transmute, ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ops::{
        AddAssign,
        Deref,
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
    str::Chars,
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
    borrow::{Borrow, BorrowMut, Cow, ToOwned},
    boxed::Box,
    cell::UnsafeCell,
    clone::Clone,
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    collections::{
        btree_map::Iter as BTreeMapIter,
        btree_map::Range as BTreeMapRange,
        btree_map::RangeMut as BTreeMapRangeMut,
        BTreeMap,
        LinkedList,
        VecDeque,
    },
    convert::{AsRef, From, Into},
    debug_assert,
    debug_assert_eq,
    debug_assert_ne,
    default::Default,
    fmt::{Debug, Display, Error as FmtError, Formatter, Result as FmtResult},
    format,
    hint::unreachable_unchecked,
    iter::{
        DoubleEndedIterator,
        Enumerate,
        ExactSizeIterator,
        FilterMap,
        FromIterator,
        FusedIterator,
        IntoIterator,
        Iterator,
        Map,
    },
    marker::{Copy, PhantomData, Send, Sized, Sync},
    matches,
    mem::{forget, replace, take, transmute, ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ops::{
        AddAssign,
        Deref,
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
    str::Chars,
    string::{String, ToString},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering},
        Arc,
        Weak as AsyncWeak,
    },
    todo,
    unimplemented,
    unreachable,
    vec::IntoIter,
    vec::Vec,
};
