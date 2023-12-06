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
    arena::{Entry, EntryVersion, Id, Repository},
    semantics::{record::Record, AttrError, AttrResult},
    std::*,
    sync::{Lazy, Table},
};

const RECORDS_CAPACITY: usize = 100;

#[repr(transparent)]
pub struct Db {
    pub(super) table: Table<Id, Storage>,
}

impl Db {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            table: Table::new(),
        }
    }

    #[inline(always)]
    pub fn global() -> &'static Self {
        static GLOBAL: Lazy<Db> = Lazy::new(|| Db::new());

        &GLOBAL
    }
}

pub(super) struct Storage {
    records: Repository<RecordCell>,
    length: usize,
}

impl Storage {
    #[inline(always)]
    pub(super) fn new() -> Self {
        Self {
            records: Repository::with_capacity(RECORDS_CAPACITY),
            length: 0,
        }
    }

    #[inline(always)]
    pub(super) fn insert(&mut self, record: Record) -> Entry {
        let entry = self.records.insert(RecordCell {
            lock: RwLock::new(record),
            writer: AtomicUsize::new(0),
        });

        self.length += 1;

        entry
    }

    #[inline(always)]
    pub(super) fn remove(&mut self, entry: &Entry) -> bool {
        if self.records.remove(entry).is_some() {
            self.records.commit(false);
            self.length -= 1;

            if self.length == 0 {
                return true;
            }
        }

        false
    }

    #[inline(always)]
    pub(super) fn commit(&mut self) {
        self.records.commit(true);
    }

    #[inline(always)]
    pub(super) fn contains(&self, entry: &Entry) -> bool {
        self.records.contains(entry)
    }

    #[inline(always)]
    pub(super) fn revision(&self) -> EntryVersion {
        self.records.revision()
    }

    #[inline(always)]
    pub(super) fn read(&self, entry: &Entry) -> AttrResult<RecordRead> {
        let cell = match self.records.get(entry) {
            Some(cell) => cell,
            None => return Err(AttrError::Deleted),
        };

        let current_writer = cell.writer.load(AtomicOrdering::Relaxed);

        if current_writer == current_thread_id() {
            return Err(AttrError::CycleDetected);
        }

        Ok(RecordRead {
            guard: cell
                .lock
                .read()
                .unwrap_or_else(|poison| poison.into_inner()),
        })
    }

    #[inline(always)]
    pub(super) fn write(&self, entry: &Entry) -> AttrResult<RecordWrite> {
        let cell = match self.records.get(entry) {
            Some(cell) => cell,
            None => return Err(AttrError::Deleted),
        };

        let current_writer = cell.writer.load(AtomicOrdering::Relaxed);

        let thread_id = current_thread_id();

        if current_writer == thread_id {
            return Err(AttrError::CycleDetected);
        }

        let guard = cell
            .lock
            .write()
            .unwrap_or_else(|poison| poison.into_inner());

        cell.writer.store(thread_id, AtomicOrdering::Relaxed);

        Ok(RecordWrite {
            guard,
            writer: &cell.writer,
        })
    }
}

#[repr(transparent)]
pub(super) struct RecordRead<'a> {
    guard: RwLockReadGuard<'a, Record>,
}

impl<'a> Deref for RecordRead<'a> {
    type Target = Record;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

pub(super) struct RecordWrite<'a> {
    guard: RwLockWriteGuard<'a, Record>,
    writer: &'a AtomicUsize,
}

impl<'a> Drop for RecordWrite<'a> {
    fn drop(&mut self) {
        self.writer.store(0, AtomicOrdering::Relaxed);
    }
}

impl<'a> Deref for RecordWrite<'a> {
    type Target = Record;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a> DerefMut for RecordWrite<'a> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

struct RecordCell {
    lock: RwLock<Record>,
    writer: AtomicUsize,
}

type ThreadId = usize;

fn current_thread_id() -> ThreadId {
    thread_local! {
        static THREAD_ID: usize = 0;
    }

    THREAD_ID.with(|id| id as *const usize as usize)
}

#[cfg(test)]
mod tests {
    use crate::{semantics::db::current_thread_id, std::*};

    #[test]
    fn test_thread_id() {
        let this_id = current_thread_id();

        let other_id = spawn(|| current_thread_id()).join().unwrap();

        assert_ne!(this_id, other_id);
    }
}
