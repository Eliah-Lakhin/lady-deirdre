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

pub mod lexis;
pub mod semantics;
pub mod syntax;

#[cfg(test)]
mod tests {
    use std::{
        fmt::{Display, Formatter},
        ops::Deref,
    };

    use lady_deirdre::{
        analysis::{
            AbstractTask,
            AnalysisTask,
            Analyzer,
            AnalyzerConfig,
            MutationAccess,
            TriggerHandle,
        },
        arena::Identifiable,
        format::{AnnotationPriority, SnippetFormatter},
        lexis::Position,
        syntax::{PolyRef, SyntaxTree},
        units::Document,
    };

    use crate::chain_analysis::{
        semantics::{ChainNodeClass, GlobalResolution},
        syntax::ChainNode,
    };

    #[test]
    fn test_chain_analysis() {
        static INPUT: &'static str = r#"
        {
            x = 100;

            {
                y = x;

                {
                    z = y;
                    w = 200;
                    u = w;
                }
            }
        }"#;

        let analyzer = Analyzer::<ChainNode>::new(AnalyzerConfig::default());

        let doc_id;

        {
            let handle = TriggerHandle::new();

            let mut task = analyzer.mutate(&handle, 1).unwrap();

            doc_id = task.add_mutable_doc(INPUT);

            let doc_read = task.read_doc(doc_id).unwrap();

            for error in doc_read.errors() {
                println!("{:#}", error.display(doc_read.deref()));
            }
        }

        {
            let handle = TriggerHandle::new();

            let task = analyzer.analyze(&handle, 1).unwrap();

            let doc_read = task.read_doc(doc_id).unwrap();

            println!(
                "{:#}",
                DisplayValues {
                    doc: doc_read.deref(),
                    task: &task,
                }
            )
        }

        {
            let handle = TriggerHandle::new();

            let mut task = analyzer.mutate(&handle, 1).unwrap();

            task.write_to_doc(doc_id, Position::new(3, 18)..Position::new(3, 19), "5")
                .unwrap();

            let doc_read = task.read_doc(doc_id).unwrap();

            for error in doc_read.errors() {
                println!("{:#}", error.display(doc_read.deref()));
            }
        }

        {
            let handle = TriggerHandle::new();

            let task = analyzer.analyze(&handle, 1).unwrap();

            let doc_read = task.read_doc(doc_id).unwrap();

            println!(
                "{:#}",
                DisplayValues {
                    doc: doc_read.deref(),
                    task: &task,
                }
            )
        }
    }

    struct DisplayValues<'a> {
        doc: &'a Document<ChainNode>,
        task: &'a AnalysisTask<'a, ChainNode>,
    }

    impl<'a> Display for DisplayValues<'a> {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            let mut snippet = formatter.snippet(self.doc);

            let all_keys = self
                .task
                .snapshot_class(self.doc.id(), &ChainNodeClass::AllKeys)
                .unwrap();

            for key_ref in all_keys.as_ref() {
                let Some(ChainNode::Key {
                    token, semantics, ..
                }) = key_ref.deref(self.doc)
                else {
                    continue;
                };

                let Some(span) = token.span(self.doc) else {
                    continue;
                };

                let (_, resolution) = semantics
                    .get()
                    .unwrap()
                    .global_resolution
                    .snapshot(self.task)
                    .unwrap();

                match resolution {
                    GlobalResolution::Broken => {
                        snippet.annotate(span, AnnotationPriority::Default, "broken")
                    }
                    GlobalResolution::Resolved(num) => {
                        snippet.annotate(span, AnnotationPriority::Default, format!("{num}"))
                    }
                };
            }

            snippet.finish()
        }
    }
}
