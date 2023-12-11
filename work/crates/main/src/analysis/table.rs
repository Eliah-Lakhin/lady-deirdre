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

use crate::{
    arena::Id,
    report::debug_unreachable,
    std::*,
    sync::{SyncBuildHasher, Table, TableReadGuard, TableWriteGuard},
};

pub(super) enum UnitTable<V, S> {
    Single(RwLock<Option<(Id, V)>>),
    Multi(Table<Id, V, S>),
}

impl<V, S: SyncBuildHasher> UnitTable<V, S> {
    #[inline(always)]
    pub(super) fn new_single() -> Self {
        Self::Single(RwLock::new(None))
    }

    #[inline(always)]
    pub(super) fn new_multi(capacity: usize) -> Self {
        Self::Multi(Table::with_capacity_and_hasher(capacity, S::default()))
    }

    // Safety: The instance does not have entries with provided `id`.
    pub(super) unsafe fn insert(&self, id: Id, value: V) {
        match self {
            Self::Single(table) => {
                let mut guard = table.write().unwrap_or_else(|poison| poison.into_inner());

                if guard.is_some() {
                    panic!("The singleton analyzer may not have more than one unit.");
                }

                *guard = Some((id, value));
            }

            Self::Multi(table) => {
                if table.insert(id, value).is_some() {
                    // Safety: Upheld by the caller.
                    unsafe { debug_unreachable!("Duplicate table entry.") }
                }
            }
        }
    }

    pub(super) fn remove(&self, id: Id) -> bool {
        match self {
            Self::Single(table) => {
                let mut guard = table.write().unwrap_or_else(|poison| poison.into_inner());

                let Some((managed_id, _)) = guard.deref() else {
                    return false;
                };

                if managed_id != &id {
                    return false;
                }

                *guard = None;

                true
            }

            Self::Multi(table) => table.remove(&id).is_some(),
        }
    }

    pub(super) fn contains(&self, id: Id) -> bool {
        match self {
            Self::Single(table) => {
                let mut guard = table.read().unwrap_or_else(|poison| poison.into_inner());

                let Some((managed_id, _)) = guard.deref() else {
                    return false;
                };

                managed_id == &id
            }

            Self::Multi(table) => table.contains_key(&id),
        }
    }

    pub(super) fn get(&self, id: Id) -> Option<UnitTableReadGuard<V, S>> {
        match self {
            Self::Single(table) => {
                let mut guard = table.read().unwrap_or_else(|poison| poison.into_inner());

                let Some((managed_id, _)) = guard.deref() else {
                    return None;
                };

                if managed_id != &id {
                    return None;
                }

                Some(UnitTableReadGuard::Single(guard))
            }

            Self::Multi(table) => Some(UnitTableReadGuard::Multi(table.get(&id)?)),
        }
    }

    pub(super) fn get_mut(&self, id: Id) -> Option<UnitTableWriteGuard<V, S>> {
        match self {
            Self::Single(table) => {
                let mut guard = table.write().unwrap_or_else(|poison| poison.into_inner());

                let Some((managed_id, _)) = guard.deref() else {
                    return None;
                };

                if managed_id != &id {
                    return None;
                }

                Some(UnitTableWriteGuard::Single(guard))
            }

            Self::Multi(table) => Some(UnitTableWriteGuard::Multi(table.get_mut(&id)?)),
        }
    }
}

pub(super) enum UnitTableReadGuard<'a, V, S> {
    Single(RwLockReadGuard<'a, Option<(Id, V)>>),
    Multi(TableReadGuard<'a, Id, V, S>),
}

impl<'a, V, S> Deref for UnitTableReadGuard<'a, V, S> {
    type Target = V;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Single(guard) => match guard.deref() {
                Some((_, value)) => value,

                // Safety: Discriminant checked during creation.
                None => unsafe { debug_unreachable!("Void guard.") },
            },

            Self::Multi(guard) => guard.deref(),
        }
    }
}

pub(super) enum UnitTableWriteGuard<'a, V, S> {
    Single(RwLockWriteGuard<'a, Option<(Id, V)>>),
    Multi(TableWriteGuard<'a, Id, V, S>),
}

impl<'a, V, S> Deref for UnitTableWriteGuard<'a, V, S> {
    type Target = V;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Single(guard) => match guard.deref() {
                Some((_, value)) => value,

                // Safety: Discriminant checked during creation.
                None => unsafe { debug_unreachable!("Void guard.") },
            },

            Self::Multi(guard) => guard.deref(),
        }
    }
}

impl<'a, V, S> DerefMut for UnitTableWriteGuard<'a, V, S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Single(guard) => match guard.deref_mut() {
                Some((_, value)) => value,

                // Safety: Discriminant checked during creation.
                None => unsafe { debug_unreachable!("Void guard.") },
            },

            Self::Multi(guard) => guard.deref_mut(),
        }
    }
}
