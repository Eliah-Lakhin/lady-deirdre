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

use crate::{
    arena::EntryIndex,
    lexis::{Length, Site, SiteRef, SiteRefInner},
    report::debug_unreachable,
    std::*,
    syntax::{ErrorRef, Node, NodeRef, NodeRule},
    units::{
        storage::{ChildCursor, Tree, TreeRefs},
        Watch,
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
    pub(crate) fn free<N: Node>(self, refs: &mut TreeRefs<N>, watch: &mut impl Watch) {
        watch.report_node(&NodeRef {
            id: refs.id,
            entry: unsafe { refs.nodes.remove_unchecked(self.primary_node) },
        });

        for index in self.secondary_nodes {
            watch.report_node(&NodeRef {
                id: refs.id,
                entry: unsafe { refs.nodes.remove_unchecked(index) },
            });
        }

        for index in self.errors {
            watch.report_error(&ErrorRef {
                id: refs.id,
                entry: unsafe { refs.errors.remove_unchecked(index) },
            });
        }
    }

    #[inline(always)]
    pub(crate) fn free_inner<N: Node>(
        self,
        refs: &mut TreeRefs<N>,
        watch: &mut impl Watch,
    ) -> (NodeRule, EntryIndex) {
        watch.report_node(&NodeRef {
            id: refs.id,
            entry: unsafe { refs.nodes.entry_of_unchecked(self.primary_node) },
        });

        for index in self.secondary_nodes {
            watch.report_node(&NodeRef {
                id: refs.id,
                entry: unsafe { refs.nodes.remove_unchecked(index) },
            });
        }

        for index in self.errors {
            watch.report_error(&ErrorRef {
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
                    unsafe { debug_unreachable!("Incorrect cache end site Ref type.") }
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
