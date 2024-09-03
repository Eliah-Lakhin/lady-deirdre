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

use std::fmt::{Debug, Display, Formatter};

use crate::{
    arena::{Id, Identifiable, SubId},
    format::SnippetFormatter,
    lexis::{SourceCode, Token, TokenBuffer},
    syntax::{ImmutableSyntaxTree, Node},
    units::{CompilationUnit, Lexis, Syntax},
};

/// A compilation unit without reparse capabilities.
///
/// This serves as an inner component
/// of the immutable [Document](crate::units::Document).
///
/// ImmutableUnit implements the same set of interfaces and provides the same
/// set of features, except for the option to edit an already created document.
///
/// You are encouraged to use this object if you don’t need a uniform
/// mutable and immutable API of the Document.
///
/// Under the hood, the ImmutableUnit contains a pair of interconnected
/// [TokenBuffer] and [ImmutableSyntaxTree]. If you only need a lexical parser
/// without extra overhead consider using a TokenBuffer directly.
pub struct ImmutableUnit<N: Node> {
    lexis: TokenBuffer<N::Token>,
    syntax: ImmutableSyntaxTree<N>,
}

impl<N: Node> Identifiable for ImmutableUnit<N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.lexis.id()
    }
}

impl<N: Node> Debug for ImmutableUnit<N> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter
            .debug_struct("ImmutableUnit")
            .field("id", &self.lexis.id())
            .field("length", &self.lexis.length())
            .finish_non_exhaustive()
    }
}

impl<N: Node> Display for ImmutableUnit<N> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter
            .snippet(self)
            .set_caption(format!("ImmutableUnit({})", self.id()))
            .finish()
    }
}

impl<N: Node> Lexis for ImmutableUnit<N> {
    type Lexis = TokenBuffer<N::Token>;

    #[inline(always)]
    fn lexis(&self) -> &Self::Lexis {
        &self.lexis
    }
}

impl<N: Node> Syntax for ImmutableUnit<N> {
    type Syntax = ImmutableSyntaxTree<N>;

    #[inline(always)]
    fn syntax(&self) -> &Self::Syntax {
        &self.syntax
    }

    #[inline(always)]
    fn syntax_mut(&mut self) -> &mut Self::Syntax {
        &mut self.syntax
    }
}

impl<N: Node, S: AsRef<str>> From<S> for ImmutableUnit<N> {
    #[inline(always)]
    fn from(string: S) -> Self {
        Self::new(string)
    }
}

impl<T: Token> TokenBuffer<T> {
    /// Turns this token buffer into ImmutableUnit.
    ///
    /// The `N` generic parameter specifies a type of the syntax tree [Node]
    /// with the `T` [lexis](Node::Token).
    #[inline(always)]
    pub fn into_immutable_unit<N>(mut self) -> ImmutableUnit<N>
    where
        N: Node<Token = T>,
    {
        self.reset_id();

        let syntax = ImmutableSyntaxTree::parse_with_id(SubId::fork(self.id()), self.cursor(..));

        ImmutableUnit {
            lexis: self,
            syntax,
        }
    }
}

impl<N: Node> CompilationUnit for ImmutableUnit<N> {
    #[inline(always)]
    fn is_mutable(&self) -> bool {
        false
    }

    #[inline(always)]
    fn into_token_buffer(mut self) -> TokenBuffer<N::Token> {
        self.lexis.reset_id();

        self.lexis
    }

    #[inline(always)]
    fn into_immutable_unit(self) -> ImmutableUnit<N> {
        self
    }
}

impl<N: Node> ImmutableUnit<N> {
    /// Creates an ImmutableUnit from the source code `text`.
    ///
    /// The parameter could be a [TokenBuffer] or just an arbitrary string.
    pub fn new(text: impl Into<TokenBuffer<N::Token>>) -> Self {
        let lexis = text.into();

        let syntax = ImmutableSyntaxTree::parse_with_id(SubId::fork(lexis.id()), lexis.cursor(..));

        Self { lexis, syntax }
    }
}
