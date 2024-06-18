////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

mod captures;
mod error;
mod immutable;
mod morphism;
mod node;
mod observer;
mod parse;
mod recovery;
mod rule;
mod session;
mod tree;
mod void;

pub(crate) use crate::syntax::void::is_void_syntax;
pub use crate::syntax::{
    captures::{Capture, CaptureIntoIter, CapturesIter, ChildrenIter, Key},
    error::{ErrorRef, SyntaxError, NIL_ERROR_REF},
    immutable::ImmutableSyntaxTree,
    morphism::{PolyRef, PolyVariant, RefKind},
    node::{AbstractNode, Node, NodeRef, NIL_NODE_REF},
    observer::{DebugObserver, Observer, VoidObserver},
    parse::{ParseBlank, ParseNode, ParseNodeChild, ParseToken, ParseTree},
    recovery::{Recovery, RecoveryResult, UNLIMITED_RECOVERY},
    rule::{NodeRule, NodeSet, EMPTY_NODE_SET, NON_RULE, ROOT_RULE},
    session::SyntaxSession,
    tree::{ErrorIter, NodeIter, SyntaxTree, Visitor},
    void::VoidSyntax,
};
