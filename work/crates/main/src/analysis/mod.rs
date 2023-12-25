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

mod analysis;
mod analyzer;
mod attribute;
mod database;
mod features;
mod grammar;
mod memo;
mod mutation;
mod record;
mod result;
mod scope;
mod semantics;
mod signal;
mod table;
mod tasks;
mod validation;

pub use crate::analysis::{
    analysis::AnalysisTask,
    analyzer::{Analyzer, DocumentReadGuard, Revision},
    attribute::{Attr, AttrContext, AttrReadGuard, AttrRef, Computable, NIL_ATTR_REF},
    features::{AbstractFeature, Feature},
    grammar::Grammar,
    mutation::{FeatureInitializer, FeatureInvalidator, MutationTask},
    result::{AnalysisError, AnalysisResult},
    scope::{Scope, ScopeAttr},
    semantics::Semantics,
    signal::{Lifecycle, Signal},
    tasks::{AbstractTask, TASKS_ALL, TASKS_ANALYSIS, TASKS_EXCLUSIVE, TASKS_MUTATION},
};

#[cfg(test)]
mod tests {
    use crate::{
        analysis::{
            AbstractFeature,
            AbstractTask,
            AnalysisResult,
            Analyzer,
            Attr,
            AttrContext,
            Computable,
            Feature,
            Semantics,
        },
        lexis::{SimpleToken, TokenRef},
        std::*,
        sync::{Latch, SyncBuildHasher},
        syntax::{Key, Node, NodeRef, ParseError, SyntaxTree},
    };

    #[derive(Node)]
    #[token(SimpleToken)]
    #[error(ParseError)]
    #[trivia($Whitespace)]
    enum TestNode {
        #[root]
        #[rule(sums: Sum*)]
        Root {
            #[node]
            node: NodeRef,
            #[parent]
            parent: NodeRef,
            #[child]
            sums: Vec<NodeRef>,
            #[semantics]
            semantics: Semantics<RootSemantics>,
        },

        #[rule($ParenOpen (numbers: $Number)+{$Symbol} $ParenClose)]
        Sum {
            #[node]
            node: NodeRef,
            #[parent]
            parent: NodeRef,
            #[child]
            numbers: Vec<TokenRef>,
            #[semantics]
            semantics: Semantics<Attr<NumSumAttr>>,
        },
    }

    #[derive(Feature)]
    #[node(TestNode)]
    struct RootSemantics {
        #[invalidate]
        total_sum: Attr<TotalSumAttr>,
    }

    #[derive(PartialEq, Eq)]
    struct TotalSumAttr {
        value: usize,
    }

    impl Computable for TotalSumAttr {
        type Node = TestNode;

        fn compute<S: SyncBuildHasher>(
            context: &mut AttrContext<Self::Node, S>,
        ) -> AnalysisResult<Self>
        where
            Self: Sized,
        {
            let doc = context
                .analyzer()
                .read_document(context.node_ref().id)
                .unwrap();

            let Some(TestNode::Root { sums, .. }) = context.node_ref().deref(doc.deref()) else {
                panic!()
            };

            let mut value = 0;

            for sum in sums {
                let Some(TestNode::Sum { semantics, .. }) = sum.deref(doc.deref()) else {
                    continue;
                };

                let sum = semantics
                    .attr_ref()
                    .query::<NumSumAttr, _>(context)
                    .unwrap();

                value += sum.sum
            }

            Ok(Self { value })
        }
    }

    #[derive(PartialEq, Eq)]
    struct NumSumAttr {
        sum: usize,
    }

    impl Computable for NumSumAttr {
        type Node = TestNode;

        fn compute<S: SyncBuildHasher>(
            context: &mut AttrContext<Self::Node, S>,
        ) -> AnalysisResult<Self>
        where
            Self: Sized,
        {
            let doc = context
                .analyzer()
                .read_document(context.node_ref().id)
                .unwrap();

            let Some(TestNode::Sum { numbers, .. }) = context.node_ref().deref(doc.deref()) else {
                panic!()
            };

            let mut sum = 0;

            for number in numbers {
                if let Some(number) = number.string(doc.deref()) {
                    if let Ok(number) = number.parse::<usize>() {
                        sum += number;
                    }
                }
            }

            Ok(Self { sum })
        }
    }

    #[test]
    fn test_analyzer() {
        let analyzer = Analyzer::<TestNode>::for_single_document();

        let id = {
            let handle = Latch::new();
            let mutation = analyzer.mutate(&handle).unwrap();

            mutation.add_mutable_document("(1+ 2) (8 + 2)")
        };

        {
            let handle = Latch::new();
            let analysis = analyzer.analyze(&handle).unwrap();

            let doc = analysis.analyzer().read_document(id).unwrap();

            assert!(doc.is_mutable());

            let root_node = doc.root_node_ref().deref(doc.deref()).unwrap();

            let total_sum = root_node
                .feature(Key::from("total_sum"))
                .unwrap()
                .attr_ref();

            let total_sum = analysis.read_attr_ref::<TotalSumAttr>(total_sum).unwrap();

            assert_eq!(total_sum.value, 13);
        }

        {
            let handle = Latch::new();
            let mutation = analyzer.mutate(&handle).unwrap();

            let _ = mutation.write_to_document(id, 4..5, "0 + 1").unwrap();
        }

        {
            let handle = Latch::new();
            let analysis = analyzer.analyze(&handle).unwrap();

            let doc = analysis.analyzer().read_document(id).unwrap();

            let Some(TestNode::Root { semantics, .. }) = doc.root_node_ref().deref(doc.deref())
            else {
                panic!();
            };

            let total_sum = &semantics.get().unwrap().total_sum;

            let total_sum = analysis.read_attr(total_sum).unwrap();

            assert_eq!(total_sum.value, 12);
        }
    }
}
