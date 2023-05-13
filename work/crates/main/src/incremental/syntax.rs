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
    arena::{Id, Identifiable, Ref, Repository},
    incremental::storage::{ChildRefIndex, ClusterCache, References, Tree},
    lexis::{Length, Site, SiteRef, TokenCount, TokenCursor, TokenRef},
    report::{debug_assert, debug_assert_eq},
    std::*,
    syntax::{Cluster, ErrorRef, NoSyntax, Node, NodeRef, SyntaxRule, SyntaxSession, ROOT_RULE},
};

pub struct IncrementalSyntaxSession<'document, N: Node> {
    id: Id,
    tree: &'document mut Tree<N>,
    references: &'document mut References<N>,
    pending: Pending<N>,
    next_chunk_ref: ChildRefIndex<N>,
    next_site: Site,
    peek_chunk_ref: ChildRefIndex<N>,
    peek_distance: TokenCount,
    peek_site: Site,
}

impl<'document, N: Node> Identifiable for IncrementalSyntaxSession<'document, N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'document, N: Node> TokenCursor<'document> for IncrementalSyntaxSession<'document, N> {
    type Token = N::Token;

    #[inline(always)]
    fn advance(&mut self) -> bool {
        if self.next_chunk_ref.is_dangling() {
            return false;
        }

        self.next_site += unsafe { self.next_chunk_ref.span() };

        unsafe { self.next_chunk_ref.next() };

        match self.peek_distance == 0 {
            true => {
                self.peek_chunk_ref = self.next_chunk_ref;
                self.peek_site = self.next_site;
            }

            false => {
                self.peek_distance -= 1;
            }
        }

        self.pending.leftmost = false;

        true
    }

    #[inline(always)]
    fn token(&mut self, distance: TokenCount) -> Option<&'document Self::Token> {
        if unsafe { self.next_chunk_ref.is_dangling() } {
            return None;
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return None;
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_ref.span() });

        Some(unsafe { self.peek_chunk_ref.token() })
    }

    #[inline(always)]
    fn site(&mut self, distance: TokenCount) -> Option<Site> {
        if self.next_chunk_ref.is_dangling() {
            return None;
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return None;
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_ref.span() });

        Some(unsafe { self.tree.site_of(&self.peek_chunk_ref) })
    }

    #[inline(always)]
    fn length(&mut self, distance: TokenCount) -> Option<Length> {
        if self.next_chunk_ref.is_dangling() {
            return None;
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return None;
        }

        let span = unsafe { *self.peek_chunk_ref.span() };

        self.pending.lookahead_end_site =
            self.pending.lookahead_end_site.max(self.peek_site + span);

        Some(span)
    }

    #[inline(always)]
    fn string(&mut self, distance: TokenCount) -> Option<&'document str> {
        if self.next_chunk_ref.is_dangling() {
            return None;
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return None;
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_ref.span() });

        Some(unsafe { self.peek_chunk_ref.string() })
    }

    #[inline(always)]
    fn token_ref(&mut self, distance: TokenCount) -> TokenRef {
        if self.next_chunk_ref.is_dangling() {
            return TokenRef::nil();
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return TokenRef::nil();
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_ref.span() });

        let ref_index = unsafe { self.peek_chunk_ref.chunk_ref_index() };

        let chunk_ref = unsafe { self.references.chunks().make_ref(ref_index) };

        TokenRef {
            id: self.id,
            chunk_ref,
        }
    }

    #[inline(always)]
    fn site_ref(&mut self, distance: TokenCount) -> SiteRef {
        if self.next_chunk_ref.is_dangling() {
            return self.end_site_ref();
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return self.end_site_ref();
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_ref.span() });

        let ref_index = unsafe { self.peek_chunk_ref.chunk_ref_index() };

        let chunk_ref = unsafe { self.references.chunks().make_ref(ref_index) };

        TokenRef {
            id: self.id,
            chunk_ref,
        }
        .site_ref()
    }

    #[inline(always)]
    fn end_site_ref(&mut self) -> SiteRef {
        SiteRef::new_code_end(self.id)
    }
}

impl<'document, N: Node> SyntaxSession<'document> for IncrementalSyntaxSession<'document, N> {
    type Node = N;

    fn descend(&mut self, rule: SyntaxRule) -> NodeRef {
        if self.pending.leftmost {
            let node = N::new(rule, self);

            let node_ref = self.pending.nodes.insert(node);

            return NodeRef {
                id: self.id,
                cluster_ref: self.pending.cluster_ref,
                node_ref,
            };
        }

        if self.next_chunk_ref.is_dangling() {
            return NodeRef::nil();
        }

        if let Some(cache) = unsafe { self.next_chunk_ref.cache() } {
            if cache.successful && cache.rule == rule {
                let cluster_ref_index = unsafe { self.next_chunk_ref.cache_index() };

                let result = NodeRef {
                    id: self.id,
                    cluster_ref: unsafe { self.references.clusters().make_ref(cluster_ref_index) },
                    node_ref: Ref::Primary,
                };

                let (end_site, end_chunk_ref) =
                    unsafe { cache.jump_to_end(self.tree, self.references) };

                self.pending.lookahead_end_site = self
                    .pending
                    .lookahead_end_site
                    .max(end_site + cache.lookahead);
                self.pending.leftmost = false;

                self.next_chunk_ref = end_chunk_ref;
                self.next_site = end_site;
                self.peek_chunk_ref = end_chunk_ref;
                self.peek_distance = 0;
                self.peek_site = end_site;

                return result;
            }
        };

        let child_chunk_ref = self.next_chunk_ref;

        let cluster_ref_index;
        let cluster_ref;

        {
            let clusters = self.references.clusters_mut();

            cluster_ref_index = clusters.insert_index(child_chunk_ref);
            cluster_ref = unsafe { clusters.make_ref(cluster_ref_index) };
        };

        let parent = replace(
            &mut self.pending,
            Pending {
                lookahead_end_site: self.next_site,
                leftmost: true,
                cluster_ref,
                nodes: Repository::default(),
                errors: Repository::default(),
                successful: true,
            },
        );

        let primary = N::new(rule, self);

        let child = replace(&mut self.pending, parent);

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(child.lookahead_end_site);

        let lookahead = child.lookahead_end_site - self.next_site;

        let parsed_end = self.parsed_end();

        let previous_ref_index = unsafe {
            child_chunk_ref.set_cache(
                cluster_ref_index,
                ClusterCache {
                    cluster: Cluster {
                        primary,
                        nodes: child.nodes,
                        errors: child.errors,
                    },
                    rule,
                    parsed_end,
                    lookahead,
                    successful: child.successful,
                },
            )
        };

        if let Some(previous_ref_index) = previous_ref_index {
            unsafe {
                self.references
                    .clusters_mut()
                    .remove_unchecked(previous_ref_index)
            };
        }

        NodeRef {
            id: self.id,
            cluster_ref,
            node_ref: Ref::Primary,
        }
    }

    #[inline(always)]
    fn error(&mut self, error: <Self::Node as Node>::Error) -> ErrorRef {
        self.pending.successful = false;

        ErrorRef {
            id: self.id,
            cluster_ref: self.pending.cluster_ref,
            error_ref: self.pending.errors.insert(error),
        }
    }
}

