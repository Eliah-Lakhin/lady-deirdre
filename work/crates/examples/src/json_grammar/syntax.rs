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
    lexis::TokenRef,
    syntax::{Node, NodeRef},
};

use crate::json_grammar::lexis::JsonToken;

#[derive(Node)]
#[token(JsonToken)]
#[trivia($Whitespace)]
#[define(ANY = Object | Array | True | False | String | Number | Null)]
#[recovery(
    $BraceClose,
    $BracketClose,
    [$BraceOpen..$BraceClose],
    [$BracketOpen..$BracketClose],
)]
pub enum JsonNode {
    #[root]
    #[rule(object: Object)]
    Root {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        object: NodeRef,
    },

    #[rule(start: $BraceOpen (entries: Entry)*{$Comma} end: $BraceClose)]
    #[denote(OBJECT)]
    #[recovery(
        [$BraceOpen..$BraceClose],
        [$BracketOpen..$BracketClose],
    )]
    Object {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        entries: Vec<NodeRef>,
        #[child]
        end: TokenRef,
    },

    #[rule(key: String $Colon value: ANY)]
    #[denote(ENTRY)]
    Entry {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        key: NodeRef,
        #[child]
        value: NodeRef,
    },

    #[rule(start: $BracketOpen (items: ANY)*{$Comma} end: $BracketClose)]
    #[denote(ARRAY)]
    #[recovery(
        [$BraceOpen..$BraceClose],
        [$BracketOpen..$BracketClose],
    )]
    Array {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        start: TokenRef,
        #[child]
        items: Vec<NodeRef>,
        #[child]
        end: TokenRef,
    },

    #[rule(value: $String)]
    #[denote(STRING)]
    #[secondary]
    String {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        value: TokenRef,
    },

    #[rule(value: $Number)]
    #[denote(NUMBER)]
    #[secondary]
    Number {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        value: TokenRef,
    },

    #[rule(token: $True)]
    #[denote(TRUE)]
    #[secondary]
    True {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
    },

    #[rule(token: $False)]
    #[denote(FALSE)]
    #[secondary]
    False {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
    },

    #[rule(token: $Null)]
    #[denote(NULL)]
    #[secondary]
    Null {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        token: TokenRef,
    },
}
