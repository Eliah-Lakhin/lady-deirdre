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

use std::fmt::{Display, Formatter};

use lady_deirdre::{
    lexis::{SourceCode, TokenRef},
    syntax::{NodeRef, SyntaxBuffer, SyntaxTree},
};

use crate::json::{lexis::JsonToken, syntax::JsonNode};

pub trait ToJsonString {
    fn to_json_string(&self) -> String;
}

impl<L: SourceCode<Token = JsonToken>> ToJsonString for L {
    fn to_json_string(&self) -> String {
        let syntax = SyntaxBuffer::parse(self.cursor(..));

        let formatter = JsonFormatter {
            lexis: self,
            syntax: &syntax,
        };

        formatter.to_string()
    }
}

pub struct JsonFormatter<'a, L: SourceCode<Token = JsonToken>, S: SyntaxTree<Node = JsonNode>> {
    pub lexis: &'a L,
    pub syntax: &'a S,
}

impl<'a, L, S> Display for JsonFormatter<'a, L, S>
where
    L: SourceCode<Token = JsonToken>,
    S: SyntaxTree<Node = JsonNode>,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.format_node(&self.syntax.root_node_ref()))
    }
}

impl<'a, L, S> JsonFormatter<'a, L, S>
where
    L: SourceCode<Token = JsonToken>,
    S: SyntaxTree<Node = JsonNode>,
{
    fn format_node(&self, node_ref: &NodeRef) -> String {
        let node: &JsonNode = match node_ref.deref(self.syntax) {
            None => return String::from("?"),
            Some(node) => node,
        };

        match node {
            JsonNode::Root { object } => self.format_node(object),

            JsonNode::Object { entries } => {
                format!(
                    "{{{}}}",
                    entries
                        .into_iter()
                        .map(|entry| self.format_node(entry))
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            }

            JsonNode::Entry { key, value } => {
                format!("{:#}: {}", self.format_token(key), self.format_node(value),)
            }

            JsonNode::Array { items } => {
                format!(
                    "[{}]",
                    items
                        .into_iter()
                        .map(|item| self.format_node(item))
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            }

            JsonNode::String { value } | JsonNode::Number { value } => self.format_token(value),

            JsonNode::True => String::from("true"),

            JsonNode::False => String::from("false"),

            JsonNode::Null => String::from("null"),
        }
    }

    fn format_token(&self, token_ref: &TokenRef) -> String {
        token_ref.string(self.lexis).unwrap_or("?").to_string()
    }
}