impl<'document, N: Node> IncrementalSyntaxSession<'document, N> {
    // Safety:
    // 1. `head` belongs to the `tree` instance.
    // 2. All references of the `tree` belong to `references` instance.
    pub(super) unsafe fn run(
        id: Id,
        tree: &'document mut Tree<N>,
        references: &'document mut References<N>,
        rule: SyntaxRule,
        start: Site,
        head: ChildRefIndex<N>,
        cluster_ref: Ref,
    ) -> (ClusterCache<N>, Site, Length) {
        if TypeId::of::<N>() == TypeId::of::<NoSyntax<<N as Node>::Token>>() {
            debug_assert_eq!(rule, ROOT_RULE, "An attempt to reparse void syntax.",);

            return (
                ClusterCache {
                    cluster: Cluster {
                        primary: unsafe { MaybeUninit::zeroed().assume_init() },
                        nodes: Default::default(),
                        errors: Default::default(),
                    },
                    rule,
                    parsed_end: SiteRef::nil(),
                    lookahead: 0,
                    successful: true,
                },
                0,
                0,
            );
        }

        let pending = Pending {
            lookahead_end_site: start,
            leftmost: true,
            cluster_ref,
            nodes: Repository::default(),
            errors: Repository::default(),
            successful: true,
        };

        let mut session = Self {
            id,
            tree,
            references,
            pending,
            next_chunk_ref: head,
            next_site: start,
            peek_chunk_ref: head,
            peek_distance: 0,
            peek_site: start,
        };

        let primary = N::new(rule, &mut session);
        let parsed_end_site = session.next_site;
        let parsed_end = session.parsed_end();
        let lookahead = session.pending.lookahead_end_site - session.next_site;
        let successful = session.pending.successful;
        let nodes = session.pending.nodes;
        let errors = session.pending.errors;

        let cluster = Cluster {
            primary,
            nodes,
            errors,
        };

        let cluster_cache = ClusterCache {
            cluster,
            rule,
            parsed_end,
            lookahead,
            successful,
        };

        (cluster_cache, parsed_end_site, lookahead)
    }

    #[inline(always)]
    fn parsed_end(&self) -> SiteRef {
        match self.next_chunk_ref.is_dangling() {
            false => {
                let chunk_ref_index = unsafe { self.next_chunk_ref.chunk_ref_index() };
                let chunk_ref = unsafe { self.references.chunks().make_ref(chunk_ref_index) };

                TokenRef {
                    id: self.id,
                    chunk_ref,
                }
                .site_ref()
            }

            true => SiteRef::new_code_end(self.id),
        }
    }

    // Returns `true` if jump has failed.
    // Safety: `self.next_chunk_ref` is not dangling.
    #[inline]
    unsafe fn jump(&mut self, target: TokenCount) -> bool {
        while self.peek_distance < target {
            self.peek_distance += 1;
            self.peek_site += unsafe { *self.peek_chunk_ref.span() };

            unsafe { self.peek_chunk_ref.next() };

            if unsafe { self.peek_chunk_ref.is_dangling() } {
                self.peek_distance = 0;
                self.peek_site = self.next_site;
                self.peek_chunk_ref = self.next_chunk_ref;
                return true;
            }
        }

        if self.peek_distance > target * 2 {
            self.peek_distance = 0;
            self.peek_site = self.next_site;
            self.peek_chunk_ref = self.next_chunk_ref;

            while self.peek_distance < target {
                self.peek_distance += 1;
                self.peek_site += unsafe { *self.peek_chunk_ref.span() };

                unsafe { self.peek_chunk_ref.next() };

                debug_assert!(!self.peek_chunk_ref.is_dangling(), "Dangling peek ref.");
            }

            return false;
        }

        while self.peek_distance > target {
            unsafe { self.peek_chunk_ref.back() }

            debug_assert!(!self.peek_chunk_ref.is_dangling(), "Dangling peek ref.");

            self.peek_distance -= 1;
            self.peek_site -= unsafe { *self.peek_chunk_ref.span() };
        }

        false
    }
}

struct Pending<N: Node> {
    lookahead_end_site: Site,
    leftmost: bool,
    cluster_ref: Ref,
    nodes: Repository<N>,
    errors: Repository<N::Error>,
    successful: bool,
}
