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
        record::{Cell, Record},
        table::UnitTableReadGuard,
        AbstractFeature,
        AbstractTask,
        AnalysisError,
        AnalysisResult,
        AnalysisTask,
        Analyzer,
        Feature,
        FeatureInitializer,
        FeatureInvalidator,
        Grammar,
        MutationTask,
        Revision,
        ScopeAttr,
    },
    arena::{Entry, Id, Identifiable, Repository},
    report::{debug_unreachable, system_panic},
    std::*,
    sync::{Latch, Lazy, SyncBuildHasher},
    syntax::{Key, NodeRef, NIL_NODE_REF},
};

pub static NIL_ATTR_REF: AttrRef = AttrRef::nil();

const DEPS_CAPACITY: usize = 30;

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
        let AttrInner::Init { attr_ref, .. } = &self.inner else {
            return &NIL_ATTR_REF;
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
    #[inline(always)]
    pub fn query<'a, S: SyncBuildHasher>(
        &self,
        reader: &mut AttrContext<'a, C::Node, S>,
    ) -> AnalysisResult<AttrReadGuard<'a, C, S>> {
        let attr_ref = self.as_ref();

        if attr_ref.is_nil() {
            return Err(AnalysisError::UninitAttribute);
        }

        // Safety: Attributes data came from the C::compute function.
        unsafe { attr_ref.fetch::<false, C, S>(reader) }
    }
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

    #[inline(always)]
    pub fn query<'a, C: Computable, S: SyncBuildHasher>(
        &self,
        reader: &mut AttrContext<'a, C::Node, S>,
    ) -> AnalysisResult<AttrReadGuard<'a, C, S>> {
        // Safety: `CHECK` set to true
        unsafe { self.fetch::<true, C, S>(reader) }
    }

    pub fn invalidate<N: Grammar, S: SyncBuildHasher>(&self, task: &mut MutationTask<N, S>) {
        let Some(records) = task.analyzer().database.records.get(self.id) else {
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
        task.analyzer().database.commit();
    }

    #[inline(always)]
    pub fn is_valid_ref<N: Grammar, S: SyncBuildHasher>(&self, task: &AttrContext<N, S>) -> bool {
        let Some(records) = task.analyzer().database.records.get(self.id) else {
            return false;
        };

        records.contains(&self.entry)
    }
}

pub struct AttrContext<'a, N: Grammar, S: SyncBuildHasher> {
    analyzer: &'a Analyzer<N, S>,
    revision: Revision,
    handle: &'a Latch,
    node_ref: &'a NodeRef,
    deps: HashSet<AttrRef, S>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> AttrContext<'a, N, S> {
    #[inline(always)]
    pub(super) fn for_analysis_task(task: &AnalysisTask<'a, N, S>) -> Self {
        Self {
            analyzer: task.analyzer(),
            revision: task.db_revision(),
            handle: task.handle(),
            node_ref: &NIL_NODE_REF,
            deps: HashSet::with_capacity_and_hasher(1, S::default()),
        }
    }

    #[inline(always)]
    pub(super) fn for_mutation_task(task: &MutationTask<'a, N, S>) -> Self {
        static DUMMY: Lazy<Latch> = Lazy::new(Latch::new);

        #[cfg(debug_assertions)]
        if DUMMY.get_relaxed() {
            system_panic!("Dummy handle cancelled");
        }

        Self {
            analyzer: task.analyzer(),
            revision: task.analyzer().database.revision(),
            handle: &DUMMY,
            node_ref: &NIL_NODE_REF,
            deps: HashSet::with_capacity_and_hasher(1, S::default()),
        }
    }

    #[inline(always)]
    pub fn node_ref(&self) -> &'a NodeRef {
        self.node_ref
    }

    #[inline(always)]
    pub fn analyzer(&self) -> &'a Analyzer<N, S> {
        self.analyzer
    }

    #[inline(always)]
    pub fn proceed(&self) -> AnalysisResult<()> {
        if self.handle().get_relaxed() {
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
            deps: HashSet::with_capacity_and_hasher(DEPS_CAPACITY, S::default()),
        }
    }

    #[inline(always)]
    pub(super) fn handle(&self) -> &'a Latch {
        self.handle
    }

    #[inline(always)]
    pub(super) fn db_revision(&self) -> Revision {
        self.revision
    }

    #[inline(always)]
    pub(super) fn track(&mut self, dep: &AttrRef) {
        let _ = self.deps.insert(*dep);
    }

    #[inline(always)]
    pub(super) fn reset_deps(&mut self) {
        self.deps.clear();
    }

    #[inline(always)]
    pub(super) fn into_deps(self) -> HashSet<AttrRef, S> {
        self.deps
    }
}

pub trait Computable: Send + Sync + 'static {
    type Node: Grammar;

    fn compute<S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, S>,
    ) -> AnalysisResult<Self>
    where
        Self: Sized;
}

// Safety: Entries order reflects guards drop semantics.
#[allow(dead_code)]
pub struct AttrReadGuard<'a, C: Computable, S: SyncBuildHasher = RandomState> {
    pub(super) data: &'a C,
    pub(super) cell_guard: RwLockReadGuard<'a, Cell<<C as Computable>::Node, S>>,
    pub(super) records_guard: UnitTableReadGuard<'a, Repository<Record<C::Node, S>>, S>,
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
    pub fn attr_revision(&self) -> Revision {
        let Some(cache) = &self.cell_guard.cache else {
            unsafe { debug_unreachable!("AttrReadGuard without cache.") }
        };

        cache.updated_at
    }
}

enum AttrInner {
    Uninit(NodeRef),

    Init {
        attr_ref: AttrRef,
        database: Weak<dyn AbstractDatabase>,
    },
}
