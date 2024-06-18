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

use lady_deirdre::syntax::{Node, NodeRef};

use crate::expr_parser::{lexis::BoolToken, parser::parse_expr};

#[derive(Node)]
#[token(BoolToken)]
#[trivia($Whitespace)]
pub enum BoolNode {
    #[root]
    #[rule(expr: Expr)]
    Root {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        expr: NodeRef,
    },

    #[rule($ParenOpen | $True | $False)]
    #[denote(EXPR)]
    #[describe("expression", "<expr>")]
    #[parser(parse_expr(session))]
    Expr {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        content: NodeRef,
    },

    #[denote(TRUE)]
    #[describe("bool", "<true>")]
    True {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
    },

    #[denote(FALSE)]
    #[describe("bool", "<false>")]
    False {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
    },

    #[denote(AND)]
    #[describe("operator", "<and op>")]
    And {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        left: NodeRef,
        #[child]
        right: NodeRef,
    },

    #[denote(OR)]
    #[describe("operator", "<or op>")]
    Or {
        #[node]
        node: NodeRef,
        #[parent]
        parent: NodeRef,
        #[child]
        left: NodeRef,
        #[child]
        right: NodeRef,
    },
}
