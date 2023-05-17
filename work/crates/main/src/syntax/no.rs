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
    lexis::Token,
    std::*,
    syntax::{Node, SyntaxError, SyntaxRule, SyntaxSession},
};

/// A special marker that forcefully skips syntax parsing stage.
///
/// This object implements [Node](crate::syntax::Node) interface, but does not produce any syntax
/// data, and skips syntax parsing stage from the beginning.
///
/// You can use this object when the syntax manager(e.g. [Document](crate::Document)) requires full
/// syntax specification, but you only need a lexical data to be managed.
///
/// ```rust
/// use lady_deirdre::{
///     syntax::{NoSyntax, SyntaxTree},
///     lexis::SimpleToken,
///     Document,
/// };
///
/// use std::mem::size_of;
///
/// let doc = Document::<NoSyntax<SimpleToken>>::from("foo bar baz");
///
/// // Resolves to a single instance of NoSyntax of zero size.
/// assert!(doc.root_node_ref().deref(&doc).is_some());
/// assert_eq!(size_of::<NoSyntax<SimpleToken>>(), 0)
/// ```
#[derive(Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct NoSyntax<T: Token> {
    _token: PhantomData<T>,
}

impl<T: Token> Debug for NoSyntax<T> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str("NoSyntax")
    }
}

impl<T: Token> Node for NoSyntax<T> {
    type Token = T;
    type Error = SyntaxError;

    #[inline(always)]
    fn new<'code>(
        _rule: SyntaxRule,
        _session: &mut impl SyntaxSession<'code, Node = Self>,
    ) -> Self {
        Self::nil()
    }
}

impl<T: Token> NoSyntax<T> {
    #[inline(always)]
    pub(crate) fn nil() -> Self {
        Self {
            _token: PhantomData::default(),
        }
    }
}
