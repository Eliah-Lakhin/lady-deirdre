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
        Computable,
        Feature,
        Slot,
        TaskHandle,
    },
    arena::{Id, Identifiable},
    sync::SyncBuildHasher,
    syntax::{NodeRef, PolyRef, SyntaxTree},
};
use log::debug;

use crate::multi_modules::syntax::MultiModulesNode;

#[derive(Feature)]
#[node(MultiModulesNode)]
pub struct CommonSemantics {
    pub modules: Slot<MultiModulesNode, HashMap<String, Id>>,
}

#[derive(Feature)]
#[node(MultiModulesNode)]
pub struct ModuleSemantics {
    #[scoped]
    pub tables: Attr<ModuleTables>,
}

#[derive(Default, Clone, PartialEq, Eq)]
pub struct ModuleTables {
    pub key_table: HashMap<NodeRef, String>,
    pub def_table: HashMap<String, Definition>,
}

impl Computable for ModuleTables {
    type Node = MultiModulesNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr::<Self, H, S>(context)?;

        let root_ref = context.node_ref();
        let doc_read = context.read_doc(root_ref.id).unwrap_abnormal()?;
        let doc = doc_read.deref();

        let Some(MultiModulesNode::Root { defs, .. }) = root_ref.deref(doc) else {
            return Ok(Self::default());
        };

        let mut key_table = HashMap::new();
        let mut def_table = HashMap::new();

        for def_ref in defs {
            let Some(MultiModulesNode::Def { key, value, .. }) = def_ref.deref(doc) else {
                continue;
            };

            let Some(MultiModulesNode::Key { token, .. }) = key.deref(doc) else {
                continue;
            };

            let Some(key_string) = token.string(doc) else {
                continue;
            };

            let _ = key_table.insert(*key, key_string.to_string());

            match value.deref(doc) {
                Some(MultiModulesNode::Num { token, .. }) => {
                    let Some(num_string) = token.string(doc) else {
                        continue;
                    };

                    let Ok(value) = num_string.parse::<usize>() else {
                        continue;
                    };

                    let _ = def_table.insert(key_string.to_string(), Definition::Num { value });
                }

                Some(MultiModulesNode::Ref { module, ident, .. }) => {
                    let Some(module_string) = module.string(doc) else {
                        continue;
                    };

                    let Some(ident_string) = ident.string(doc) else {
                        continue;
                    };

                    let _ = def_table.insert(
                        key_string.to_string(),
                        Definition::Ref {
                            module: module_string.to_string(),
                            key: ident_string.to_string(),
                        },
                    );
                }

                _ => continue,
            }
        }

        Ok(Self {
            key_table,
            def_table,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum Definition {
    Num { value: usize },
    Ref { module: String, key: String },
}

#[derive(Feature)]
#[node(MultiModulesNode)]
pub struct KeySemantics {
    pub resolution: Attr<KeyResolution>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum KeyResolution {
    Unresolved,
    Recusrive,
    Number(usize),
}

impl Computable for KeyResolution {
    type Node = MultiModulesNode;

    fn compute<H: TaskHandle, S: SyncBuildHasher>(
        context: &mut AttrContext<Self::Node, H, S>,
    ) -> AnalysisResult<Self> {
        log_attr::<Self, H, S>(context)?;

        let key_ref = context.node_ref();
        let doc_read = context.read_doc(key_ref.id).unwrap_abnormal()?;
        let module_doc = doc_read.deref();
        let root_ref = module_doc.root_node_ref();

        let Some(MultiModulesNode::Root { semantics, .. }) = root_ref.deref(module_doc) else {
            return Ok(Self::Unresolved);
        };

        let module_semantics = semantics.get().unwrap_abnormal()?;
        let module_tables = module_semantics.tables.read(context).unwrap_abnormal()?;

        let Some(key) = module_tables.key_table.get(key_ref) else {
            return Ok(Self::Unresolved);
        };

        let (ref_module_name, mut ref_key) = match module_tables.def_table.get(key) {
            Some(Definition::Num { value }) => return Ok(Self::Number(*value)),
            Some(Definition::Ref { module, key }) => (module, key.clone()),
            None => return Ok(Self::Unresolved),
        };

        let mut trace = HashSet::new();

        let _ = trace.insert((module_doc.id(), key.clone()));

        let modules = context.common().modules.read(context).unwrap_abnormal()?;

        let Some(mut ref_module_id) = modules.get(ref_module_name).copied() else {
            return Ok(Self::Unresolved);
        };

        loop {
            if !trace.insert((ref_module_id, ref_key.clone())) {
                return Ok(Self::Recusrive);
            }

            let doc_read = context.read_doc(ref_module_id).unwrap_abnormal()?;
            let module_doc = doc_read.deref();
            let root_ref = module_doc.root_node_ref();

            let Some(MultiModulesNode::Root { semantics, .. }) = root_ref.deref(module_doc) else {
                return Ok(Self::Unresolved);
            };

            let module_semantics = semantics.get().unwrap_abnormal()?;
            let module_tables = module_semantics.tables.read(context).unwrap_abnormal()?;

            let ref_module_name = match module_tables.def_table.get(&ref_key) {
                Some(Definition::Num { value }) => return Ok(Self::Number(*value)),
                Some(Definition::Ref { module, key }) => {
                    ref_key = key.clone();

                    module
                }
                None => return Ok(Self::Unresolved),
            };

            let modules = context.common().modules.read(context).unwrap_abnormal()?;

            match modules.get(ref_module_name) {
                Some(id) => ref_module_id = *id,
                None => return Ok(Self::Unresolved),
            };
        }
    }
}

fn log_attr<C: Any, H: TaskHandle, S: SyncBuildHasher>(
    context: &mut AttrContext<MultiModulesNode, H, S>,
) -> AnalysisResult<()> {
    let node_ref = context.node_ref();
    let doc_read = context.read_doc(node_ref.id).unwrap_abnormal()?;

    let name = type_name::<C>();
    let display = context.node_ref().display(doc_read.deref());

    debug!("Computing {name}.\n{display:#}");

    Ok(())
}
