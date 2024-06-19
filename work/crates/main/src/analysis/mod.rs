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

mod analyzer;
mod attribute;
mod compute;
mod database;
mod entry;
mod error;
mod grammar;
mod manager;
mod scope;
mod tasks;

pub use crate::analysis::{
    analyzer::{Analyzer, AnalyzerConfig},
    attribute::{Attr, AttrRef, NIL_ATTR_REF},
    compute::{AttrContext, AttrReadGuard, Computable, SharedComputable},
    database::Revision,
    entry::{
        DocumentReadGuard,
        Event,
        CUSTOM_EVENT_START_RANGE,
        DOC_ADDED_EVENT,
        DOC_ERRORS_EVENT,
        DOC_REMOVED_EVENT,
        DOC_UPDATED_EVENT,
    },
    error::{AnalysisError, AnalysisResult, AnalysisResultEx},
    grammar::{
        AbstractFeature,
        Classifier,
        Feature,
        Grammar,
        Initializer,
        Invalidator,
        Semantics,
        VoidClassifier,
        VoidFeature,
    },
    manager::{TaskHandle, TaskPriority, TriggerHandle},
    scope::{Scope, ScopeAttr},
    tasks::{
        AbstractTask,
        AnalysisTask,
        ExclusiveTask,
        MutationAccess,
        MutationTask,
        SemanticAccess,
    },
};
