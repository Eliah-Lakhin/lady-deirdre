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

use lady_deirdre::{
    analysis::{Semantics, VoidFeature},
    lexis::TokenRef,
    syntax::{Node, NodeRef},
};

use crate::chain_analysis::{
    lexis::ChainToken,
    semantics::{BlockSemantics, ChainNodeClassifier, KeySemantics},
};

#[derive(Node)]
#[token(ChainToken)]
#[trivia($Whitespace)]
#[recovery(
    $BraceClose, $Semicolon,
    [$BraceOpen..$BraceClose],
)]
#[classifier(ChainNodeClassifier)]
pub enum ChainNode {
    #[root]
    #[rule(block: Block)]
    Root {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        block: NodeRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ChainNode>>,
    },

    #[rule($BraceOpen statements: (Block | Assignment)* $BraceClose)]
    #[recovery(
        [$BraceOpen..$BraceClose],
    )]
    #[scope]
    Block {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        statements: Vec<NodeRef>,
        #[semantics]
        semantics: Semantics<BlockSemantics>,
    },

    #[rule(key: Key $Assign value: (Ref | Num) $Semicolon)]
    #[secondary]
    Assignment {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        key: NodeRef,
        #[child]
        value: NodeRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ChainNode>>,
    },

    #[rule(token: $Ident)]
    #[secondary]
    Key {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<KeySemantics>,
    },

    #[rule(token: $Ident)]
    #[secondary]
    Ref {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ChainNode>>,
    },

    #[rule(token: $Num)]
    #[secondary]
    Num {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<ChainNode>>,
    },
}
