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
    arena::{Id, Identifiable},
    format::SnippetFormatter,
    lexis::{SourceCode, Token, TokenBuffer},
    std::*,
    syntax::{ImmutableSyntaxTree, Node},
    units::{CompilationUnit, Lexis, Syntax},
};

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
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter
            .debug_struct("ImmutableUnit")
            .field("id", &self.lexis.id())
            .field("length", &self.lexis.length())
            .finish_non_exhaustive()
    }
}

impl<N: Node> Display for ImmutableUnit<N> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
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
    #[inline(always)]
    pub fn into_immutable_unit<N>(mut self) -> ImmutableUnit<N>
    where
        N: Node<Token = T>,
    {
        self.reset_id();

        let syntax = ImmutableSyntaxTree::with_id(self.id(), self.cursor(..));

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
    pub fn new(text: impl Into<TokenBuffer<N::Token>>) -> Self {
        let lexis = text.into();

        let syntax = ImmutableSyntaxTree::with_id(lexis.id(), lexis.cursor(..));

        Self { lexis, syntax }
    }
}
