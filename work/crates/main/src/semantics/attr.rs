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
    arena::{Entry, EntryVersion, Id, Identifiable},
    report::debug_unreachable,
    semantics::{
        db::{Db, RecordRead, Storage},
        record::{Cache, Record},
        AttrError,
        AttrResult,
    },
    std::*,
    sync::{Latch, Lazy, TableEntry, TableReadGuard},
    syntax::NodeRef,
};

const DEPS_CAPACITY: usize = 3;

#[repr(transparent)]
pub struct Attr<T> {
    attr_ref: AttrRef,
    data: PhantomData<T>,
}

impl<T> Debug for Attr<T> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        match self.attr_ref.is_nil() {
            false => formatter.write_fmt(format_args!(
                "Attr(id: {:?}, entry: {:?})",
                self.attr_ref.id, self.attr_ref.entry,
            )),
            true => formatter.write_str("Attr(Nil)"),
        }
    }
}

impl<T> Identifiable for Attr<T> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.attr_ref.id
    }
}

impl<T, U> PartialEq<Attr<U>> for Attr<T> {
    #[inline(always)]
    fn eq(&self, other: &Attr<U>) -> bool {
        self.attr_ref.eq(&other.attr_ref)
    }
}

impl<T> Eq for Attr<T> {}

impl<T, U> PartialOrd<Attr<U>> for Attr<T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Attr<U>) -> Option<Ordering> {
        self.attr_ref.partial_cmp(&other.attr_ref)
    }
}

impl<T> Ord for Attr<T> {
    #[inline(always)]
    fn cmp(&self, other: &Attr<T>) -> Ordering {
        self.attr_ref.cmp(&other.attr_ref)
    }
}

impl<T> Hash for Attr<T> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.attr_ref.hash(state)
    }
}

impl<T> AsRef<AttrRef> for Attr<T> {
    #[inline(always)]
    fn as_ref(&self) -> &AttrRef {
        &self.attr_ref
    }
}

impl<T> Drop for Attr<T> {
    fn drop(&mut self) {
        if let TableEntry::Occupied(mut table_entry) =
            self.attr_ref.db.table.entry(self.attr_ref.id)
        {
            let storage = table_entry.get_mut();

            if storage.remove(&self.attr_ref.entry) {
                table_entry.remove();
            }
        }
    }
}

impl<T: Eq + Send + Sync + 'static> Attr<T> {
    pub fn new(
        db: &'static Db,
        node_ref: NodeRef,
        function: &'static (impl Fn(&mut AttrContext) -> AttrResult<T> + Send + Sync + 'static),
    ) -> Self {
        let id = node_ref.id;

        let entry;

        match db.table.entry(id) {
            TableEntry::Occupied(mut table_entry) => {
                let storage = table_entry.get_mut();

                entry = storage.insert(Record::new(node_ref, function));
            }

            TableEntry::Vacant(table_entry) => {
                let mut storage = Storage::new();

                entry = storage.insert(Record::new(node_ref, function));

                table_entry.insert(storage);
            }
        }

        Self {
            attr_ref: AttrRef { db, id, entry },
            data: PhantomData,
        }
    }

    pub fn read(&self, context: &mut AttrContext) -> AttrResult<AttrReadGuard<T>> {
        if let Some(deps) = &mut context.deps {
            let _ = deps.insert(self.attr_ref);
        }

        loop {
            let storage_guard = match self.attr_ref.db.table.get(&self.attr_ref.id) {
                Some(guard) => guard,
                None => return Err(AttrError::Deleted),
            };

            let record_guard = storage_guard.read(&self.attr_ref.entry)?;

            if record_guard.verified_at == storage_guard.revision() {
                if let Some(cache) = &record_guard.cache {
                    // Safety: The record belongs to this attribute.
                    let value = unsafe { cache.downcast_ref::<T>() };

                    // Safety:
                    //   Prolongs reference lifetime to static.
                    //   The value will be valid for as long as the guard is held.
                    let value = unsafe { transmute::<&T, &'static T>(value) };

                    // Safety:
                    //   Prolongs guard's lifetime to static.
                    //   The value will be valid for as long as the parent guard is held.
                    let record_guard =
                        unsafe { transmute::<RecordRead<'_>, RecordRead<'static>>(record_guard) };

                    return Ok(AttrReadGuard {
                        value,
                        record_guard,
                        _storage_guard: storage_guard,
                    });
                }
            }

            drop(record_guard);

            self.attr_ref
                .validate_with_cancellation(context.cancellation)?;
        }
    }
}

