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
    analysis::{
        database::AbstractDatabase,
        record::{Cache, Cell, Record},
        table::UnitTableReadGuard,
        AbstractFeature,
        AnalysisError,
        AnalysisResult,
        AnalysisTask,
        Feature,
        FeatureInitializer,
        FeatureInvalidator,
        Grammar,
        MutationTask,
        ScopeAttr,
    },
    arena::{Entry, Id, Identifiable, Repository},
    report::debug_unreachable,
    std::*,
    sync::{Shared, SyncBuildHasher},
    syntax::{Key, NodeRef},
};

#[repr(transparent)]
pub struct Attr<C: Computable> {
    inner: AttrInner,
    _data: PhantomData<C>,
}

impl<C: Computable> Debug for Attr<C> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        let attr_ref = self.as_ref();

        match attr_ref.is_nil() {
            false => formatter.write_fmt(format_args!(
                "Attr(id: {:?}, entry: {:?})",
                attr_ref.id, attr_ref.entry,
            )),

            true => formatter.write_str("Attr(Nil)"),
        }
    }
}

impl<C: Computable> Identifiable for Attr<C> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.as_ref().id
    }
}

impl<T: Computable, U: Computable> PartialEq<Attr<U>> for Attr<T> {
    #[inline(always)]
    fn eq(&self, other: &Attr<U>) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl<C: Computable> Eq for Attr<C> {}

impl<T: Computable, U: Computable> PartialOrd<Attr<U>> for Attr<T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Attr<U>) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<C: Computable> Ord for Attr<C> {
    #[inline(always)]
    fn cmp(&self, other: &Attr<C>) -> Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<C: Computable> Hash for Attr<C> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<C: Computable> AsRef<AttrRef> for Attr<C> {
    #[inline(always)]
    fn as_ref(&self) -> &AttrRef {
        static NIL_REF: AttrRef = AttrRef::nil();

        let AttrInner::Init { attr_ref, .. } = &self.inner else {
            return &NIL_REF;
        };

        attr_ref
    }
}

impl<C: Computable> Drop for Attr<C> {
    fn drop(&mut self) {
        let AttrInner::Init { attr_ref, database } = &self.inner else {
            return;
        };

        let Some(database) = database.upgrade() else {
            return;
        };

        database.deregister_attribute(attr_ref.id, &attr_ref.entry);
    }
}

impl<C: Computable> AbstractFeature for Attr<C> {
    #[inline(always)]
    fn attr_ref(&self) -> &AttrRef {
        self.as_ref()
    }

    #[inline(always)]
    fn feature(&self, _key: Key) -> AnalysisResult<&dyn AbstractFeature> {
        Err(AnalysisError::MissingFeature)
    }

    #[inline(always)]
    fn feature_keys(&self) -> &'static [&'static Key] {
        &[]
    }
}

impl<C: Computable + Eq> Feature for Attr<C> {
    type Node = C::Node;

    #[inline(always)]
    fn new_uninitialized(node_ref: NodeRef) -> Self {
        Self {
            inner: AttrInner::Uninit(node_ref),
            _data: PhantomData,
        }
    }

    fn initialize<S: SyncBuildHasher>(
        &mut self,
        initializer: &mut FeatureInitializer<Self::Node, S>,
    ) {
        let AttrInner::Uninit(node_ref) = &self.inner else {
            return;
        };

        let id = node_ref.id;

        #[cfg(debug_assertions)]
        if initializer.id() != id {
            panic!("Attribute and Compilation Unit mismatch.");
        }

        let node_ref = *node_ref;

        let (database, entry) = initializer.register_attribute::<C>(node_ref);

        self.inner = AttrInner::Init {
            attr_ref: AttrRef { id, entry },
            database,
        };
    }

    fn invalidate<S: SyncBuildHasher>(&self, invalidator: &mut FeatureInvalidator<Self::Node, S>) {
        let AttrInner::Init { attr_ref, .. } = &self.inner else {
            return;
        };

        #[cfg(debug_assertions)]
        if invalidator.id() != attr_ref.id {
            panic!("Attribute and Compilation Unit mismatch.");
        }

        invalidator.invalidate_attribute(&attr_ref.entry);
    }

    #[inline(always)]
    fn scope_attr(&self) -> AnalysisResult<&ScopeAttr<Self::Node>> {
        if TypeId::of::<Self>() == TypeId::of::<ScopeAttr<Self::Node>>() {
            // Safety: Type ids match.
            return Ok(unsafe { transmute::<&Self, &ScopeAttr<Self::Node>>(self) });
        }

        Err(AnalysisError::MissingScope)
    }
}

