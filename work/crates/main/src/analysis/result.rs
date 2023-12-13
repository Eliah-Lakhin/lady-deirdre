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
    UninitAttribute,
    MissingAttribute,
    TypeMismatch,
    CycleDetected,
}

impl Display for AnalysisError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        let text = match self {
            AnalysisError::Interrupted => "Analysis task was interrupted.",

            AnalysisError::MissingDocument => "Referred document does not exist in the analyzer.",

            AnalysisError::ImmutableDocument => "An attempt to write into immutable document.",

            AnalysisError::InvalidSpan => "Provided span is not valid for specified document.",

            AnalysisError::UninitAttribute => {
                "An attempt to access uninitialized attribute object."
            }

            AnalysisError::MissingAttribute => "Referred attribute does not exist in the analyzer.",

            AnalysisError::TypeMismatch => "Incorrect attribute type.",

            AnalysisError::CycleDetected => "Attribute graph contains a cycle.",
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
