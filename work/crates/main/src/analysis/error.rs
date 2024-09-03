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

use std::{
    error::Error,
    fmt::{Display, Formatter},
};

/// A result of the semantic analysis.
///
/// See [AnalysisError] for details.
pub type AnalysisResult<T> = Result<T, AnalysisError>;

/// An error occurring during semantics analysis.
///
/// There are two types of errors:
///
///   - An [abnormal](AnalysisError::is_abnormal) that indicate an error
///     in the user code, such as an issue in
///     the [Grammar](crate::analysis::Grammar) configuration or misuse
///     of the analysis API. In this case, it is recommended to panic as early
///     as possible such that the panic backtrace will point to the exact piece
///     of code of where the error occurred.
///
///   - A normal error that should be propagated up to the caller of the current
///     function that returns an [AnalysisResult]. For example, such errors
///     should be returned from
///     the [Computable::compute](crate::analysis::Computable::compute)
///     implementations.
///
/// For convenience, the [AnalysisResult] type extended by
/// the [AnalysisResultEx] trait with the [AnalysisResultEx::unwrap_abnormal]
/// function that panics in place if the underlying error is abnormal, or
/// passes the Result object through if the underlying variant is Ok or denotes
/// a normal error.
///
/// Currently, the AnalysisError defines two normal errors:
///
///  - The [Interrupted](AnalysisError::Interrupted) error, which denotes that
///    the operation cannot be completed, because the underlying task has been
///    [signaled](crate::analysis::TaskHandle::is_triggered) to shut down.
///
///  - The [Timeout](AnalysisError::Timeout) error, which denotes that
///    the operation computation exceeded predefined timeout. This error may
///    occur due to recursion in the attributes graph, which is a user code
///    issue, or just a normal time out if the operation takes too long.
///    In the production builds (when the `debug_assertions` feature
///    is disabled), this type of error is a normal error, but in non-production
///    builds, this error considered abnormal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum AnalysisError {
    /// The operation cannot be completed because the underlying task has been
    /// [signaled](crate::analysis::TaskHandle::is_triggered) to shut down.
    ///
    /// This error is a **normal** error.
    Interrupted,

    /// The document referred to by the specified [id](crate::arena::Id) does
    /// not exits in the Analyzer.
    ///
    /// This error is an **abnormal** error.
    MissingDocument,

    /// The [content edit](crate::analysis::MutationAccess::write_to_doc)
    /// operation cannot be performed on the specified document, because
    /// the document is
    /// [not mutable](crate::analysis::MutationAccess::add_immutable_doc).
    ///
    /// This error is an **abnormal** error.
    ImmutableDocument,

    /// The specified [span](crate::lexis::ToSpan) is not valid for
    /// the specified document.
    ///
    /// This error is an **abnormal** error.
    InvalidSpan,

    /// An attempt to access an [Attr](crate::analysis::Attr) object which is
    /// not fully initialized.
    ///
    /// See [Feature Lifetime](crate::analysis::Feature#feature-lifetime) for details.
    ///
    /// This error is an **abnormal** error.
    UninitAttribute,

    /// An attempt to access an [Slot](crate::analysis::Slot) object which is
    /// not fully initialized.
    ///
    /// See [Feature Lifetime](crate::analysis::Feature#feature-lifetime) for details.
    ///
    /// This error is an **abnormal** error.
    UninitSlot,

    /// The referred attribute does not exist in the Analyzer's semantic graph.
    ///
    /// This error is an **abnormal** error.
    MissingAttribute,

    /// The referred slot does not exist in the Analyzer's semantic graph.
    ///
    /// This error is an **abnormal** error.
    MissingSlot,

    /// An attempt to access a [Semantics](crate::analysis::Semantics) object
    /// which is not fully initialized.
    ///
    /// See [Feature Lifetime](crate::analysis::Feature#feature-lifetime) for details.
    ///
    /// This error is an **abnormal** error.
    UninitSemantics,

    /// The specified syntax tree node does not have semantics.
    ///
    /// This error may occur, for example, if
    /// the [Grammar](crate::analysis::Grammar) object does not specify any
    /// semantics for any node, or if a particular type of the node does not
    /// specify semantics.
    ///
    /// This error is an **abnormal** error.
    MissingSemantics,

    /// The [attribute](crate::analysis::Attr) value type is differ from
    /// the specified type.
    ///
    /// This error is an **abnormal** error.
    TypeMismatch,

    /// The specified feature does not exist in the syntax tree node's
    /// semantics.
    ///
    /// This error is an **abnormal** error.
    MissingFeature,

    /// Operation timeout.
    ///
    /// This error indicates that the requested operation takes too long to
    /// finish, which is generally acceptable, or if the semantics graph has
    /// a recursion, which is an issue in the semantics design.
    ///
    /// This error is a **normal** error if the target build is a production
    /// build (`debug_assertions` feature is disabled). Otherwise, the error is
    /// **abnormal**.
    Timeout,
}

impl Display for AnalysisError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Interrupted => "Analysis task was interrupted.",
            Self::MissingDocument => "Referred document does not exist in the analyzer.",
            Self::ImmutableDocument => "An attempt to write into immutable document.",
            Self::InvalidSpan => "Provided span is not valid for the specified document.",
            Self::UninitAttribute => "An attempt to access uninitialized attribute object.",
            Self::UninitSlot => "An attempt to access uninitialized slot object.",
            Self::MissingAttribute => "Referred attribute does not exist in the analyzer.",
            Self::MissingSlot => "Referred slot does not exist in the analyzer.",
            Self::UninitSemantics => "An attempt to access uninitialized semantics object.",
            Self::MissingSemantics => "Node variant does not have semantics.",
            Self::TypeMismatch => "Incorrect attribute type.",
            Self::MissingFeature => "An attempt to access semantic feature that does not exist.",
            Self::Timeout => "Attribute computation timeout.",
        };

        formatter.write_str(text)
    }
}

impl Error for AnalysisError {}

impl AnalysisError {
    /// Returns true if the underlying error object denotes an issue in
    /// the user code, or in the [Grammar](crate::analysis::Grammar)
    /// configuration.
    #[inline(always)]
    pub fn is_abnormal(&self) -> bool {
        match self {
            Self::Interrupted => false,
            Self::Timeout => cfg!(debug_assertions),
            _ => true,
        }
    }
}

/// An helper extension of the [AnalysisResult].
///
/// The trait provides a function that unwraps
/// [abnormal](AnalysisError::is_abnormal) errors or passes the result object
/// through if the underlying error is normal or the result is Ok.
///
/// See [AnalysisError] for details.
pub trait AnalysisResultEx<T> {
    /// Panics in places with caller-tracking if the Result is
    /// an [abnormal](AnalysisError::is_abnormal) error; otherwise returns
    /// `self`.
    ///
    /// The intended use of this function is convenient unwrapping of
    /// the abnormal results in the call chain code:
    ///
    /// ```ignore
    /// let attr_read_guard = my_attr.read().unwrap_abnormal()?;
    /// ```
    fn unwrap_abnormal(self) -> AnalysisResult<T>;
}

impl<T> AnalysisResultEx<T> for AnalysisResult<T> {
    #[track_caller]
    #[inline(always)]
    fn unwrap_abnormal(self) -> AnalysisResult<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) if !error.is_abnormal() => Err(error),
            Err(error) => panic!("Analysis internal error. {error}"),
        }
    }
}
