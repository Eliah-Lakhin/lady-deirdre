////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{
    cmp::Ordering,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
    sync::Weak,
};

use crate::{
    analysis::{
        database::AbstractDatabase,
        AbstractFeature,
        AbstractTask,
        AnalysisError,
        AnalysisResult,
        AttrContext,
        AttrRef,
        Feature,
        Grammar,
        Initializer,
        Invalidator,
        MutationAccess,
        Revision,
        SemanticAccess,
        SlotReadGuard,
        TaskHandle,
        NIL_ATTR_REF,
    },
    arena::{Entry, Id, Identifiable},
    sync::SyncBuildHasher,
    syntax::{Key, NodeRef},
};

pub static NIL_SLOT_REF: SlotRef = SlotRef::nil();

#[repr(transparent)]
pub struct Slot<N: Grammar, T: Default + Send + Sync + 'static> {
    inner: SlotInner,
    _node: PhantomData<N>,
    _data: PhantomData<T>,
}

impl<N, T> Debug for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        let slot_ref = self.as_ref();

        match (slot_ref.id.is_nil(), slot_ref.entry.is_nil()) {
            (false, _) => formatter.write_fmt(format_args!(
                "Slot(id: {:?}, entry: {:?})",
                slot_ref.id, slot_ref.entry,
            )),

            (true, false) => formatter.write_fmt(format_args!("Slot(entry: {:?})", slot_ref.entry)),

            (true, true) => formatter.write_str("Slot(Nil)"),
        }
    }
}

impl<N: Grammar, T: Default + Send + Sync + 'static> Identifiable for Slot<N, T> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.as_ref().id
    }
}

impl<N, T, U> PartialEq<Slot<N, U>> for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
    U: Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn eq(&self, other: &Slot<N, U>) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl<N, T> Eq for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
}

impl<N, T, U> PartialOrd<Slot<N, U>> for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
    U: Default + Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn partial_cmp(&self, other: &Slot<N, U>) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<N, T> Ord for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn cmp(&self, other: &Slot<N, T>) -> Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<N, T> Hash for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<N, T> AsRef<SlotRef> for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn as_ref(&self) -> &SlotRef {
        let SlotInner::Init { attr_ref, .. } = &self.inner else {
            return &NIL_SLOT_REF;
        };

        attr_ref
    }
}

impl<N, T> Drop for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    fn drop(&mut self) {
        let SlotInner::Init { attr_ref, database } = &self.inner else {
            return;
        };

        let Some(database) = database.upgrade() else {
            return;
        };

        database.deregister_attribute(attr_ref.id, &attr_ref.entry);
    }
}

impl<N, T> AbstractFeature for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    #[inline(always)]
    fn attr_ref(&self) -> &AttrRef {
        &NIL_ATTR_REF
    }

    #[inline(always)]
    fn slot_ref(&self) -> &SlotRef {
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

impl<N, T> Feature for Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    type Node = N;

    #[inline(always)]
    fn new(node_ref: NodeRef) -> Self {
        Self {
            inner: SlotInner::Uninit(node_ref),
            _node: PhantomData,
            _data: PhantomData,
        }
    }

    fn init<H: TaskHandle, S: SyncBuildHasher>(
        &mut self,
        initializer: &mut Initializer<Self::Node, H, S>,
    ) {
        let SlotInner::Uninit(node_ref) = &self.inner else {
            return;
        };

        let id = node_ref.id;

        #[cfg(debug_assertions)]
        if initializer.id() != id {
            panic!("Slot and Compilation Unit mismatch.");
        }

        let (database, entry) = initializer.register_slot::<T>();

        self.inner = SlotInner::Init {
            attr_ref: SlotRef { id, entry },
            database,
        };
    }

    #[inline(always)]
    fn invalidate<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        _invalidator: &mut Invalidator<Self::Node, H, S>,
    ) {
    }
}

