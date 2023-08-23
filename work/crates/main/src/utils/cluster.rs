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
    arena::Entry,
    lexis::{SiteSpan, SourceCode, ToSpan},
    std::*,
    syntax::{ClusterRef, Node, NodeRef, SyntaxTree, TreeContent},
};

impl ClusterRef {
    #[inline(always)]
    pub fn primary_node_ref(&self) -> NodeRef {
        NodeRef {
            id: self.id,
            cluster_entry: self.cluster_entry,
            node_entry: Entry::Primary,
        }
    }

    #[inline(always)]
    pub fn site_span<N: Node>(
        &self,
        storage: &(impl SyntaxTree<Node = N> + SourceCode<Token = N::Token>),
    ) -> Option<SiteSpan> {
        if self.id != storage.id() {
            return None;
        }

        let span = storage.get_cluster_span(&self.cluster_entry);

        span.to_span(storage)
    }

    pub fn parent<N: Node>(
        &self,
        storage: &(impl SyntaxTree<Node = N> + SourceCode<Token = N::Token>),
    ) -> Self {
        let self_span = match self.site_span(storage) {
            Some(span) => span,
            None => return Self::nil(),
        };

        let mut probe = *self;

        loop {
            probe = probe.previous(storage);

            if probe.is_nil() {
                return storage.root_cluster_ref();
            }

            let span = match probe.site_span(storage) {
                Some(span) => span,
                None => continue,
            };

            if span.start <= self_span.start && span.end >= self_span.end {
                return probe;
            }
        }
    }

    pub fn children<'storage, N: Node>(
        &self,
        storage: &'storage (impl SyntaxTree<Node = N> + SourceCode<Token = N::Token>),
    ) -> impl Iterator<Item = Self> + 'storage {
        struct ChildrenIterator<
            'storage,
            N: Node,
            S: SyntaxTree<Node = N> + SourceCode<Token = N::Token>,
        > {
            storage: &'storage S,
            cover: SiteSpan,
            probe: ClusterRef,
        }

        impl<'storage, N, S> Iterator for ChildrenIterator<'storage, N, S>
        where
            N: Node,
            S: SyntaxTree<Node = N> + SourceCode<Token = N::Token>,
        {
            type Item = ClusterRef;

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    self.probe = self.probe.next(self.storage);

                    if self.probe.is_nil() {
                        return None;
                    }

                    let span = match self.probe.site_span(self.storage) {
                        Some(span) => span,
                        None => {
                            self.probe = ClusterRef::nil();
                            return None;
                        }
                    };

                    if span.end <= self.cover.start {
                        continue;
                    }

                    if span.start >= self.cover.end {
                        self.probe = ClusterRef::nil();
                        return None;
                    }

                    self.cover.start = span.end;

                    return Some(self.probe);
                }
            }
        }

        impl<'storage, N, S> FusedIterator for ChildrenIterator<'storage, N, S>
        where
            N: Node,
            S: SyntaxTree<Node = N> + SourceCode<Token = N::Token>,
        {
        }

        match self.site_span(storage) {
            Some(cover) => ChildrenIterator {
                storage,
                cover,
                probe: *self,
            },
            None => ChildrenIterator {
                storage,
                cover: Default::default(),
                probe: ClusterRef::nil(),
            },
        }
    }
}
