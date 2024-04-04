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
        database::{CacheDeps, Record, RecordCache, RecordData, RecordReadGuard},
        AnalysisError,
        AnalysisResult,
        Analyzer,
        AttrRef,
        Classifier,
        DocumentReadGuard,
        Event,
        Grammar,
        Handle,
        Revision,
        DOC_REMOVED_EVENT,
        DOC_UPDATED_EVENT,
    },
    arena::{Id, Repo},
    report::debug_unreachable,
    std::*,
    sync::{Shared, SyncBuildHasher, TableReadGuard},
    syntax::{NodeRef, NIL_NODE_REF},
};

pub trait Computable: Send + Sync + 'static {
    type Node: Grammar;

    fn compute<S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, S>,
    ) -> AnalysisResult<Self>
    where
        Self: Sized;
}

pub trait SharedComputable: Send + Sync + 'static {
    type Node: Grammar;

    fn compute_shared<S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, S>,
    ) -> AnalysisResult<Shared<Self>>
    where
        Self: Sized;
}

impl<D: SharedComputable> Computable for Shared<D> {
    type Node = D::Node;

    fn compute<S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, S>,
    ) -> AnalysisResult<Self> {
        Ok(D::compute_shared(context)?)
    }
}

pub struct AttrContext<'a, N: Grammar, S: SyncBuildHasher> {
    analyzer: &'a Analyzer<N, S>,
    revision: Revision,
    handle: &'a Handle,
    node_ref: &'a NodeRef,
    deps: CacheDeps<N, S>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> AttrContext<'a, N, S> {
    #[inline(always)]
    pub(super) fn new(
        analyzer: &'a Analyzer<N, S>,
        revision: Revision,
        handle: &'a Handle,
    ) -> Self {
        Self {
            analyzer,
            revision,
            handle,
            node_ref: &NIL_NODE_REF,
            deps: CacheDeps::default(),
        }
    }

    #[inline(always)]
    pub fn node_ref(&self) -> &'a NodeRef {
        self.node_ref
    }

    #[inline(always)]
    pub fn contains_doc(&mut self, id: Id) -> bool {
        let result = self.analyzer.docs.contains_key(&id);

        if result && id != self.node_ref.id {
            self.subscribe(Id::nil(), DOC_REMOVED_EVENT);
        }

        result
    }

    #[inline(always)]
    pub fn read_doc(&mut self, id: Id) -> AnalysisResult<DocumentReadGuard<'a, N, S>> {
        let Some(guard) = self.analyzer.docs.get(&id) else {
            return Err(AnalysisError::MissingDocument);
        };

        if id != self.node_ref.id {
            self.subscribe(id, DOC_REMOVED_EVENT);
        }

        Ok(DocumentReadGuard::from(guard))
    }

    #[inline(always)]
    pub fn read_class(
        &mut self,
        id: Id,
        class: &<N::Classifier as Classifier>::Class,
    ) -> AnalysisResult<Shared<HashSet<NodeRef, S>>> {
        let _ = self.deps.classes.insert((id, class.clone()));

        let Some(guard) = self.analyzer.docs.get(&id) else {
            return Err(AnalysisError::MissingDocument);
        };

        let Some(class_to_nodes) = guard.classes_to_nodes.get(class) else {
            self.subscribe(id, DOC_UPDATED_EVENT);
            return Ok(Shared::default());
        };

        if id != self.node_ref.id {
            self.subscribe(id, DOC_REMOVED_EVENT);
        }

        Ok(class_to_nodes.nodes.clone())
    }

    #[inline(always)]
    pub fn subscribe(&mut self, id: Id, event: Event) {
        let _ = self.deps.events.insert((id, event));
    }

    #[inline(always)]
    pub fn proceed(&self) -> AnalysisResult<()> {
        if self.handle.triggered() {
            return Err(AnalysisError::Interrupted);
        }

        Ok(())
    }

    #[inline(always)]
    pub(super) fn fork(&self, node_ref: &'a NodeRef) -> AttrContext<'a, N, S> {
        AttrContext {
            analyzer: self.analyzer,
            revision: self.revision,
            handle: self.handle,
            node_ref,
            deps: CacheDeps::default(),
        }
    }

    #[inline(always)]
    pub(super) fn track(&mut self, dep: &AttrRef) {
        let _ = self.deps.attrs.insert(*dep);
    }

    #[inline(always)]
    pub(super) fn into_deps(self) -> Shared<CacheDeps<N, S>> {
        Shared::new(self.deps)
    }
}

// Safety: Entries order reflects guards drop semantics.
#[allow(dead_code)]
pub struct AttrReadGuard<'a, C: Computable, S: SyncBuildHasher = RandomState> {
    pub(super) data: &'a C,
    pub(super) cell_guard: RecordReadGuard<'a, <C as Computable>::Node, S>,
    pub(super) records_guard: TableReadGuard<'a, Id, Repo<Record<C::Node, S>>, S>,
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

impl<'a, C: Computable, S: SyncBuildHasher> AttrReadGuard<'a, C, S> {
    #[inline(always)]
    pub(super) fn attr_revision(&self) -> Revision {
        let Some(cache) = &self.cell_guard.cache else {
            unsafe { debug_unreachable!("AttrReadGuard without cache.") }
        };

        cache.updated_at
    }
}

