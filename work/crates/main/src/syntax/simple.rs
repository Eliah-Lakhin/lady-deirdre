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
    lexis::SimpleToken,
    std::*,
    syntax::{Node, NodeRef, ParseError},
};

/// A common generic syntax.
///
/// This is a companion object of the [SimpleToken](SimpleToken) lexis that represents
/// a set of nested parens: parenthesis, braces and brackets.
#[derive(Node, Clone, Debug, PartialEq, Eq)]
#[token(SimpleToken)]
#[error(ParseError)]
#[trivia($Number | $Symbol | $Identifier | $String | $Char | $Whitespace | $Mismatch)]
#[define(ANY = Parenthesis | Brackets | Braces)]
#[recovery([$ParenOpen..$ParenClose], [$BracketOpen..$BracketClose], [$BraceOpen..$BraceClose])]
pub enum SimpleNode {
    /// A root node that contains all top-level parents.
    #[root]
    #[rule(inner: ANY*)]
    Root {
        /// Top-level parens of the source code.
        inner: Vec<NodeRef>,
    },

    /// A pair of parenthesis(`( ... )`)
    #[rule($ParenOpen & inner: ANY* & $ParenClose)]
    Parenthesis {
        /// Parens that nested inside this Parenthesis pair.
        inner: Vec<NodeRef>,
    },

    /// A pair of brackets(`[ ... ]`)
    #[rule($BracketOpen & inner: ANY* & $BracketClose)]
    Brackets {
        /// Parens that nested inside this Brackets pair.
        inner: Vec<NodeRef>,
    },

    /// A pair of braces(`{ ... }`)
    #[rule($BraceOpen & inner: ANY* & $BraceClose)]
    Braces {
        /// Parens that nested inside this Braces pair.
        inner: Vec<NodeRef>,
    },
}

impl Display for SimpleNode {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        Debug::fmt(self, formatter)
    }
}

impl SimpleNode {
    /// Returns a complete slice of the inner parens nested inside this parens node.
    #[inline(always)]
    pub fn inner(&self) -> &[NodeRef] {
        match self {
            Self::Root { inner } => &inner,
            Self::Parenthesis { inner } => &inner,
            Self::Brackets { inner } => &inner,
            Self::Braces { inner } => &inner,
        }
    }
}
