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

use proc_macro2::Span;
use syn::{spanned::Spanned, Error, Result};

pub(in crate::node) enum VariantKind {
    Unspecified(Span),
    Root(Span),
    Comment(Span),
    Sentence(Span),
}

impl Spanned for VariantKind {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Unspecified(span) => *span,
            Self::Root(span) => *span,
            Self::Comment(span) => *span,
            Self::Sentence(span) => *span,
        }
    }
}

impl VariantKind {
    #[inline]
    pub(in crate::node) fn is_vacant(&self, span: Span) -> Result<()> {
        match self {
            Self::Unspecified(..) => Ok(()),
            Self::Root(..) => Err(Error::new(
                span,
                "The variant already annotated as a Root rule.",
            )),
            Self::Comment(..) => Err(Error::new(
                span,
                "The variant already annotated as a Comment rule.",
            )),
            Self::Sentence(..) => Err(Error::new(
                span,
                "The variant already annotated as a Sentence rule.",
            )),
        }
    }
}