// Safety: Entries order reflects guards drop semantics.
pub struct AttrReadGuard<T: 'static> {
    value: &'static T,
    record_guard: RecordRead<'static>,
    _storage_guard: TableReadGuard<'static, Id, Storage>,
}

impl<T> Deref for AttrReadGuard<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> AttrReadGuard<T> {
    #[inline(always)]
    pub fn version(&self) -> EntryVersion {
        match &self.record_guard.cache {
            Some(cache) => cache.updated_at,

            // Safety: The `cache` value exists because the guard is held.
            None => unsafe { debug_unreachable!("Missing cache.") },
        }
    }
}

static NIL_NODE_REF: NodeRef = NodeRef::nil();
static UN_CANCELLABLE: Lazy<Latch> = Lazy::new(|| Latch::new());

pub struct AttrContext<'a> {
    pub(super) node_ref: &'a NodeRef,
    cancellation: &'a Latch,
    //todo consider how to manage attributes from the external databases
    deps: Option<StdSet<AttrRef>>,
}

impl<'a> AttrContext<'a> {
    #[inline(always)]
    pub fn new() -> Self {
        Self::no_tracking(&NIL_NODE_REF, UN_CANCELLABLE.deref())
    }

    #[inline(always)]
    pub fn with_cancellation(cancellation: &'a Latch) -> Self {
        Self::no_tracking(&NIL_NODE_REF, cancellation)
    }

    #[inline(always)]
    fn no_tracking(node_ref: &'a NodeRef, cancellation: &'a Latch) -> Self {
        Self {
            node_ref,
            cancellation,
            deps: None,
        }
    }

    #[inline(always)]
    fn tracking(node_ref: &'a NodeRef, cancellation: &'a Latch) -> Self {
        Self {
            node_ref,
            cancellation,
            deps: Some(StdSet::new_std_set_with_capacity(DEPS_CAPACITY)),
        }
    }

    #[inline(always)]
    pub fn node_ref(&self) -> &'a NodeRef {
        self.node_ref
    }

    #[inline(always)]
    pub fn proceed(&self) -> AttrResult<()> {
        match self.cancellation.get_relaxed() {
            false => Ok(()),
            true => Err(AttrError::Interrupted),
        }
    }
}

#[derive(Clone, Copy)]
pub struct AttrRef {
    pub db: &'static Db,
    pub id: Id,
    pub entry: Entry,
}

impl Debug for AttrRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        match self.is_nil() {
            false => formatter.write_fmt(format_args!(
                "AttrRef(id: {:?}, entry: {:?})",
                self.id, self.entry,
            )),
            true => formatter.write_str("AttrRef(Nil)"),
        }
    }
}

impl Default for AttrRef {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl PartialEq for AttrRef {
    fn eq(&self, other: &Self) -> bool {
        if self.db_addr().ne(&other.db_addr()) {
            return false;
        }

        if self.id.ne(&other.id) {
            return false;
        }

        if self.entry.ne(&other.entry) {
            return false;
        }

        true
    }
}

impl Eq for AttrRef {}

impl PartialOrd for AttrRef {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AttrRef {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.db_addr().cmp(&other.db_addr()) {
            Ordering::Equal => match self.id.cmp(&other.id) {
                Ordering::Equal => self.entry.cmp(&other.entry),
                ordering => ordering,
            },

            ordering => ordering,
        }
    }
}

impl Hash for AttrRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.db_addr().hash(state);
        self.id.hash(state);
        self.entry.hash(state);
    }
}

impl Identifiable for AttrRef {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl AttrRef {
    #[inline(always)]
    pub fn nil() -> Self {
        Self {
            db: Db::global(),
            id: Id::nil(),
            entry: Entry::Nil,
        }
    }

    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        self.id.is_nil() || self.entry.is_nil()
    }

    #[inline(always)]
    pub fn exists(&self) -> bool {
        match self.db.table.get(&self.id) {
            Some(storage) => storage.contains(&self.entry),
            None => false,
        }
    }

