////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::{collections::HashSet, hash::RandomState};

use crate::{
    analysis::{
        manager::TaskId,
        AnalysisError,
        AnalysisResult,
        Analyzer,
        Classifier,
        DocumentReadGuard,
        Event,
        Grammar,
        Revision,
        TaskHandle,
        TriggerHandle,
    },
    arena::Id,
    lexis::{ToSpan, TokenBuffer},
    sync::{Shared, SyncBuildHasher},
    syntax::NodeRef,
    units::{CompilationUnit, Document},
};

/// A task that grants access to the semantic features of the [Analyzer].
///
/// This kind of task implements a [SemanticAccess] trait through which you can
/// read particular [attribute](crate::analysis::Attr) values, but you cannot
/// change the content of the documents.
///
/// You may have as many instances of this task as needed at the same time, and
/// you can use them from multiple threads to read the attributes. The analyzer
/// is capable to manage the semantic graph concurrently.
pub struct AnalysisTask<
    'a,
    N: Grammar,
    H: TaskHandle = TriggerHandle,
    S: SyncBuildHasher = RandomState,
> {
    id: TaskId,
    analyzer: &'a Analyzer<N, H, S>,
    revision: Revision,
    handle: &'a H,
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> SemanticAccess<N, H, S>
    for AnalysisTask<'a, N, H, S>
{
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> AbstractTask<N, H, S>
    for AnalysisTask<'a, N, H, S>
{
    #[inline(always)]
    fn handle(&self) -> &H {
        self.handle
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> TaskSealed<N, H, S>
    for AnalysisTask<'a, N, H, S>
{
    #[inline(always)]
    fn analyzer(&self) -> &Analyzer<N, H, S> {
        self.analyzer
    }

    #[inline(always)]
    fn revision(&self) -> Revision {
        self.revision
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> Drop for AnalysisTask<'a, N, H, S> {
    fn drop(&mut self) {
        self.analyzer.tasks.release_task(self.id);
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> AnalysisTask<'a, N, H, S> {
    #[inline(always)]
    pub(super) fn new(id: TaskId, analyzer: &'a Analyzer<N, H, S>, handle: &'a H) -> Self {
        Self {
            id,
            analyzer,
            revision: analyzer.db.load_revision(),
            handle,
        }
    }
}

/// A task that grants access to change the content of the documents managed by
/// the [Analyzer].
///
/// This kind of task implements a [MutationAccess] trait through which you can
/// create, delete, or edit the existing documents' content, but you cannot
/// read [attributes](crate::analysis::Attr) of the semantic graph.
///
/// You may have as many instances of this task as needed at the same time, and
/// you can use them from multiple threads to manage distinct documents.
///
/// The analyzer allows you to edit independent documents concurrently, but
/// if two independent threads would edit the same document, one of them will
/// block until another one finishes its job.
pub struct MutationTask<
    'a,
    N: Grammar,
    H: TaskHandle = TriggerHandle,
    S: SyncBuildHasher = RandomState,
> {
    id: TaskId,
    analyzer: &'a Analyzer<N, H, S>,
    handle: &'a H,
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> MutationAccess<N, H, S>
    for MutationTask<'a, N, H, S>
{
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> AbstractTask<N, H, S>
    for MutationTask<'a, N, H, S>
{
    #[inline(always)]
    fn handle(&self) -> &H {
        self.handle
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> TaskSealed<N, H, S>
    for MutationTask<'a, N, H, S>
{
    #[inline(always)]
    fn analyzer(&self) -> &Analyzer<N, H, S> {
        self.analyzer
    }

    #[inline(always)]
    fn revision(&self) -> Revision {
        self.analyzer.db.load_revision()
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> Drop for MutationTask<'a, N, H, S> {
    fn drop(&mut self) {
        self.analyzer.tasks.release_task(self.id);
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> MutationTask<'a, N, H, S> {
    #[inline(always)]
    pub(super) fn new(id: TaskId, analyzer: &'a Analyzer<N, H, S>, handle: &'a H) -> Self {
        Self {
            id,
            analyzer,
            handle,
        }
    }
}

/// An exclusive task that grants access to change documents and to observe
/// the semantic graph of the [Analyzer].
///
/// This task implements a [MutationAccess] trait through which you can
/// create, delete, or edit the existing documents' content, and implements
/// a [SemanticAccess] trait through which you can read particular
/// [attribute](crate::analysis::Attr) values.
///
/// You can request both kinds of operations sequentially in a single thread,
/// but the Analyzer does not allow you to have more than one active Exclusive
/// task, and exclusive access is granted if and only if no other types
/// of active tasks are granted.
pub struct ExclusiveTask<
    'a,
    N: Grammar,
    H: TaskHandle = TriggerHandle,
    S: SyncBuildHasher = RandomState,
> {
    id: TaskId,
    analyzer: &'a Analyzer<N, H, S>,
    handle: &'a H,
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> SemanticAccess<N, H, S>
    for ExclusiveTask<'a, N, H, S>
{
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> MutationAccess<N, H, S>
    for ExclusiveTask<'a, N, H, S>
{
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> AbstractTask<N, H, S>
    for ExclusiveTask<'a, N, H, S>
{
    #[inline(always)]
    fn handle(&self) -> &H {
        self.handle
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> TaskSealed<N, H, S>
    for ExclusiveTask<'a, N, H, S>
{
    #[inline(always)]
    fn analyzer(&self) -> &Analyzer<N, H, S> {
        self.analyzer
    }

    #[inline(always)]
    fn revision(&self) -> Revision {
        self.analyzer.db.load_revision()
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> Drop for ExclusiveTask<'a, N, H, S> {
    fn drop(&mut self) {
        self.analyzer.tasks.release_task(self.id);
    }
}

impl<'a, N: Grammar, H: TaskHandle, S: SyncBuildHasher> ExclusiveTask<'a, N, H, S> {
    #[inline(always)]
    pub(super) fn new(id: TaskId, analyzer: &'a Analyzer<N, H, S>, handle: &'a H) -> Self {
        Self {
            id,
            analyzer,
            handle,
        }
    }
}

/// A trait that provides documents' mutation operations for the [MutationTask]
/// and the [ExclusiveTask].
///
/// This trait is sealed and cannot be implemented outside of this crate.
///
/// The MutationAccess trait is a subtrait of the [AbstractTask] trait that
/// provides general access to the [Analyzer]'s content.
pub trait MutationAccess<N: Grammar, H: TaskHandle, S: SyncBuildHasher>:
    AbstractTask<N, H, S>
{
    /// Creates a mutable [Document] inside the [Analyzer].
    ///
    /// This type of documents supports [write](Self::write_to_doc) operations.
    ///
    /// Returns a unique [identifier](Id) of the created document.
    ///
    /// The parameter could be a [TokenBuffer] or just an arbitrary string.
    #[inline(always)]
    fn add_mutable_doc(&mut self, text: impl Into<TokenBuffer<N::Token>>) -> Id {
        self.analyzer().register_doc(Document::new_mutable(text))
    }

    /// Creates an immutable [Document] inside the [Analyzer].
    ///
    /// This type of documents does not support [write](Self::write_to_doc)
    /// operations.
    ///
    /// Returns a unique [identifier](Id) of the created document.
    ///
    /// The parameter could be a [TokenBuffer] or just an arbitrary string.
    #[inline(always)]
    fn add_immutable_doc(&mut self, text: impl Into<TokenBuffer<N::Token>>) -> Id {
        self.analyzer().register_doc(Document::new_immutable(text))
    }

    /// Writes user-input edit into the document managed by the [Analyzer].
    ///
    /// The `id` parameter specifies the document's [identifier](Id). If there is
    /// no corresponding document managed by the analyzer
    /// (e.g., if the document has been [removed](Self::remove_doc) or was not
    /// created), the function returns
    /// a [MissingDocument](AnalysisError::MissingDocument) error.
    ///
    /// If the document exists but is not [mutable](Self::add_mutable_doc),
    /// the function returns
    /// an [ImmutableDocument](AnalysisError::ImmutableDocument) error.
    ///
    /// The `span` parameter specifies a [span](ToSpan) of the text that needs
    /// to be rewritten (e.g., an absolute chars range `10..30`, the entire text
    /// cover `..`, or a single site inside the text `10..10`, or any other
    /// type of span). If the span parameter is not
    /// [valid](ToSpan::is_valid_span) for this document, the function returns
    /// [InvalidSpan](AnalysisError::InvalidSpan) error.
    ///
    /// The `text` parameter is a string to be written in place of the spanned
    /// source code text.
    ///
    /// This function instantly reparses a part of the underlying source code
    /// relative to the edit, and it invalidates corresponding parts of the
    /// analyzer's semantic graph, but it does not recompute invalid graph
    /// [attributes](crate::analysis::Attr). The corresponding graph attributes
    /// will be recomputed on demand later on when you try to read them
    /// directly or indirectly through other related attributes.
    ///
    /// The reparsing process and the semantic graph invalidation usually take
    /// a short time if the edit is short, and even if the entire source code is
    /// big. Therefore, it is acceptable to call this function on every
    /// user-input action. For instance, you can call this function on every
    /// content change event from the text editor.
    #[inline(always)]
    fn write_to_doc(
        &mut self,
        id: Id,
        span: impl ToSpan,
        text: impl AsRef<str>,
    ) -> AnalysisResult<()> {
        self.analyzer().write_to_doc(self.handle(), id, span, text)
    }

    /// Removes a document managed by the [Analyzer].
    ///
    /// The `id` parameter specifies the document's [identifier](Id).
    ///
    /// If the document exists in the analyzer, the function returns true,
    /// indicating that the document was successfully removed.
    /// Otherwise, the function returns false.
    #[inline(always)]
    fn remove_doc(&mut self, id: Id) -> bool {
        self.analyzer().remove_doc(id)
    }

    /// Invalidates semantic graph [attributes](crate::analysis::Attr)
    /// associated with the corresponding `event` and the `id` parameters.
    ///
    /// This function will invalidate the attributes currently
    /// [subscribed](crate::analysis::AttrContext::subscribe) on this event
    /// with the [nil](Id::nil) identifier (passed to the subscribe function).
    ///
    /// Additionally, if the `id` parameter of this function is not
    /// [nil](Id::nil), the function will invalidate attributes currently
    /// subscribed on this event with this identifier.
    ///
    /// Note that the function does not recompute invalid attributes instantly.
    /// The corresponding graph attributes will be recomputed on demand later on
    /// when you try to read them directly or indirectly through other related
    /// attributes.
    #[inline(always)]
    fn trigger_event(&mut self, id: Id, event: Event) {
        let revision = self.analyzer().db.commit_revision();

        self.analyzer().trigger_event(id, event, revision)
    }
}

/// A marker trait of the [AnalysisTask] and the [ExclusiveTask] tasks indicating
/// that these objects are allowed to read semantics information of
/// the [Analyzer]'s semantic graph.
///
/// References to the object implementing this trait are passed to
/// the corresponding semantics read functions such as
/// the [Attr::snapshot](crate::analysis::Attr::snapshot) function to get
/// a copy of the attribute's value.
///
/// This trait is sealed and cannot be implemented outside of this crate.
///
/// The SemanticAccess trait is a subtrait of the [AbstractTask] trait that
/// provides general access to the [Analyzer]'s content.
pub trait SemanticAccess<N: Grammar, H: TaskHandle, S: SyncBuildHasher>:
    AbstractTask<N, H, S>
{
}

/// A trait that provides general access to the [Analyzer]'s content.
///
/// This trait is a supertrait of the [MutationAccess] and the [SemanticAccess]
/// trait, and therefore implemented for all three kinds of tasks:
/// [AnalysisTask], [MutationTask], and [ExclusiveTask].
///
/// This trait is sealed and cannot be implemented outside of this crate.
pub trait AbstractTask<N: Grammar, H: TaskHandle, S: SyncBuildHasher>: TaskSealed<N, H, S> {
    /// Returns a [handle](TaskHandle) of the task through which you can
    /// manually check if the task has been interrupted, and through which you
    /// can interrupt the task manually.
    fn handle(&self) -> &H;

    /// A convenient function that checks if the task was interrupted.
    ///
    /// Returns Ok, if the task was not interrupted. Otherwise, returns
    /// [Interrupted](AnalysisError::Interrupted) error.
    #[inline(always)]
    fn proceed(&self) -> AnalysisResult<()> {
        if self.handle().is_triggered() {
            return Err(AnalysisError::Interrupted);
        }

        Ok(())
    }

    /// Returns true, if the analyzer has a document with the `id` identifier.
    #[inline(always)]
    fn contains_doc(&self, id: Id) -> bool {
        self.analyzer().docs.contains_key(&id)
    }

    /// Returns a RAII guard that provides read-only access to the analyzer's
    /// document with specified `id`.
    ///
    /// Returns a [MissingDocument](AnalysisError::MissingDocument) error
    /// if there is no document with the specified `id`.
    ///
    /// If the document exists but is currently locked for write (e.g., another
    /// mutation task is performing a [write](MutationAccess::write_to_doc)
    /// operation), the current thread will be blocked until the document is
    /// unlocked.
    #[inline(always)]
    fn read_doc(&self, id: Id) -> AnalysisResult<DocumentReadGuard<N, S>> {
        let Some(guard) = self.analyzer().docs.get(&id) else {
            return Err(AnalysisError::MissingDocument);
        };

        Ok(DocumentReadGuard::from(guard))
    }

    /// Returns a RAII guard that provides read-only access to the analyzer's
    /// document with specified `id`.
    ///
    /// This function is a non-blocking version of
    /// the [read_doc](Self::read_doc) function.
    ///
    /// Returns None if there is no document with the specified `id`.
    ///
    /// Returns None if the document currently locked for write.
    #[inline(always)]
    fn try_read_doc(&self, id: Id) -> Option<DocumentReadGuard<N, S>> {
        Some(DocumentReadGuard::from(self.analyzer().docs.try_get(&id)?))
    }

    /// Returns true if the document with the specified `id` exists in
    /// the analyzer, and this document **allows**
    /// [content edit](MutationAccess::write_to_doc) operations.
    #[inline(always)]
    fn is_doc_mutable(&self, id: Id) -> bool {
        let Some(guard) = self.analyzer().docs.get(&id) else {
            return false;
        };

        guard.doc.is_mutable()
    }

    /// Returns true if the document with the specified `id` exists in
    /// the analyzer, and this document **does not allow**
    /// [content edit](MutationAccess::write_to_doc) operations.
    #[inline(always)]
    fn is_doc_immutable(&self, id: Id) -> bool {
        let Some(guard) = self.analyzer().docs.get(&id) else {
            return false;
        };

        guard.doc.is_mutable()
    }

    /// Returns a snapshot of the [node references](NodeRef) set of the document
    /// with `id` that refer to syntax tree nodes belonging to specified `class`.
    ///
    /// The returning object is a clone of the already precomputed [Shared] set.
    /// Therefore, it is relatively cheap to call this function.
    ///
    /// If the document addressed by the `id` parameter does not exist in the
    /// analyzer, the function returns
    /// a [MissingDocument](AnalysisError::MissingDocument) error.
    #[inline(always)]
    fn snapshot_class(
        &self,
        id: Id,
        class: &<N::Classifier as Classifier>::Class,
    ) -> AnalysisResult<Shared<HashSet<NodeRef, S>>> {
        let Some(guard) = self.analyzer().docs.get(&id) else {
            return Err(AnalysisError::MissingDocument);
        };

        let Some(class_to_nodes) = guard.classes_to_nodes.get(class) else {
            return Ok(Shared::default());
        };

        Ok(class_to_nodes.nodes.clone())
    }

    /// Provides access to the Analyzer's
    /// [common semantics](Grammar::CommonSemantics), a special semantic
    /// feature that is instantiated during the Analyzer's creation. It does
    /// not belong to any specific document and is common across the entire
    /// Analyzer.
    ///
    /// If the Analyzer's grammar does not specify common semantics, this
    /// function returns a reference to the
    /// [VoidFeature](crate::analysis::VoidFeature).
    #[inline(always)]
    fn common(&self) -> &N::CommonSemantics {
        &self.analyzer().common
    }
}

pub trait TaskSealed<N: Grammar, H: TaskHandle, S: SyncBuildHasher> {
    fn analyzer(&self) -> &Analyzer<N, H, S>;

    fn revision(&self) -> Revision;
}