impl<C: Computable> Attr<C> {
    pub fn read<'a, S: SyncBuildHasher>(
        &self,
        task: &mut AnalysisTask<'a, C::Node, S>,
    ) -> AnalysisResult<AttrReadGuard<'a, C, S>> {
        let attr_ref = self.as_ref();

        if attr_ref.is_nil() {
            return Err(AnalysisError::UninitAttribute);
        }

        loop {
            let Some(records_guard) = task.analyzer.database.records.get(attr_ref.id) else {
                return Err(AnalysisError::MissingDocument);
            };

            let Some(record) = records_guard.get(&attr_ref.entry) else {
                return Err(AnalysisError::MissingAttribute);
            };

            let cell_guard = record.read();

            if cell_guard.verified_at >= task.revision {
                if let Some(cache) = &cell_guard.cache {
                    // Safety: Attributes data came from the C::compute function.
                    let data = unsafe { cache.downcast_unchecked::<C>() };

                    task.track(attr_ref);

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The reference will ve valid for as long as the parent guard is held.
                    let data = unsafe { transmute::<&C, &'a C>(data) };

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The guard will ve valid for as long as the parent guard is held.
                    let cell_guard = unsafe {
                        transmute::<
                            RwLockReadGuard<Cell<<C as Computable>::Node, S>>,
                            RwLockReadGuard<'a, Cell<<C as Computable>::Node, S>>,
                        >(cell_guard)
                    };

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The reference will ve valid for as long as the Analyzer is held.
                    let records_guard = unsafe {
                        transmute::<
                            UnitTableReadGuard<Repository<Record<<C as Computable>::Node, S>>, S>,
                            UnitTableReadGuard<
                                'a,
                                Repository<Record<<C as Computable>::Node, S>>,
                                S,
                            >,
                        >(records_guard)
                    };

                    return Ok(AttrReadGuard {
                        data,
                        cell_guard,
                        records_guard,
                    });
                }
            }

            drop(cell_guard);
            drop(records_guard);

            attr_ref.validate(task)?;
        }
    }
}

