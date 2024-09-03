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
        syntax::{PolyRef, SyntaxTree},
    };

    use crate::shared_semantics::{semantics::KeyResolution, syntax::SharedSemanticsNode};

    #[test]
    fn test_multi_modules() {
        let analyzer = Analyzer::<SharedSemanticsNode>::new(AnalyzerConfig::default());

        {
            let handle = TriggerHandle::new();

            let mut task = analyzer.mutate(&handle, 1).unwrap();

            let doc_id = task.add_mutable_doc("x = 10; y = module_2::b; z = module_2::c;");

            doc_id.set_name("module_1");

            task.common()
                .modules
                .mutate(&task, |modules| {
                    let _ = modules.insert(String::from("module_1"), doc_id);

                    true
                })
                .unwrap();
        }

        {
            let handle = TriggerHandle::new();

            let mut task = analyzer.mutate(&handle, 1).unwrap();

            let doc_id = task.add_mutable_doc("a = module_1::x; b = module_2::c; c = 20;");

            doc_id.set_name("module_2");

            task.common()
                .modules
                .mutate(&task, |modules| {
                    let _ = modules.insert(String::from("module_2"), doc_id);

                    true
                })
                .unwrap();
        }

        {
            let handle = TriggerHandle::new();
            let task = analyzer.analyze(&handle, 1).unwrap();
            println!("{:#}", DisplayModules(&task));
        }
    }

    struct DisplayModules<'a>(&'a AnalysisTask<'a, SharedSemanticsNode>);

    impl<'a> Display for DisplayModules<'a> {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            let (_, modules) = self.0.common().modules.snapshot(self.0).unwrap();

            for doc_id in modules.values() {
                let doc_read = self.0.read_doc(*doc_id).unwrap();
                let doc = doc_read.deref();

                let mut snippet = formatter.snippet(doc);

                snippet.set_caption(doc.id().name());

                let SharedSemanticsNode::Root { defs, .. } = doc.root() else {
                    unreachable!("Malformed root");
                };

                for def_ref in defs {
                    let Some(SharedSemanticsNode::Def { key, .. }) = def_ref.deref(doc) else {
                        continue;
                    };

                    let Some(SharedSemanticsNode::Key {
                        token, semantics, ..
                    }) = key.deref(doc)
                    else {
                        continue;
                    };

                    let Some(span) = token.span(doc) else {
                        continue;
                    };

                    let (_, resolution) = semantics
                        .get()
                        .unwrap()
                        .resolution
                        .snapshot(self.0)
                        .unwrap();

                    match resolution {
                        KeyResolution::Unresolved => {
                            snippet.annotate(span, AnnotationPriority::Default, "unresolved");
                        }
                        KeyResolution::Recusrive => {
                            snippet.annotate(span, AnnotationPriority::Default, "recusrive");
                        }
                        KeyResolution::Number(value) => {
                            snippet.annotate(span, AnnotationPriority::Default, format!("{value}"));
                        }
                    };
                }

                snippet.finish()?;
                drop(snippet);
                formatter.write_str("\n")?;
            }

            Ok(())
        }
    }
}
