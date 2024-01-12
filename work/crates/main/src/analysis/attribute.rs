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
        tasks::TaskSealed,
        AbstractFeature,
        AnalysisError,
        AnalysisResult,
        AttrContext,
        AttrReadGuard,
        Computable,
        Feature,
        FeatureInitializer,
        FeatureInvalidator,
        Grammar,
        MutationTask,
        Revision,
        ScopeAttr,
        SemanticAccess,
    },
    arena::{Entry, Id, Identifiable},
    std::*,
    sync::SyncBuildHasher,
    syntax::{Key, NodeRef},
};

pub static NIL_ATTR_REF: AttrRef = AttrRef::nil();

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
    pub fn snapshot<S: SyncBuildHasher>(
        &self,
        task: &impl SemanticAccess<C::Node, S>,
    ) -> AnalysisResult<(Revision, C)>
    where
        C: Clone,
    {
        let mut reader = AttrContext::new(task.analyzer(), task.revision(), task.handle());

        let result = self.read(&mut reader)?;
        let revision = result.attr_revision();
        let data = result.deref().clone();

        Ok((revision, data))
    }

    #[inline(always)]
    pub fn read<'a, S: SyncBuildHasher>(
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
            entry: Entry::nil(),
        }
    }

    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.id.is_nil() || self.entry.is_nil()
    }

    #[inline(always)]
    pub fn snapshot<C: Computable + Clone, S: SyncBuildHasher>(
        &self,
        task: &impl SemanticAccess<C::Node, S>,
    ) -> AnalysisResult<(Revision, C)> {
        let mut reader = AttrContext::new(task.analyzer(), task.revision(), task.handle());

        let result = self.read::<C, S>(&mut reader)?;
        let revision = result.attr_revision();
        let data = result.deref().clone();

        Ok((revision, data))
    }

    #[inline(always)]
    pub fn read<'a, C: Computable, S: SyncBuildHasher>(
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

enum AttrInner {
    Uninit(NodeRef),

    Init {
        attr_ref: AttrRef,
        database: Weak<dyn AbstractDatabase>,
    },
}