pub trait Computable: Send + Sync + 'static {
    type Node: Grammar;

    fn compute<S: SyncBuildHasher>(task: &mut AnalysisTask<Self::Node, S>) -> AnalysisResult<Self>
    where
        Self: Sized;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttrRef {
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

impl Identifiable for AttrRef {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl Default for AttrRef {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl AttrRef {
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            entry: Entry::Nil,
        }
    }

    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.id.is_nil() || self.entry.is_nil()
    }

    pub fn read<'a, C: Computable, S: SyncBuildHasher>(
        &self,
        task: &mut AnalysisTask<'a, C::Node, S>,
    ) -> AnalysisResult<AttrReadGuard<'a, C, S>> {
        loop {
            let Some(records_guard) = task.analyzer.database.records.get(self.id) else {
                return Err(AnalysisError::MissingDocument);
            };

            let Some(record) = records_guard.get(&self.entry) else {
                return Err(AnalysisError::MissingAttribute);
            };

            let cell_guard = record.read();

            if cell_guard.verified_at >= task.revision {
                if let Some(cache) = &cell_guard.cache {
                    let data = cache.downcast::<C>()?;

                    task.track(self);

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The reference will ve valid for as long as the parent guard is held.
                    let data = unsafe { transmute::<&C, &'a C>(data) };

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The guard will ve valid for as long as the parent guard is held.
                    let cell_guard = unsafe {
                        transmute::<
                            RwLockReadGuard<Cell<<C as Computable>::Node, S>>,
                            RwLockReadGuard<'a, Cell<<C as Computable>::Node, S>>,
                        >(cell_guard)
                    };

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The reference will ve valid for as long as the Analyzer is held.
                    let records_guard = unsafe {
                        transmute::<
                            UnitTableReadGuard<Repository<Record<<C as Computable>::Node, S>>, S>,
                            UnitTableReadGuard<
                                'a,
                                Repository<Record<<C as Computable>::Node, S>>,
                                S,
                            >,
                        >(records_guard)
                    };

                    return Ok(AttrReadGuard {
                        data,
                        cell_guard,
                        records_guard,
                    });
                }
            }

            drop(cell_guard);
            drop(records_guard);

            self.validate(task)?;
        }
    }

    pub fn invalidate<N: Grammar, S: SyncBuildHasher>(&self, task: &mut MutationTask<N, S>) {
        let Some(records) = task.analyzer.database.records.get(self.id) else {
            #[cfg(debug_assertions)]
            {
                panic!("Attribute does not belong to specified Analyzer.");
            }

            #[cfg(not(debug_assertions))]
            {
                return;
            }
        };

        let Some(record) = records.get(&self.entry) else {
            return;
        };

        record.invalidate();
        task.analyzer.database.commit();
    }

    #[inline(always)]
    pub fn is_valid_ref<N: Grammar, S: SyncBuildHasher>(&self, task: &AnalysisTask<N, S>) -> bool {
        let Some(records) = task.analyzer.database.records.get(self.id) else {
            return false;
        };

        records.contains(&self.entry)
    }

    fn validate<N: Grammar, S: SyncBuildHasher>(
        &self,
        task: &AnalysisTask<N, S>,
    ) -> AnalysisResult<()> {
        loop {
            let Some(records) = task.analyzer.database.records.get(self.id) else {
                return Err(AnalysisError::MissingDocument);
            };

            let Some(record) = records.get(&self.entry) else {
                return Err(AnalysisError::MissingAttribute);
            };

            let mut record_write_guard = record.write(task.handle())?;
            let cell = record_write_guard.deref_mut();

            let Some(cache) = &mut cell.cache else {
                let mut forked = task.fork(&cell.node_ref);

                let memo = cell.function.invoke(&mut forked)?;

                let deps = match forked.take_deps() {
                    Some(deps) => Shared::new(deps),

                    // Safety: Forked tasks always have dependencies set.
                    None => unsafe { debug_unreachable!("Missing dependencies") },
                };

                cell.cache = Some(Cache {
                    dirty: false,
                    updated_at: task.revision,
                    memo,
                    deps,
                });

                cell.verified_at = task.revision;

                return Ok(());
            };

            if cell.verified_at >= task.revision {
                return Ok(());
            }

            if !cache.dirty {
                let mut valid = true;
                let mut deps_verified = true;

                for dep in cache.deps.as_ref() {
                    let Some(dep_records) = task.analyzer.database.records.get(dep.id) else {
                        valid = false;
                        break;
                    };

                    let Some(dep_record) = dep_records.get(&dep.entry) else {
                        valid = false;
                        break;
                    };

                    let dep_record_read_guard = dep_record.read();

                    let Some(dep_cache) = &dep_record_read_guard.cache else {
                        valid = false;
                        break;
                    };

                    if dep_cache.dirty {
                        valid = false;
                        break;
                    }

                    if dep_cache.updated_at > cell.verified_at {
                        valid = false;
                        break;
                    }

                    deps_verified =
                        deps_verified && dep_record_read_guard.verified_at >= task.revision;
                }

                if valid {
                    if deps_verified {
                        cell.verified_at = task.revision;
                        return Ok(());
                    }

                    task.proceed()?;

                    let deps = cache.deps.clone();

                    drop(record_write_guard);

                    //todo dependencies shuffling probably should improve parallelism between tasks
                    for dep in deps.as_ref() {
                        dep.validate(task)?;
                    }

                    continue;
                }
            }

            let mut forked = task.fork(&cell.node_ref);

            let new_memo = cell.function.invoke(&mut forked)?;

            let new_deps = match forked.take_deps() {
                Some(deps) => Shared::new(deps),

                // Safety: Forked tasks always have dependencies set.
                None => unsafe { debug_unreachable!("Missing dependencies") },
            };

            // Safety: New and previous values produced by the same Cell function.
            let same = unsafe { cache.memo.memo_eq(new_memo.as_ref()) };

            cache.dirty = false;
            cache.memo = new_memo;
            cache.deps = new_deps;

            if !same {
                cache.updated_at = task.revision;
            }

            cell.verified_at = task.revision;

            return Ok(());
        }
    }
}

// Safety: Entries order reflects guards drop semantics.
#[allow(dead_code)]
pub struct AttrReadGuard<'a, C: Computable, S: SyncBuildHasher = RandomState> {
    data: &'a C,
    cell_guard: RwLockReadGuard<'a, Cell<<C as Computable>::Node, S>>,
    records_guard: UnitTableReadGuard<'a, Repository<Record<C::Node, S>>, S>,
}

impl<'a, C: Computable + Debug, S: SyncBuildHasher> Debug for AttrReadGuard<'a, C, S> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(self.data, formatter)
    }
}

impl<'a, C: Computable + Display, S: SyncBuildHasher> Display for AttrReadGuard<'a, C, S> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self.data, formatter)
    }
}

impl<'a, C: Computable, S: SyncBuildHasher> Deref for AttrReadGuard<'a, C, S> {
    type Target = C;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

enum AttrInner {
    Uninit(NodeRef),

    Init {
        attr_ref: AttrRef,
        database: Weak<dyn AbstractDatabase>,
    },
}
