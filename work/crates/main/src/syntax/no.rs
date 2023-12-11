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
    analysis::{FeatureInitializer, FeatureInvalidator},
    lexis::Token,
    std::*,
    sync::SyncBuildHasher,
    syntax::{Children, Node, NodeRef, NodeRule, ParseError, SyntaxSession, NON_RULE},
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
///     units::Document,
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
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_str("NoSyntax")
    }
}

impl<T: Token> Node for NoSyntax<T> {
    type Token = T;
    type Error = ParseError;

    #[inline(always)]
    fn parse<'code>(
        _session: &mut impl SyntaxSession<'code, Node = Self>,
        _rule: NodeRule,
    ) -> Self {
        Self::nil()
    }

    #[inline(always)]
    fn rule(&self) -> NodeRule {
        NON_RULE
    }

    #[inline(always)]
    fn node_ref(&self) -> NodeRef {
        NodeRef::nil()
    }

    #[inline(always)]
    fn parent_ref(&self) -> NodeRef {
        NodeRef::nil()
    }

    #[inline(always)]
    fn set_parent_ref(&mut self, _parent_ref: NodeRef) {}

    #[inline(always)]
    fn children(&self) -> Children {
        Children::new()
    }

    #[inline(always)]
    fn initialize<S: SyncBuildHasher>(&mut self, _initializer: &mut FeatureInitializer<Self, S>) {}

    #[inline(always)]
    fn invalidate<S: SyncBuildHasher>(&self, _invalidator: &mut FeatureInvalidator<Self, S>) {}

    #[inline(always)]
    fn name(_rule: NodeRule) -> Option<&'static str> {
        None
    }

    #[inline(always)]
    fn describe(_rule: NodeRule, _verbose: bool) -> Option<&'static str> {
        None
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
