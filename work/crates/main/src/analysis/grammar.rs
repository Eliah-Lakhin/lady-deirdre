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

extern crate lady_deirdre_derive;

pub use lady_deirdre_derive::Feature;

use crate::{
    analysis::{
        database::{AbstractDatabase, Record},
        AnalysisError,
        AnalysisResult,
        AttrRef,
        Computable,
        ScopeAttr,
        NIL_ATTR_REF,
    },
    arena::{Entry, Id, Identifiable, Repo},
    std::*,
    sync::SyncBuildHasher,
    syntax::{Key, Node, NodeRef},
    units::Document,
};

pub trait Grammar: Node + AbstractFeature {
    type Classifier: Classifier<Node = Self>;

    fn init<S: SyncBuildHasher>(&mut self, initializer: &mut Initializer<Self, S>);

    fn invalidate<S: SyncBuildHasher>(&self, invalidator: &mut Invalidator<Self, S>);

    fn scope_attr(&self) -> AnalysisResult<&ScopeAttr<Self>>;

    fn is_scope(&self) -> bool;
}

pub trait Classifier {
    type Node: Node;
    type Class: Clone + Eq + Hash + Send + Sync;

    fn classify<S: SyncBuildHasher>(
        doc: &Document<Self::Node>,
        node_ref: &NodeRef,
    ) -> HashSet<Self::Class, S>;
}

pub struct VoidClassifier<N: Node>(PhantomData<N>);

impl<N: Node> Classifier for VoidClassifier<N> {
    type Node = N;
    type Class = ();

    #[inline(always)]
    fn classify<S: SyncBuildHasher>(
        _doc: &Document<Self::Node>,
        _node_ref: &NodeRef,
    ) -> HashSet<Self::Class, S> {
        HashSet::default()
    }
}

pub trait Feature: AbstractFeature {
    type Node: Grammar;

    fn new(node_ref: NodeRef) -> Self
    where
        Self: Sized;

    fn init<S: SyncBuildHasher>(&mut self, initializer: &mut Initializer<Self::Node, S>);

    fn invalidate<S: SyncBuildHasher>(&self, invalidator: &mut Invalidator<Self::Node, S>);
}

pub trait AbstractFeature {
    fn attr_ref(&self) -> &AttrRef;

    fn feature(&self, key: Key) -> AnalysisResult<&dyn AbstractFeature>;

    fn feature_keys(&self) -> &'static [&'static Key];
}

pub struct Initializer<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    pub(super) id: Id,
    pub(super) database: Weak<dyn AbstractDatabase>,
    pub(super) records: &'a mut Repo<Record<N, S>>,
}

pub struct VoidFeature<N: Grammar>(PhantomData<N>);

impl<N: Grammar> Default for VoidFeature<N> {
    #[inline(always)]
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<N: Grammar> AbstractFeature for VoidFeature<N> {
    #[inline(always)]
    fn attr_ref(&self) -> &AttrRef {
        &NIL_ATTR_REF
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

impl<N: Grammar> Feature for VoidFeature<N> {
    type Node = N;

    #[inline(always)]
    fn new(_node_ref: NodeRef) -> Self {
        Self::default()
    }

    #[inline(always)]
    fn init<S: SyncBuildHasher>(&mut self, _initializer: &mut Initializer<Self::Node, S>) {}

    #[inline(always)]
    fn invalidate<S: SyncBuildHasher>(&self, _invalidator: &mut Invalidator<Self::Node, S>) {}
}

impl<'a, N: Grammar, S: SyncBuildHasher> Identifiable for Initializer<'a, N, S> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Initializer<'a, N, S> {
    #[inline(always)]
    pub(super) fn register_attribute<C: Computable<Node = N> + Eq>(
        &mut self,
        node_ref: NodeRef,
    ) -> (Weak<dyn AbstractDatabase>, Entry) {
        (
            self.database.clone(),
            self.records.insert(Record::new::<C>(node_ref)),
        )
    }
}

pub struct Invalidator<'a, N: Grammar, S: SyncBuildHasher = RandomState> {
    pub(super) id: Id,
    pub(super) records: &'a mut Repo<Record<N, S>>,
}

impl<'a, N: Grammar, S: SyncBuildHasher> Identifiable for Invalidator<'a, N, S> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'a, N: Grammar, S: SyncBuildHasher> Invalidator<'a, N, S> {
    #[inline(always)]
    pub(super) fn invalidate_attribute(&mut self, entry: &Entry) {
        let Some(record) = self.records.get(entry) else {
            return;
        };

        record.invalidate();
    }
}

pub struct Semantics<F: Feature> {
    inner: Box<SemanticsInner<F>>,
}

impl<F: Feature> AbstractFeature for Semantics<F> {
    #[inline(always)]
    fn attr_ref(&self) -> &AttrRef {
        let Ok(inner) = self.get() else {
            return &NIL_ATTR_REF;
        };

        inner.attr_ref()
    }

    #[inline(always)]
    fn feature(&self, key: Key) -> AnalysisResult<&dyn AbstractFeature> {
        self.get()?.feature(key)
    }

    #[inline(always)]
    fn feature_keys(&self) -> &'static [&'static Key] {
        let Ok(inner) = self.get() else {
            return &[];
        };

        inner.feature_keys()
    }
}

impl<F: Feature> Feature for Semantics<F> {
    type Node = F::Node;

    fn new(node_ref: NodeRef) -> Self {
        Self {
            inner: Box::new(SemanticsInner::Uninit(node_ref)),
        }
    }

    fn init<S: SyncBuildHasher>(&mut self, initializer: &mut Initializer<Self::Node, S>) {
        let SemanticsInner::Uninit(node_ref) = self.inner.deref() else {
            return;
        };

        let node_ref = *node_ref;

        let mut feature = F::new(node_ref);
        let mut scope_attr = ScopeAttr::new(node_ref);

        feature.init(initializer);
        scope_attr.init(initializer);

        *self.inner = SemanticsInner::Init {
            feature,
            scope_attr,
        };
    }

    fn invalidate<S: SyncBuildHasher>(&self, invalidator: &mut Invalidator<Self::Node, S>) {
        let SemanticsInner::Init { feature, .. } = self.inner.deref() else {
            return;
        };

        feature.invalidate(invalidator);
    }
}

impl<F: Feature> Semantics<F> {
    #[inline(always)]
    pub fn get(&self) -> AnalysisResult<&F> {
        let SemanticsInner::Init { feature, .. } = self.inner.deref() else {
            return Err(AnalysisError::UninitSemantics);
        };

        Ok(feature)
    }

    #[inline(always)]
    pub fn scope_attr(&self) -> AnalysisResult<&ScopeAttr<F::Node>> {
        let SemanticsInner::Init { scope_attr, .. } = self.inner.deref() else {
            return Err(AnalysisError::UninitSemantics);
        };

        Ok(scope_attr)
    }
}

enum SemanticsInner<F: Feature> {
    Uninit(NodeRef),
    Init {
        feature: F,
        scope_attr: ScopeAttr<F::Node>,
    },
}