impl<N, T> Slot<N, T>
where
    N: Grammar,
    T: Default + Send + Sync + 'static,
{
    #[inline(always)]
    pub fn snapshot<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        task: &impl SemanticAccess<N, H, S>,
    ) -> AnalysisResult<(Revision, T)>
    where
        T: Clone,
    {
        let mut reader = AttrContext::new(task.analyzer(), task.revision(), task.handle());

        let result = self.read::<H, S>(&mut reader)?;
        let revision = result.revision;
        let data = result.deref().clone();

        Ok((revision, data))
    }

    #[inline(always)]
    pub fn mutate<H: TaskHandle, S: SyncBuildHasher>(
        &self,
        task: &impl MutationAccess<N, H, S>,
        map: impl FnOnce(&mut T) -> bool,
    ) -> AnalysisResult<()> {
        let slot_ref = self.as_ref();

        if slot_ref.is_nil() {
            return Err(AnalysisError::UninitSlot);
        }

        // Safety: Slot initialized with `T` default value.
        unsafe { slot_ref.change::<false, T, N, H, S>(task, map) }
    }

    #[inline(always)]
    pub fn read<'a, H: TaskHandle, S: SyncBuildHasher>(
        &self,
        reader: &mut AttrContext<'a, N, H, S>,
    ) -> AnalysisResult<SlotReadGuard<'a, T, N, H, S>> {
        let slot_ref = self.as_ref();

        if slot_ref.is_nil() {
            return Err(AnalysisError::UninitSlot);
        }

        // Safety: Slot initialized with `T` default value.
        unsafe { slot_ref.fetch::<true, T, N, H, S>(reader) }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotRef {
    pub id: Id,
    pub entry: Entry,
}

impl Debug for SlotRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        match (self.id.is_nil(), self.entry.is_nil()) {
            (false, _) => formatter.write_fmt(format_args!(
                "SlotRef(id: {:?}, entry: {:?})",
                self.id, self.entry,
            )),

            (true, false) => formatter.write_fmt(format_args!("SlotRef(entry: {:?})", self.entry)),

            (true, true) => formatter.write_str("SlotRef(Nil)"),
        }
    }
}

impl Identifiable for SlotRef {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl Default for SlotRef {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl SlotRef {
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            entry: Entry::nil(),
        }
    }

    #[inline(always)]
    pub const fn is_nil(&self) -> bool {
        self.id.is_nil() && self.entry.is_nil()
    }

    #[inline(always)]
    pub fn snapshot<T, N, H, S>(
        &self,
        task: &impl SemanticAccess<N, H, S>,
    ) -> AnalysisResult<(Revision, T)>
    where
        T: Clone + Default + Send + Sync + 'static,
        N: Grammar,
        H: TaskHandle,
        S: SyncBuildHasher,
    {
        let mut reader = AttrContext::new(task.analyzer(), task.revision(), task.handle());

        let result = self.read::<T, N, H, S>(&mut reader)?;
        let revision = result.revision;
        let data = result.deref().clone();

        Ok((revision, data))
    }

    #[inline(always)]
    pub fn mutate<T, N, H, S>(
        &self,
        task: &impl MutationAccess<N, H, S>,
        map: impl FnOnce(&mut T) -> bool,
    ) -> AnalysisResult<()>
    where
        T: Default + Send + Sync + 'static,
        N: Grammar,
        H: TaskHandle,
        S: SyncBuildHasher,
    {
        // Safety: `CHECK` set to true
        unsafe { self.change::<true, T, N, H, S>(task, map) }
    }

    #[inline(always)]
    pub fn read<'a, T, N, H, S>(
        &self,
        reader: &mut AttrContext<'a, N, H, S>,
    ) -> AnalysisResult<SlotReadGuard<'a, T, N, H, S>>
    where
        T: Default + Send + Sync + 'static,
        N: Grammar,
        H: TaskHandle,
        S: SyncBuildHasher,
    {
        // Safety: `CHECK` set to true
        unsafe { self.fetch::<true, T, N, H, S>(reader) }
    }

    #[inline(always)]
    pub fn is_valid_ref<N: Grammar, H: TaskHandle, S: SyncBuildHasher>(
        &self,
        task: &mut impl AbstractTask<N, H, S>,
    ) -> bool {
        let Some(records) = task.analyzer().db.records.get(&self.id) else {
            return false;
        };

        records.slots.contains(&self.entry)
    }
}

enum SlotInner {
    Uninit(NodeRef),

    Init {
        attr_ref: SlotRef,
        database: Weak<dyn AbstractDatabase>,
    },
}