    pub fn version(&self) -> AttrResult<Option<EntryVersion>> {
        let storage_guard = match self.db.table.get(&self.id) {
            Some(guard) => guard,
            None => return Err(AttrError::Deleted),
        };

        let record_guard = storage_guard.read(&self.entry)?;

        Ok(record_guard.cache.as_ref().map(|cache| cache.updated_at))
    }

    pub fn invalidate(&self) -> AttrResult<()> {
        let mut storage_guard = match self.db.table.get_mut(&self.id) {
            Some(guard) => guard,
            None => return Err(AttrError::Deleted),
        };

        let mut record_guard = storage_guard.write(&self.entry)?;

        if let Some(cache) = &mut record_guard.cache {
            cache.dirty = true;
        }

        drop(record_guard);

        storage_guard.commit();

        Ok(())
    }

    #[inline(always)]
    pub fn validate(&self) -> AttrResult<()> {
        self.validate_with_cancellation(UN_CANCELLABLE.deref())
    }

    pub fn validate_with_cancellation(&self, cancellation: &Latch) -> AttrResult<()> {
        loop {
            let storage_guard = match self.db.table.get(&self.id) {
                Some(guard) => guard,
                None => return Err(AttrError::Deleted),
            };

            let mut record_guard = storage_guard.write(&self.entry)?;

            let cache = match &record_guard.cache {
                Some(cache) => cache,

                None => {
                    let mut context = AttrContext::tracking(&record_guard.node_ref, cancellation);

                    let memo = record_guard.function.invoke(&mut context)?;

                    record_guard.cache = Some(Cache {
                        dirty: false,
                        updated_at: storage_guard.revision(),
                        memo,
                        deps: match context.deps {
                            Some(deps) => deps,

                            // Safety: `context` as a tracking AttrContext.
                            None => unsafe { debug_unreachable!("Missing dependencies") },
                        },
                    });

                    record_guard.verified_at = storage_guard.revision();

                    return Ok(());
                }
            };

            if record_guard.verified_at >= storage_guard.revision() {
                return Ok(());
            }

            if !cache.dirty {
                //todo consider releasing write guard here to improve multi-thread parallelism

                let mut valid = true;

                //todo consider shuffling this iterator to improve multi-thread parallelism
                'outer: for dep in &cache.deps {
                    loop {
                        let dep_storage_guard = match dep.db.table.get(&dep.id) {
                            Some(guard) => guard,
                            None => {
                                valid = false;
                                break 'outer;
                            }
                        };

                        let dep_record_guard = match dep_storage_guard.read(&dep.entry) {
                            Ok(guard) => guard,
                            Err(_) => {
                                valid = false;
                                break 'outer;
                            }
                        };

                        if dep_record_guard.verified_at < dep_storage_guard.revision() {
                            drop(dep_record_guard);
                            drop(dep_storage_guard);

                            dep.validate_with_cancellation(cancellation)?;

                            if cancellation.get_relaxed() {
                                return Ok(());
                            }

                            continue;
                        }

                        let dep_cache = match &dep_record_guard.cache {
                            Some(cache) => cache,
                            None => {
                                valid = false;
                                break 'outer;
                            }
                        };

                        if dep_cache.updated_at <= record_guard.verified_at {
                            continue 'outer;
                        }

                        valid = false;
                        break 'outer;
                    }
                }

                if valid {
                    record_guard.verified_at = storage_guard.revision();
                    return Ok(());
                }
            }

            let mut context = AttrContext::tracking(&record_guard.node_ref, cancellation);

            let new_memo = record_guard.function.invoke(&mut context)?;

            let new_deps = match context.deps {
                Some(deps) => deps,

                // Safety: `context` as a tracking AttrContext.
                None => unsafe { debug_unreachable!("Missing dependencies") },
            };

            match &mut record_guard.cache {
                Some(cache) => {
                    // Safety: New and previous values produced by the same Record's function.
                    let same = unsafe { cache.memo.memo_eq(new_memo.as_ref()) };

                    cache.dirty = false;
                    cache.memo = new_memo;
                    cache.deps = new_deps;

                    if !same {
                        cache.updated_at = storage_guard.revision();
                    }
                }

                // Safety: Cache existence checked above.
                None => unsafe { debug_unreachable!("Missing cache.") },
            }

            record_guard.verified_at = storage_guard.revision();

            return Ok(());
        }
    }

    #[inline(always)]
    fn db_addr(&self) -> usize {
        self.db as *const Db as usize
    }
}
