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
