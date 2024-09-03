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

use crate::shared_semantics::{
    lexis::SharedSemanticsToken,
    semantics::{CommonSemantics, KeySemantics, ModuleSemantics},
};

#[derive(Node)]
#[token(SharedSemanticsToken)]
#[trivia($Whitespace)]
#[semantics(CommonSemantics)]
pub enum SharedSemanticsNode {
    #[root]
    #[rule(defs: Def*)]
    Root {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        defs: Vec<NodeRef>,
        #[semantics]
        semantics: Semantics<ModuleSemantics>,
    },

    #[rule(key: Key $Assign value: (Ref | Num) $Semicolon)]
    Def {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        key: NodeRef,
        #[child]
        value: NodeRef,
        #[semantics]
        semantics: Semantics<VoidFeature<SharedSemanticsNode>>,
    },

    #[rule(token: $Ident)]
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

    #[rule(module: $Ident $DoubleColon ident: $Ident)]
    Ref {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        module: TokenRef,
        #[child]
        ident: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<SharedSemanticsNode>>,
    },

    #[rule(token: $Num)]
    Num {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
        #[semantics]
        semantics: Semantics<VoidFeature<SharedSemanticsNode>>,
    },
}
