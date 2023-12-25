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
        record::{Cache, Cell, Record},
        table::UnitTableReadGuard,
        AnalysisError,
        AnalysisResult,
        AttrContext,
        AttrReadGuard,
        AttrRef,
        Computable,
        Grammar,
    },
    arena::Repository,
    std::*,
    sync::{Shared, SyncBuildHasher},
};

impl AttrRef {
    // Safety: If `CHECK == false` then `C` properly describes underlying attribute's computable data.
    pub(super) unsafe fn fetch<'a, const CHECK: bool, C: Computable, S: SyncBuildHasher>(
        &self,
        context: &mut AttrContext<'a, C::Node, S>,
    ) -> AnalysisResult<AttrReadGuard<'a, C, S>> {
        loop {
            let Some(records_guard) = context.analyzer().database.records.get(self.id) else {
                return Err(AnalysisError::MissingDocument);
            };

            let Some(record) = records_guard.get(&self.entry) else {
                return Err(AnalysisError::MissingAttribute);
            };

            let cell_guard = record.read();

            if cell_guard.verified_at >= context.db_revision() {
                if let Some(cache) = &cell_guard.cache {
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

            self.validate(context)?;
        }
    }

    fn validate<N: Grammar, S: SyncBuildHasher>(
        &self,
        context: &AttrContext<N, S>,
    ) -> AnalysisResult<()> {
        loop {
            let Some(records) = context.analyzer().database.records.get(self.id) else {
                return Err(AnalysisError::MissingDocument);
            };

            let Some(record) = records.get(&self.entry) else {
                return Err(AnalysisError::MissingAttribute);
            };

            let mut record_write_guard = record.write(context.handle())?;
            let cell = record_write_guard.deref_mut();

            let Some(cache) = &mut cell.cache else {
                let mut forked = context.fork(&cell.node_ref);
                let memo = cell.function.invoke(&mut forked)?;
                let deps = Shared::new(forked.into_deps());

                cell.cache = Some(Cache {
                    dirty: false,
                    updated_at: context.db_revision(),
                    memo,
                    deps,
                });

                cell.verified_at = context.db_revision();

                return Ok(());
            };

            if cell.verified_at >= context.db_revision() {
                return Ok(());
            }

            if !cache.dirty {
                let mut valid = true;
                let mut deps_verified = true;

                for dep in cache.deps.as_ref() {
                    let Some(dep_records) = context.analyzer().database.records.get(dep.id) else {
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
                        deps_verified && dep_record_read_guard.verified_at >= context.db_revision();
                }

                if valid {
                    if deps_verified {
                        cell.verified_at = context.db_revision();
                        return Ok(());
                    }

                    context.proceed()?;

                    let deps = cache.deps.clone();

                    drop(record_write_guard);

                    //todo dependencies shuffling probably should improve parallelism between tasks
                    for dep in deps.as_ref() {
                        dep.validate(context)?;
                    }

                    continue;
                }
            }

            let mut forked = context.fork(&cell.node_ref);
            let new_memo = cell.function.invoke(&mut forked)?;
            let new_deps = Shared::new(forked.into_deps());

            // Safety: New and previous values produced by the same Cell function.
            let same = unsafe { cache.memo.memo_eq(new_memo.as_ref()) };

            cache.dirty = false;
            cache.memo = new_memo;
            cache.deps = new_deps;

            if !same {
                cache.updated_at = context.db_revision();
            }

            cell.verified_at = context.db_revision();

            return Ok(());
        }
    }
}
