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

pub mod highlighter;

#[cfg(test)]
mod tests {
    use lady_deirdre::{format::AnnotationPriority, lexis::Position, units::Document};

    use crate::json_highlight::highlighter::JsonSnippet;

    #[test]
    fn test_snippet() {
        static INPUT: &'static str = r#"
        {
            "foo": [1, 2, 3],
            "bar": {
                "a": true,
                "b": false,
                "c": null,
                "d": "string"
            }
        }
        "#;

        let doc = Document::new_immutable(INPUT);

        println!(
            "{:#}",
            JsonSnippet {
                doc: &doc,
                annotation: Vec::new(),
            }
        );

        println!(
            "{:#}",
            JsonSnippet {
                doc: &doc,
                annotation: vec![
                    (
                        Position::new(3, 20)..Position::new(3, 29),
                        AnnotationPriority::Secondary,
                        "Annotation message."
                    ),
                    (
                        Position::new(10, 9)..Position::new(10, 10),
                        AnnotationPriority::Primary,
                        "Another message."
                    )
                ],
            }
        );
    }
}
