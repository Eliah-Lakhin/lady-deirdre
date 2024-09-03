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
    fmt::{Debug, Formatter},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::{
    analysis::{
        database::{AttrMemo, AttrRecord, AttrRecordCache, CacheDeps},
        AnalysisError,
        AnalysisResult,
        Attr,
        AttrContext,
        AttrRef,
        Computable,
        Grammar,
        Revision,
        TaskHandle,
    },
    arena::Repo,
    report::ld_unreachable,
    sync::{Shared, SyncBuildHasher},
    syntax::NodeRef,
    units::Document,
};

/// A type alias of the `Attr<Scope<N>>` type that denotes a built-in
/// attribute that resolves to the scope node of the current node.
///
/// See [Scope] for details.
pub type ScopeAttr<N> = Attr<Scope<N>>;

/// A [computable](Computable) object that infers the scope of the current node.
///
/// Scopes are the syntax tree nodes for which the [Grammar::is_scope]
/// function returns true.
///
/// The scope of the current node is the closest ancestor of the current
/// node which is a scope. If there is no such ancestor, the scope
/// of the current is the root node of the syntax tree.
///
/// Scope attributes are special built-in attributes.
///
/// You don't have to specify them explicitly in
/// the [Feature](crate::analysis::Feature) objects that build up the node
/// semantics. Instead, every [Semantics](crate::analysis::Semantics) object
/// has a built-in scope attribute instance, and
/// the [Analyzer](crate::analysis::Analyzer) is responsible to keep its value
/// up to date.
pub struct Scope<N: Grammar> {
    /// A [NodeRef] reference to the syntax tree node that represents a scope
    /// node of the current node.
    pub scope_ref: NodeRef,
    _grammar: PhantomData<N>,
}

impl<N: Grammar> PartialEq for Scope<N> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.scope_ref.eq(&other.scope_ref)
    }
}

impl<N: Grammar> Eq for Scope<N> {}

impl<N: Grammar> Debug for Scope<N> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Scope")
            .field("scope_ref", &self.scope_ref)
            .finish()
    }
}

impl<N: Grammar> Clone for Scope<N> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Grammar> Copy for Scope<N> {}

impl<N: Grammar> Default for Scope<N> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            scope_ref: NodeRef::nil(),
            _grammar: PhantomData,
        }
    }
}

impl<N: Grammar> Computable for Scope<N> {
    type Node = N;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self>
    where
        Self: Sized,
    {
        let node_ref = context.node_ref();
        let doc_read = context.read_doc(node_ref.id)?;

        let Some(node) = node_ref.deref(doc_read.deref()) else {
            return Ok(Self::default());
        };

        let parent_ref = node.parent_ref();

        let Some(parent) = parent_ref.deref(doc_read.deref()) else {
            return Ok(Self::default());
        };

        if parent.is_scope() {
            return Ok(Self {
                scope_ref: parent_ref,
                _grammar: PhantomData,
            });
        }

        let parent_scope_attr = parent.scope_attr()?;

        Ok(*parent_scope_attr.read(context)?.deref())
    }
}

