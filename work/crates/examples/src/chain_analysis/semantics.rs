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

use std::{
    any::{type_name, Any},
    collections::{HashMap, HashSet},
    ops::Deref,
};

use lady_deirdre::{
    analysis::{
        AnalysisResult,
        AnalysisResultEx,
        Attr,
        AttrContext,
        Classifier,
        Computable,
        Feature,
        SharedComputable,
        TaskHandle,
    },
    sync::{Shared, SyncBuildHasher},
    syntax::{NodeRef, PolyRef},
    units::Document,
};
use log::debug;

use crate::chain_analysis::syntax::ChainNode;

#[derive(Feature)]
#[node(ChainNode)]
pub struct BlockSemantics {
    #[scoped]
    pub analysis: Attr<BlockAnalysis>,
    pub assignments: Attr<Shared<BlockAssignmentMap>>,
    pub blocks: Attr<Shared<BlockNamespaceMap>>,
    pub namespace: Attr<Shared<BlockNamespace>>,
}

#[derive(Feature)]
#[node(ChainNode)]
pub struct KeySemantics {
    pub local_resolution: Attr<Shared<LocalResolution>>,
    pub global_resolution: Attr<GlobalResolution>,
}

#[derive(Default, Clone, PartialEq, Eq)]
pub enum GlobalResolution {
    #[default]
    Broken,
    Resolved(usize),
}

impl Computable for GlobalResolution {
    type Node = ChainNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr::<Self, H, S>(context)?;

