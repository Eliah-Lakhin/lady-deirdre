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

use crate::{
    compiler::{ImmutableUnit, MutableUnit},
    lexis::{SourceCode, TokenBuffer},
    std::*,
    syntax::{Node, SyntaxTree},
    Document,
};

pub trait CompilationUnit:
    SourceCode<Token = <<Self as SyntaxTree>::Node as Node>::Token> + SyntaxTree
{
    fn is_mutable(&self) -> bool;

    #[inline(always)]
    fn is_immutable(&self) -> bool {
        !self.is_mutable()
    }

    fn into_token_buffer(self) -> TokenBuffer<<Self as SourceCode>::Token>;

    #[inline(always)]
    fn into_document(self) -> Document<<Self as SyntaxTree>::Node>
    where
        Self: Sized,
    {
        match self.is_mutable() {
            true => Document::Mutable(self.into_mutable_unit()),
            false => Document::Immutable(self.into_immutable_unit()),
        }
    }

    #[inline(always)]
    fn into_mutable_unit(self) -> MutableUnit<<Self as SyntaxTree>::Node>
    where
        Self: Sized,
    {
        self.into_token_buffer().into_mutable_unit()
    }

    #[inline(always)]
    fn into_immutable_unit(self) -> ImmutableUnit<<Self as SyntaxTree>::Node>
    where
        Self: Sized,
    {
        self.into_token_buffer().into_immutable_unit()
    }
}
