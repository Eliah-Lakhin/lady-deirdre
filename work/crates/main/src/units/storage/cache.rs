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

use crate::{
    arena::EntryIndex,
    lexis::{Length, Site, SiteRef, SiteRefInner},
    report::ld_unreachable,
    syntax::{ErrorRef, Node, NodeRef, NodeRule},
    units::{
        storage::{ChildCursor, Tree, TreeRefs},
        Watcher,
    },
};

pub(crate) struct Cache {
    pub(crate) rule: NodeRule,
    pub(crate) parse_end: SiteRef,
    pub(crate) lookahead: Length,
    pub(crate) primary_node: EntryIndex,
    pub(crate) secondary_nodes: Vec<EntryIndex>,
    pub(crate) errors: Vec<EntryIndex>,
}

impl Cache {
    #[inline(always)]
    pub(crate) fn free<N: Node>(self, refs: &mut TreeRefs<N>, watcher: &mut impl Watcher) {
        watcher.report_node(&NodeRef {
            id: refs.id,
            entry: unsafe { refs.nodes.remove_unchecked(self.primary_node) },
        });

        for index in self.secondary_nodes {
            watcher.report_node(&NodeRef {
                id: refs.id,
                entry: unsafe { refs.nodes.remove_unchecked(index) },
            });
        }

        for index in self.errors {
            watcher.report_error(&ErrorRef {
                id: refs.id,
                entry: unsafe { refs.errors.remove_unchecked(index) },
            });
        }
    }

    #[inline(always)]
    pub(crate) fn free_inner<N: Node>(
        self,
        refs: &mut TreeRefs<N>,
        watcher: &mut impl Watcher,
    ) -> (NodeRule, EntryIndex) {
        watcher.report_node(&NodeRef {
            id: refs.id,
            entry: unsafe { refs.nodes.entry_of_unchecked(self.primary_node) },
        });

        for index in self.secondary_nodes {
            watcher.report_node(&NodeRef {
                id: refs.id,
                entry: unsafe { refs.nodes.remove_unchecked(index) },
            });
        }

        for index in self.errors {
            watcher.report_error(&ErrorRef {
                id: refs.id,
                entry: unsafe { refs.errors.remove_unchecked(index) },
            });
        }

        (self.rule, self.primary_node)
    }

    // Safety:
    // 1. Cache belongs to specified `tree` and `refs` pair.
    #[inline(always)]
    pub(crate) unsafe fn jump_to_end<N: Node>(
        &self,
        tree: &Tree<N>,
        refs: &TreeRefs<N>,
    ) -> (Site, ChildCursor<N>) {
        match self.parse_end.inner() {
            SiteRefInner::ChunkStart(token_ref) => {
                if token_ref.entry.version == 0 {
                    // Safety: Chunks stored in Repository.
                    unsafe { ld_unreachable!("Incorrect cache end site Ref type.") }
                }

                let chunk_entry_index = token_ref.entry.index;

                let chunk_cursor = unsafe { refs.chunks.get_unchecked(chunk_entry_index) };

                let site = unsafe { tree.site_of(chunk_cursor) };

                (site, *chunk_cursor)
            }

            SiteRefInner::CodeEnd(_) => (tree.code_length(), ChildCursor::dangling()),
        }
    }

    // Safety:
    // 1. Cache belongs to specified `tree` and `refs` pair.
    #[inline(always)]
    pub(crate) unsafe fn end_site<N: Node>(
        &self,
        tree: &Tree<N>,
        refs: &TreeRefs<N>,
    ) -> Option<Site> {
        match self.parse_end.inner() {
            SiteRefInner::ChunkStart(token_ref) => {
                let chunk_cursor = refs.chunks.get(&token_ref.entry)?;

                Some(unsafe { tree.site_of(chunk_cursor) })
            }

            SiteRefInner::CodeEnd(_) => Some(tree.code_length()),
        }
    }
}