        let key_ref = context.node_ref();
        let doc_read = context.read_doc(key_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let Some(ChainNode::Key { semantics, .. }) = key_ref.deref(doc) else {
            return Ok(Self::default());
        };

        let key_semantics = semantics.get().unwrap_abnormal()?;

        let local_resolution = key_semantics
            .local_resolution
            .read(context)
            .unwrap_abnormal()?;

        let mut ref_name = match local_resolution.as_ref() {
            LocalResolution::Broken => return Ok(Self::Broken),
            LocalResolution::Resolved(num) => return Ok(Self::Resolved(*num)),
            LocalResolution::External(name) => String::from(name),
        };

        let mut block_ref = semantics
            .scope_attr()
            .unwrap_abnormal()?
            .read(context)
            .unwrap_abnormal()?
            .scope_ref;

        loop {
            let Some(ChainNode::Block { semantics, .. }) = block_ref.deref(doc) else {
                return Ok(Self::default());
            };

            let block_semantics = semantics.get().unwrap_abnormal()?;

            let block_namespace = block_semantics.namespace.read(context).unwrap_abnormal()?;

            match block_namespace.as_ref().namespace.get(&ref_name) {
                Some(LocalResolution::Broken) => return Ok(Self::Broken),
                Some(LocalResolution::Resolved(num)) => return Ok(Self::Resolved(*num)),
                Some(LocalResolution::External(name)) => ref_name = String::from(name),
                None => (),
            }

            block_ref = semantics
                .scope_attr()
                .unwrap_abnormal()?
                .read(context)
                .unwrap_abnormal()?
                .scope_ref;
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub enum LocalResolution {
    #[default]
    Broken,
    Resolved(usize),
    External(String),
}

impl SharedComputable for LocalResolution {
    type Node = ChainNode;

    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr::<Self, H, S>(context)?;

        let key_ref = context.node_ref();
        let doc_read = context.read_doc(key_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let Some(ChainNode::Key { semantics, .. }) = key_ref.deref(doc) else {
            return Ok(Shared::default());
        };

        let block_ref = semantics
            .scope_attr()
            .unwrap_abnormal()?
            .read(context)?
            .scope_ref;

        let Some(ChainNode::Block { semantics, .. }) = block_ref.deref(doc) else {
            return Ok(Shared::default());
        };

        let block_semantics = semantics.get().unwrap_abnormal()?;

        let assignments = block_semantics
            .assignments
            .read(context)
            .unwrap_abnormal()?;

        let Some(resolution) = assignments.as_ref().map.get(key_ref) else {
            return Ok(Shared::default());
        };

        Ok(resolution.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub struct BlockAnalysis {
    pub assignments: Shared<BlockAssignmentMap>,
    pub blocks: Shared<BlockNamespaceMap>,
}

impl Computable for BlockAnalysis {
    type Node = ChainNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr::<Self, H, S>(context)?;

        let block_ref = context.node_ref();
        let doc_read = context.read_doc(block_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let mut result = Self::default();

        let Some(ChainNode::Block { statements, .. }) = block_ref.deref(doc) else {
            return Ok(result);
        };

        let mut block_namespace = BlockNamespace::default();

        for st_ref in statements {
            context.proceed()?;

            match st_ref.deref(doc) {
                Some(ChainNode::Block { .. }) => {
                    result
                        .blocks
                        .get_mut()
                        .unwrap()
                        .map
                        .insert(*st_ref, Shared::new(block_namespace.clone()));
                }

                Some(ChainNode::Assignment { key, value, .. }) => {
                    let Some(ChainNode::Key {
                        token: key_token, ..
                    }) = key.deref(doc)
                    else {
                        continue;
                    };

                    let Some(key_string) = key_token.string(doc) else {
                        continue;
                    };

                    let local_resolution = loop {
                        break match value.deref(doc) {
                            Some(ChainNode::Num {
                                token: value_token, ..
                            }) => {
                                let Some(value_string) = value_token.string(doc) else {
                                    break LocalResolution::Broken;
                                };

                                let Ok(num) = value_string.parse::<usize>() else {
                                    break LocalResolution::Broken;
                                };

                                LocalResolution::Resolved(num)
                            }

                            Some(ChainNode::Ref {
                                token: value_token, ..
                            }) => {
                                let Some(value_string) = value_token.string(doc) else {
                                    break LocalResolution::Broken;
                                };

                                let Some(local_resolution) =
                                    block_namespace.namespace.get(value_string)
                                else {
                                    break LocalResolution::External(String::from(value_string));
                                };

                                local_resolution.clone()
                            }

                            _ => LocalResolution::Broken,
                        };
                    };

                    let key_string = String::from(key_string);

                    block_namespace
                        .namespace
                        .insert(key_string, local_resolution.clone());

                    result
                        .assignments
                        .get_mut()
                        .unwrap()
                        .map
                        .insert(*key, Shared::new(local_resolution));
                }

                _ => (),
            }
        }

        Ok(result)
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub struct BlockAssignmentMap {
    pub map: HashMap<NodeRef, Shared<LocalResolution>>,
}

impl SharedComputable for BlockAssignmentMap {
    type Node = ChainNode;

    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr::<Self, H, S>(context)?;

        let block_ref = context.node_ref();
        let doc_read = context.read_doc(block_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let Some(ChainNode::Block { semantics, .. }) = block_ref.deref(doc) else {
            return Ok(Shared::default());
        };

        let block_semantics = semantics.get().unwrap_abnormal()?;

        Ok(block_semantics.analysis.read(context)?.assignments.clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub struct BlockNamespaceMap {
    pub map: HashMap<NodeRef, Shared<BlockNamespace>>,
}

impl SharedComputable for BlockNamespaceMap {
    type Node = ChainNode;

    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr::<Self, H, S>(context)?;

        let block_ref = context.node_ref();
        let doc_read = context.read_doc(block_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let Some(ChainNode::Block { semantics, .. }) = block_ref.deref(doc) else {
            return Ok(Shared::default());
        };

        let block_semantics = semantics.get().unwrap_abnormal()?;

        Ok(block_semantics
            .analysis
            .read(context)
            .unwrap_abnormal()?
            .blocks
            .clone())
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub struct BlockNamespace {
    pub namespace: HashMap<String, LocalResolution>,
}

impl SharedComputable for BlockNamespace {
    type Node = ChainNode;

    fn compute_shared<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Shared<Self>> {
        log_attr::<Self, H, S>(context)?;

        let block_ref = context.node_ref();
        let doc_read = context.read_doc(block_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let Some(ChainNode::Block { semantics, .. }) = block_ref.deref(doc) else {
            return Ok(Shared::default());
        };

        let scope_ref = semantics
            .scope_attr()
            .unwrap_abnormal()?
            .read(context)?
            .scope_ref;

        let Some(ChainNode::Block { semantics, .. }) = scope_ref.deref(doc) else {
            return Ok(Shared::default());
        };

        let parent_block_semantics = semantics.get().unwrap_abnormal()?;

        let all_blocks = parent_block_semantics
            .blocks
            .read(context)
            .unwrap_abnormal()?;

        let Some(namespace) = all_blocks.as_ref().map.get(block_ref) else {
            return Ok(Shared::default());
        };

        Ok(namespace.clone())
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ChainNodeClass {
    AllKeys,
}

pub struct ChainNodeClassifier;

impl Classifier for ChainNodeClassifier {
    type Node = ChainNode;
    type Class = ChainNodeClass;

    fn classify<S: SyncBuildHasher>(
        doc: &Document<Self::Node>,
        node_ref: &NodeRef,
    ) -> HashSet<Self::Class, S> {
        let mut result = HashSet::with_hasher(S::default());

        let Some(node) = node_ref.deref(doc) else {
            return result;
        };

        match node {
            ChainNode::Key { .. } => {
                let _ = result.insert(ChainNodeClass::AllKeys);
            }

            _ => (),
        }

        result
    }
}

fn log_attr<C: Any, H: TaskHandle, S: SyncBuildHasher>(
    context: &mut AttrContext<ChainNode, H, S>,
) -> AnalysisResult<()> {
    let node_ref = context.node_ref();
    let doc_read = context.read_doc(node_ref.id).unwrap_abnormal()?;

    let name = type_name::<C>();
    let display = context.node_ref().display(doc_read.deref());

    debug!("Computing {name}.\n{display:#}");

    Ok(())
}
