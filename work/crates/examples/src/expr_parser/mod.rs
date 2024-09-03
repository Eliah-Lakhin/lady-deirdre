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

pub mod lexis;
pub mod parser;
pub mod syntax;

#[cfg(test)]
mod tests {
    use lady_deirdre::{
        syntax::SyntaxTree,
        units::{CompilationUnit, Document},
    };

    use crate::expr_parser::syntax::BoolNode;

    #[test]
    fn test_expression_parser() {
        let doc = Document::<BoolNode>::new_immutable("true & false & (true | false) & true");

        println!("{:#?}", doc.display(&doc.root_node_ref()));

        assert!(doc.errors().next().is_none());
    }

    #[test]
    fn test_expression_recovery() {
        let doc = Document::<BoolNode>::new_immutable("(false  true) & tru | false");

        println!("{:#?}", doc.display(&doc.root_node_ref()));

        for error in doc.errors() {
            println!("{:#}", error.display(&doc));
        }
    }
}
