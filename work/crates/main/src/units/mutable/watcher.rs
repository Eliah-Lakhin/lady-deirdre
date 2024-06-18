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

use crate::syntax::{ErrorRef, NodeRef};

/// An object that tracks incremental reparse changes.
///
/// By supplying a reference to this object into
/// the [Document::write_and_watch](crate::units::Document::write_and_watch)
/// or [MutableUnit::write_and_watch](crate::units::MutableUnit::write_and_watch)
/// functions, the function will report any changes in the syntax tree
/// structure that occur during reparsing.
///
/// In particular, a mutable document (or a mutable unit) reports
/// the following types of events to the Watcher instance:
///
/// 1. Syntax tree node creation, deletion or node update events will be
///    reported to the [report_node](Watcher::report_node) function.
/// 2. Syntax tree parse error creation or deletion will be reported to
///    the [report_error](Watcher::report_error) function.
///
/// The Watcher interface does not distinguish between creation, deletion,
/// or updating. It is up to the trait implementor to decide what to do with
/// provided references. In particular, the trait implementor can collect these
/// references, and later test their validity. If the [NodeRef] or
/// the [ErrorRef] represents invalid reference, it means that corresponding
/// object has been deleted from the syntax tree. In turn, if the reference is
/// valid, it means that the corresponding object has been created or updated.
///
/// Also note that the only situation of where the syntax tree node could be
/// updated is when the parser changes the node's parent. In other words, when
/// the reparser transplants a syntax tree branch into another branch.
///
/// Additionally, the watching mechanism does not report descendant nodes of
/// the altered ancestor node. The algorithm only reports nodes and the errors
/// of the syntax tree that have been directly affected during reparsing.
///
/// Lady Deirdre provides two default implementations of the Watcher trait:
///
///  - [VoidWatcher], which is a noop.
///  - [DebugWatcher], that instantly prints node and error refs directly to
///    terminal for debugging purposes.
pub trait Watcher {
    /// This function is intended to be invoked by the inner algorithm of the
    /// [Document::write_and_watch](crate::units::Document::write_and_watch)
    /// or the [MutableUnit::write_and_watch](crate::units::MutableUnit::write_and_watch)
    /// functions on each syntax tree alteration occurring during reparsing.
    ///
    /// The `node_ref` parameter is a reference of the node in the syntax tree
    /// that has been created, deleted, or updated.
    fn report_node(&mut self, node_ref: &NodeRef);

    /// This function is intended to be invoked by the inner algorithm of the
    /// [Document::write_and_watch](crate::units::Document::write_and_watch)
    /// or the [MutableUnit::write_and_watch](crate::units::MutableUnit::write_and_watch)
    /// functions on each syntax tree alteration occurring during reparsing.
    ///
    /// The `error_ref` parameter is a reference of the parse error in
    /// the syntax tree that has been created or deleted.
    fn report_error(&mut self, error_ref: &ErrorRef);
}

/// A default implementation of the [Watcher] interface, which is a noop.
#[repr(transparent)]
pub struct VoidWatcher;

impl Default for VoidWatcher {
    #[inline(always)]
    fn default() -> Self {
        Self
    }
}

impl Watcher for VoidWatcher {
    #[inline(always)]
    fn report_node(&mut self, _node_ref: &NodeRef) {}

    #[inline(always)]
    fn report_error(&mut self, _error_ref: &ErrorRef) {}
}

/// A default implementation of the [Watcher] interface that prints each
/// report invocation to the terminal.
#[repr(transparent)]
pub struct DebugWatcher;

impl Default for DebugWatcher {
    #[inline(always)]
    fn default() -> Self {
        Self
    }
}

impl Watcher for DebugWatcher {
    #[inline(always)]
    fn report_node(&mut self, node_ref: &NodeRef) {
        println!("{node_ref:?}");
    }

    #[inline(always)]
    fn report_error(&mut self, error_ref: &ErrorRef) {
        println!("{error_ref:?}");
    }
}
