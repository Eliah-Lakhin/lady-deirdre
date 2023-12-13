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
    analysis::{record::Record, table::UnitTable, Grammar, Revision},
    arena::{Entry, Id, Repository},
    std::*,
    sync::SyncBuildHasher,
};

pub(super) struct Database<N: Grammar, S: SyncBuildHasher> {
    pub(super) records: UnitTable<Repository<Record<N, S>>, S>,
    revision: AtomicU64,
}

impl<N: Grammar, S: SyncBuildHasher> Database<N, S> {
    #[inline(always)]
    pub(super) fn new_single() -> Self {
        Self {
            records: UnitTable::new_single(),
            revision: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    pub(super) fn new_multi() -> Self {
        Self {
            records: UnitTable::new_multi(0),
            revision: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    pub(super) fn revision(&self) -> Revision {
        self.revision.load(AtomicOrdering::Relaxed)
    }

    #[inline(always)]
    pub(super) fn commit(&self) {
        let _ = self.revision.fetch_add(1, AtomicOrdering::Relaxed);
    }
}

pub(super) trait AbstractDatabase: Send + Sync + 'static {
    fn deregister_attribute(&self, id: Id, entry: &Entry);
}

impl<N: Grammar, S: SyncBuildHasher> AbstractDatabase for Database<N, S> {
    fn deregister_attribute(&self, id: Id, entry: &Entry) {
        let Some(mut records_guard) = self.records.get_mut(id) else {
            return;
        };

        records_guard.remove(entry);
    }
}
