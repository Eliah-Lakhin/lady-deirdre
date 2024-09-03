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
    any::TypeId,
    fmt::{Debug, Formatter},
    marker::PhantomData,
};

use crate::{
    lexis::Token,
    syntax::{AbstractNode, Capture, Key, Node, NodeRef, NodeRule, SyntaxSession, NON_RULE},
};

/// A Node without a syntax parser.
///
/// You are encouraged to use this object with
/// the [Document](crate::units::Document) or
/// the [MutableUnit](crate::units::MutableUnit) when you need an incremental
/// lexical parser, but you don't need a syntax parser:
/// `Document::<VoidSyntax<MyToken>>::new_mutable("foo bar baz")`.
///
/// This object makes the syntax parsing stage of the incremental reprarser
/// a noop.
#[derive(Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct VoidSyntax<T: Token> {
    _token: PhantomData<T>,
}

impl<T: Token> Default for VoidSyntax<T> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            _token: PhantomData,
        }
    }
}

impl<T: Token> Debug for VoidSyntax<T> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("NoSyntax")
    }
}

impl<T: Token> AbstractNode for VoidSyntax<T> {
    #[inline(always)]
    fn rule(&self) -> NodeRule {
        NON_RULE
    }

    #[inline(always)]
    fn name(&self) -> Option<&'static str> {
        None
    }

    #[inline(always)]
    fn describe(&self, _verbose: bool) -> Option<&'static str> {
        None
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
    fn capture(&self, _key: Key) -> Option<Capture> {
        None
    }

    #[inline(always)]
    fn capture_keys(&self) -> &'static [Key<'static>] {
        &[]
    }

    #[inline(always)]
    fn rule_name(_rule: NodeRule) -> Option<&'static str>
    where
        Self: Sized,
    {
        None
    }

    #[inline(always)]
    fn rule_description(_rule: NodeRule, _verbose: bool) -> Option<&'static str>
    where
        Self: Sized,
    {
        None
    }
}

impl<T: Token> Node for VoidSyntax<T> {
    type Token = T;

    #[inline(always)]
    fn parse<'code>(
        _session: &mut impl SyntaxSession<'code, Node = Self>,
        _rule: NodeRule,
    ) -> Self {
        Self::default()
    }
}

#[inline(always)]
pub(crate) fn is_void_syntax<N: Node>() -> bool {
    TypeId::of::<N>() == TypeId::of::<VoidSyntax<N::Token>>()
}