impl AttrRef {
    // Safety: If `CHECK == false` then `C` properly describes underlying attribute's computable data.
    pub(super) unsafe fn fetch<'a, const CHECK: bool, C: Computable, S: SyncBuildHasher>(
        &self,
        context: &mut AttrContext<'a, C::Node, S>,
    ) -> AnalysisResult<AttrReadGuard<'a, C, S>> {
        loop {
            let Some(records_guard) = context.analyzer.db.records.get(&self.id) else {
                return Err(AnalysisError::MissingDocument);
            };

            let Some(record) = records_guard.get(&self.entry) else {
                return Err(AnalysisError::MissingAttribute);
            };

            let record_read_guard = record.read(&context.analyzer.db.timeout)?;

            if record_read_guard.verified_at >= context.revision {
                if let Some(cache) = &record_read_guard.cache {
                    let data = match CHECK {
                        true => cache.downcast::<C>()?,

                        // Safety: Upheld by the caller.
                        false => unsafe { cache.downcast_unchecked::<C>() },
                    };

                    context.track(self);

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The reference will ve valid for as long as the parent guard is held.
                    let data = unsafe { transmute::<&C, &'a C>(data) };

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The guard will ve valid for as long as the parent guard is held.
                    let cell_guard = unsafe {
                        transmute::<
                            RecordReadGuard<<C as Computable>::Node, S>,
                            RecordReadGuard<'a, <C as Computable>::Node, S>,
                        >(record_read_guard)
                    };

                    // Safety: Prolongs lifetime to Analyzer's lifetime.
                    //         The reference will ve valid for as long as the Analyzer is held.
                    let records_guard = unsafe {
                        transmute::<
                            TableReadGuard<Id, Repo<Record<<C as Computable>::Node, S>>, S>,
                            TableReadGuard<'a, Id, Repo<Record<<C as Computable>::Node, S>>, S>,
                        >(records_guard)
                    };

                    return Ok(AttrReadGuard {
                        data,
                        cell_guard,
                        records_guard,
                    });
                }
            }

            drop(record_read_guard);
            drop(records_guard);

            self.validate(context)?;
        }
    }

    fn validate<N: Grammar, S: SyncBuildHasher>(
        &self,
        context: &AttrContext<N, S>,
    ) -> AnalysisResult<()> {
        loop {
            let Some(records) = context.analyzer.db.records.get(&self.id) else {
                return Err(AnalysisError::MissingDocument);
            };

            let Some(record) = records.get(&self.entry) else {
                return Err(AnalysisError::MissingAttribute);
            };

            {
                let record_read_guard = record.read(&context.analyzer.db.timeout)?;

                if record_read_guard.verified_at >= context.revision {
                    return Ok(());
                }
            }

            let mut record_write_guard = record.write(&context.analyzer.db.timeout)?;

            let record_data = record_write_guard.deref_mut();

            let Some(cache) = &mut record_data.cache else {
                let mut forked = context.fork(&record_data.node_ref);
                let memo = record_data.function.invoke(&mut forked)?;
                let deps = forked.into_deps();

                record_data.cache = Some(RecordCache {
                    dirty: false,
                    updated_at: context.revision,
                    memo,
                    deps,
                });

                record_data.verified_at = context.revision;

                return Ok(());
            };

            if record_data.verified_at >= context.revision {
                return Ok(());
            }

            if !cache.dirty && !cache.deps.as_ref().events.is_empty() {
                for (id, event) in &cache.deps.as_ref().events {
                    let Some(guard) = context.analyzer.events.get(id) else {
                        continue;
                    };

                    let Some(updated_at) = guard.get(event) else {
                        continue;
                    };

                    if *updated_at > record_data.verified_at {
                        cache.dirty = true;
                        break;
                    }
                }
            }

            if !cache.dirty && !cache.deps.as_ref().classes.is_empty() {
                for (id, class) in &cache.deps.as_ref().classes {
                    let Some(guard) = context.analyzer.docs.get(id) else {
                        continue;
                    };

                    let Some(class_to_nodes) = guard.classes_to_nodes.get(class) else {
                        continue;
                    };

                    if class_to_nodes.revision > record_data.verified_at {
                        cache.dirty = true;
                        break;
                    }
                }
            }

            if !cache.dirty && !cache.deps.as_ref().attrs.is_empty() {
                let mut deps_verified = true;

                for attr_ref in &cache.deps.as_ref().attrs {
                    let Some(dep_records) = context.analyzer.db.records.get(&attr_ref.id) else {
                        cache.dirty = true;
                        break;
                    };

                    let Some(dep_record) = dep_records.get(&attr_ref.entry) else {
                        cache.dirty = true;
                        break;
                    };

                    let dep_record_read_guard = dep_record.read(&context.analyzer.db.timeout)?;

                    let Some(dep_cache) = &dep_record_read_guard.cache else {
                        cache.dirty = true;
                        break;
                    };

                    if dep_cache.dirty {
                        cache.dirty = true;
                        break;
                    }

                    if dep_cache.updated_at > record_data.verified_at {
                        cache.dirty = true;
                        break;
                    }

                    deps_verified =
                        deps_verified && dep_record_read_guard.verified_at >= context.revision;
                }

                if !cache.dirty {
                    if deps_verified {
                        record_data.verified_at = context.revision;
                        return Ok(());
                    }

                    context.proceed()?;

                    let deps = cache.deps.clone();

                    drop(record_write_guard);

                    for attr_ref in &deps.as_ref().attrs {
                        attr_ref.validate(context)?;
                    }

                    continue;
                }
            }

            if !cache.dirty {
                record_data.verified_at = context.revision;
                return Ok(());
            }

            let mut forked = context.fork(&record_data.node_ref);
            let new_memo = record_data.function.invoke(&mut forked)?;
            let new_deps = forked.into_deps();

            // Safety: New and previous values produced by the same Cell function.
            let same = unsafe { cache.memo.memo_eq(new_memo.as_ref()) };

            cache.dirty = false;
            cache.memo = new_memo;
            cache.deps = new_deps;

            if !same {
                cache.updated_at = context.revision;
            }

            record_data.verified_at = context.revision;

            return Ok(());
        }
    }
}