impl<N: Grammar> ScopeAttr<N> {
    // Safety: `attr_ref` refers ScopeAttr.
    pub(super) unsafe fn snapshot_manually<H: TaskHandle, S: SyncBuildHasher>(
        attr_ref: &AttrRef,
        handle: &H,
        doc: &Document<N>,
        records: &Repo<AttrRecord<N, H, S>>,
        revision: Revision,
    ) -> AnalysisResult<NodeRef> {
        let Some(record) = records.get(&attr_ref.entry) else {
            return Err(AnalysisError::UninitAttribute);
        };

        loop {
            let record_read_guard = record.read(&Duration::ZERO)?;

            if record_read_guard.verified_at >= revision {
                if let Some(cache) = &record_read_guard.cache {
                    // Safety: Upheld by the caller.
                    let scope = unsafe { cache.downcast_unchecked::<Scope<N>>() };

                    return Ok(scope.scope_ref);
                }

                // Records with `verified_at > 0` always have cache.
                unsafe { ld_unreachable!("Verified scope attribute without cache.") };
            }

            drop(record_read_guard);

            let mut record_write_guard = record.write(&Duration::ZERO)?;

            let record_data = record_write_guard.deref_mut();

            let Some(cache) = &mut record_data.cache else {
                let (dep, scope_ref) =
                    Self::compute_manually(&record_data.node_ref, handle, doc, records, revision)?;

                let mut deps = CacheDeps::default();

                if let Some(dep) = dep {
                    let _ = deps.attrs.insert(dep);
                }

                record_data.cache = Some(AttrRecordCache {
                    dirty: false,
                    updated_at: revision,
                    memo: Box::new(Scope {
                        scope_ref,
                        _grammar: PhantomData::<N>,
                    }) as Box<dyn AttrMemo>,
                    deps: Shared::new(deps),
                });

                record_data.verified_at = revision;

                return Ok(scope_ref);
            };

            if record_data.verified_at >= revision {
                // Safety: Upheld by the caller.
                let scope = unsafe { cache.downcast_unchecked::<Scope<N>>() };

                return Ok(scope.scope_ref);
            }

            if !cache.dirty {
                if let Some(attr_ref) = cache.deps.as_ref().attrs.iter().next() {
                    let mut deps_verified = true;

                    loop {
                        let Some(dep_record) = records.get(&attr_ref.entry) else {
                            cache.dirty = true;
                            break;
                        };

                        let dep_record_read_guard = dep_record.read(&Duration::ZERO)?;

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
                            deps_verified && dep_record_read_guard.verified_at >= revision;

                        break;
                    }

                    if !cache.dirty {
                        if deps_verified {
                            record_data.verified_at = revision;

                            // Safety: Upheld by the caller.
                            let scope = unsafe { cache.downcast_unchecked::<Scope<N>>() };

                            return Ok(scope.scope_ref);
                        }

                        let attr_ref = *attr_ref;

                        drop(record_write_guard);

                        // Safety: ScopeAttr can only depend on another ScopeAttr.
                        let _ = unsafe {
                            Self::snapshot_manually(&attr_ref, handle, doc, records, revision)
                        };

                        continue;
                    }
                }
            }

            if !cache.dirty {
                record_data.verified_at = revision;

                // Safety: Upheld by the caller.
                let scope = unsafe { cache.downcast_unchecked::<Scope<N>>() };

                return Ok(scope.scope_ref);
            }

            let (dep, scope_ref) =
                Self::compute_manually(&record_data.node_ref, handle, doc, records, revision)?;

            // Safety: Upheld by the caller.
            let old_scope = unsafe { cache.downcast_unchecked_mut::<Scope<N>>() };

            if &old_scope.scope_ref != &scope_ref {
                old_scope.scope_ref = scope_ref;
                cache.updated_at = revision;
            }

            let mut deps = CacheDeps::default();

            if let Some(dep) = dep {
                let _ = deps.attrs.insert(dep);
            }

            cache.deps = Shared::new(deps);

            record_data.verified_at = revision;

            return Ok(scope_ref);
        }
    }

    #[inline]
    fn compute_manually<H: TaskHandle, S: SyncBuildHasher>(
        node_ref: &NodeRef,
        handle: &H,
        doc: &Document<N>,
        records: &Repo<AttrRecord<N, H, S>>,
        revision: Revision,
    ) -> AnalysisResult<(Option<AttrRef>, NodeRef)> {
        let Some(node) = node_ref.deref(doc) else {
            return Ok((None, NodeRef::default()));
        };

        let parent_ref = node.parent_ref();

        let Some(parent) = parent_ref.deref(doc) else {
            return Ok((None, NodeRef::default()));
        };

        if parent.is_scope() {
            return Ok((None, parent_ref));
        }

        let parent_scope_attr = parent.scope_attr()?;
        let parent_scope_attr_ref = parent_scope_attr.as_ref();

        // Safety: `parent_scope_attr_ref` belongs to `parent_scope_attr`.
        let scope_ref = unsafe {
            Self::snapshot_manually(parent_scope_attr_ref, handle, doc, records, revision)?
        };

        Ok((Some(*parent_scope_attr.as_ref()), scope_ref))
    }
}
