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

use crate::std::*;

pub type AnalysisResult<T> = Result<T, AnalysisError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnalysisError {
    Interrupted,
    MissingDocument,
    ImmutableDocument,
    InvalidSpan,
    DuplicateHandle,
    UninitAttribute,
    MissingAttribute,
    UninitSemantics,
    TypeMismatch,
    MissingScope,
    MissingFeature,
    CycleDetected,
}

impl Display for AnalysisError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        let text = match self {
            Self::Interrupted => "Analysis task was interrupted.",
            Self::MissingDocument => "Referred document does not exist in the analyzer.",
            Self::ImmutableDocument => "An attempt to write into immutable document.",
            Self::InvalidSpan => "Provided span is not valid for specified document.",
            Self::DuplicateHandle => "Provided analysis handle already used by another task.",
            Self::UninitAttribute => "An attempt to access uninitialized attribute object.",
            Self::MissingAttribute => "Referred attribute does not exist in the analyzer.",
            Self::UninitSemantics => "An attempt to access uninitialized semantics object.",
            Self::TypeMismatch => "Incorrect attribute type.",
            Self::MissingScope => "One of the semantics object does not have scope attribute.",
            Self::MissingFeature => "One of the semantics objects does not have scope feature.",
            Self::CycleDetected => "Attribute graph contains a cycle.",
        };

        formatter.write_str(text)
    }
}

impl Error for AnalysisError {}

impl AnalysisError {
    #[inline(always)]
    pub fn is_interrupt(&self) -> bool {
        match self {
            Self::Interrupted => true,
            _ => false,
        }
    }
}

pub trait AnalysisResultEx<T> {
    fn unwrap_abnormal(self) -> AnalysisResult<T>;
}

impl<T> AnalysisResultEx<T> for AnalysisResult<T> {
    #[track_caller]
    #[inline(always)]
    fn unwrap_abnormal(self) -> AnalysisResult<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) if error.is_interrupt() => Err(error),
            Err(error) => panic!("Analysis internal error. {error}"),
        }
    }
}
