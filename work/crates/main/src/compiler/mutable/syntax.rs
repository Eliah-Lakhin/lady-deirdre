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

#[cfg(debug_assertions)]
use crate::syntax::ROOT_RULE;
use crate::{
    arena::{Entry, Id, Identifiable, Repository},
    compiler::mutable::storage::{ChildCursor, ClusterCache, References, Tree},
    lexis::{Length, Site, SiteRef, Token, TokenCount, TokenCursor, TokenRef},
    report::{debug_assert, debug_assert_eq},
    std::*,
    syntax::{Cluster, ErrorRef, NoSyntax, Node, NodeRef, NodeRule, SyntaxSession},
};

pub struct MutableSyntaxSession<'unit, N: Node> {
    id: Id,
    tree: &'unit mut Tree<N>,
    references: &'unit mut References<N>,
    updates: Option<&'unit mut StdSet<NodeRef>>,
    context: Vec<NodeRef>,
    pending: Pending<N>,
    failing: bool,
    next_chunk_cursor: ChildCursor<N>,
    next_site: Site,
    peek_chunk_cursor: ChildCursor<N>,
    peek_distance: TokenCount,
    peek_site: Site,
    peek_caches: usize,
    end_site: Site,
}

impl<'unit, N: Node> Identifiable for MutableSyntaxSession<'unit, N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'unit, N: Node> TokenCursor<'unit> for MutableSyntaxSession<'unit, N> {
    type Token = N::Token;

    fn advance(&mut self) -> bool {
        if self.next_chunk_cursor.is_dangling() {
            return false;
        }

        self.next_site += unsafe { self.next_chunk_cursor.span() };

        let has_cache = unsafe { self.next_chunk_cursor.cache().is_some() };

        if has_cache {
            match &self.pending.cluster_entry {
                Entry::Repo { index, .. }
                    if index == &unsafe { self.next_chunk_cursor.cache_index() } =>
                {
                    ()
                }

                _ => {
                    let entry_index = unsafe { self.next_chunk_cursor.remove_cache() };

                    unsafe { self.references.clusters_mut().remove_unchecked(entry_index) };
                }
            }
        }

        unsafe { self.next_chunk_cursor.next() };

        match self.peek_distance == 0 {
            true => {
                self.peek_chunk_cursor = self.next_chunk_cursor;
                self.peek_site = self.next_site;

                debug_assert!(self.peek_caches == 0, "Incorrect cache counter.");
            }

            false => {
                self.peek_distance -= 1;

                if has_cache {
                    self.peek_caches -= 1;
                }
            }
        }

        self.pending.leftmost = false;
        self.failing = false;

        true
    }

    fn skip(&mut self, mut distance: TokenCount) {
        if distance == 0 {
            return;
        }

        match self.peek_distance == distance {
            true => {
                while self.peek_caches > 0 {
                    #[allow(unused)]
                    let advanced = self.advance();

                    debug_assert!(advanced, "Skip advancing failure.");
                }

                self.next_chunk_cursor = self.peek_chunk_cursor;
                self.next_site = self.peek_site;
                self.peek_distance = 0;
                self.pending.leftmost = false;
                self.failing = false;
            }

            false => {
                while distance > 0 {
                    distance -= 1;

                    if !self.advance() {
                        break;
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn token(&mut self, distance: TokenCount) -> Self::Token {
        if unsafe { self.next_chunk_cursor.is_dangling() } {
            return <Self::Token as Token>::eoi();
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return <Self::Token as Token>::eoi();
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_cursor.span() });

        unsafe { self.peek_chunk_cursor.token() }
    }

    #[inline(always)]
    fn site(&mut self, distance: TokenCount) -> Option<Site> {
        if self.next_chunk_cursor.is_dangling() {
            return None;
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return None;
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_cursor.span() });

        Some(unsafe { self.tree.site_of(&self.peek_chunk_cursor) })
    }

    #[inline(always)]
    fn length(&mut self, distance: TokenCount) -> Option<Length> {
        if self.next_chunk_cursor.is_dangling() {
            return None;
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return None;
        }

        let span = unsafe { *self.peek_chunk_cursor.span() };

        self.pending.lookahead_end_site =
            self.pending.lookahead_end_site.max(self.peek_site + span);

        Some(span)
    }

    #[inline(always)]
    fn string(&mut self, distance: TokenCount) -> Option<&'unit str> {
        if self.next_chunk_cursor.is_dangling() {
            return None;
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return None;
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_cursor.span() });

        Some(unsafe { self.peek_chunk_cursor.string() })
    }

    #[inline(always)]
    fn token_ref(&mut self, distance: TokenCount) -> TokenRef {
        if self.next_chunk_cursor.is_dangling() {
            return TokenRef::nil();
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return TokenRef::nil();
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_cursor.span() });

        let entry_index = unsafe { self.peek_chunk_cursor.chunk_entry_index() };

        let chunk_entry = unsafe { self.references.chunks().entry_of(entry_index) };

        TokenRef {
            id: self.id,
            chunk_entry,
        }
    }

    #[inline(always)]
    fn site_ref(&mut self, distance: TokenCount) -> SiteRef {
        if self.next_chunk_cursor.is_dangling() {
            return self.end_site_ref();
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.length();
            return self.end_site_ref();
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_cursor.span() });

        let entry_index = unsafe { self.peek_chunk_cursor.chunk_entry_index() };

        let chunk_entry = unsafe { self.references.chunks().entry_of(entry_index) };

        TokenRef {
            id: self.id,
            chunk_entry,
        }
        .site_ref()
    }

    #[inline(always)]
    fn end_site_ref(&mut self) -> SiteRef {
        self.pending.lookahead_end_site = self.end_site;

        SiteRef::end_of(self.id)
    }
}

impl<'unit, N: Node> SyntaxSession<'unit> for MutableSyntaxSession<'unit, N> {
    type Node = N;

    fn descend(&mut self, rule: NodeRule) -> NodeRef {
        if self.pending.leftmost {
            let index = self.pending.nodes.reserve();

            let node_ref = NodeRef {
                id: self.id,
                cluster_entry: self.pending.cluster_entry,
                node_entry: unsafe { self.pending.nodes.entry_of(index) },
            };

            self.context.push(node_ref);

            let node = N::parse(self, rule);

            #[allow(unused)]
            let last = self.context.pop();

            #[cfg(debug_assertions)]
            if last != Some(node_ref) {
                panic!("Inheritance imbalance.");
            }

            unsafe { self.pending.nodes.set_unchecked(index, node) };

            if let Some(updates) = &mut self.updates {
                updates.insert(node_ref);
            }

            return node_ref;
        }

        if self.next_chunk_cursor.is_dangling() {
            return NodeRef::nil();
        }

        if let Some(cache) = unsafe { self.next_chunk_cursor.cache_mut() } {
            if cache.successful && cache.rule == rule {
                cache.cluster.primary.set_parent_ref(self.node_ref());

                let cluster_entry_index = unsafe { self.next_chunk_cursor.cache_index() };

                let result = NodeRef {
                    id: self.id,
                    cluster_entry: unsafe {
                        self.references.clusters().entry_of(cluster_entry_index)
                    },
                    node_entry: Entry::Primary,
                };

                let (end_site, end_chunk_cursor) =
                    unsafe { cache.jump_to_end(self.tree, self.references) };

                self.pending.lookahead_end_site = self
                    .pending
                    .lookahead_end_site
                    .max(end_site + cache.lookahead);
                self.pending.leftmost = false;

                self.next_chunk_cursor = end_chunk_cursor;
                self.next_site = end_site;
                self.peek_chunk_cursor = end_chunk_cursor;
                self.peek_distance = 0;
                self.peek_site = end_site;
                self.peek_caches = 0;

                if let Some(updates) = &mut self.updates {
                    updates.insert(result);
                }

                return result;
            }
        };

        let child_chunk_cursor = self.next_chunk_cursor;

        let cluster_entry_index;
        let cluster_entry;

        {
            let clusters = self.references.clusters_mut();

            cluster_entry_index = clusters.insert_raw(child_chunk_cursor);
            cluster_entry = unsafe { clusters.entry_of(cluster_entry_index) };
        };

        let parent = replace(
            &mut self.pending,
            Pending {
                lookahead_end_site: self.next_site,
                leftmost: true,
                cluster_entry,
                nodes: Repository::default(),
                errors: Repository::default(),
                successful: true,
            },
        );

        let node_ref = NodeRef {
            id: self.id,
            cluster_entry,
            node_entry: Entry::Primary,
        };

        self.context.push(node_ref);

        if let Some(updates) = &mut self.updates {
            updates.insert(node_ref);
        }

        let primary = N::parse(self, rule);

        #[allow(unused)]
        let last = self.context.pop();

        #[cfg(debug_assertions)]
        if last != Some(node_ref) {
            panic!("Inheritance imbalance.");
        }

        let child = replace(&mut self.pending, parent);

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(child.lookahead_end_site);

        let lookahead = child.lookahead_end_site - self.next_site;

        let parsed_end = self.parsed_end();

        #[allow(unused)]
        let previous_entry_index = unsafe {
            child_chunk_cursor.set_cache(
                cluster_entry_index,
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

        debug_assert!(previous_entry_index.is_none(), "Unreleased cache entry.");

        node_ref
    }

    #[inline(always)]
    fn enter_node(&mut self) -> NodeRef {
        let index = self.pending.nodes.reserve();

        let node_ref = NodeRef {
            id: self.id,
            cluster_entry: self.pending.cluster_entry,
            node_entry: unsafe { self.pending.nodes.entry_of(index) },
        };

        self.context.push(node_ref);

        if let Some(updates) = &mut self.updates {
            updates.insert(node_ref);
        }

        node_ref
    }

    #[inline(always)]
    fn leave_node(&mut self, node: Self::Node) -> NodeRef {
        #[cfg(debug_assertions)]
        if self.context.len() <= 2 {
            panic!("Inheritance imbalance.");
        }

        let node_ref = match self.context.pop() {
            None => panic!("Inheritance imbalance."),
            Some(node_ref) => node_ref,
        };

        if node_ref.cluster_entry != self.pending.cluster_entry {
            panic!("Inheritance imbalance.");
        }

        let index = match &node_ref.node_entry {
            Entry::Repo { index, .. } => *index,
            _ => panic!("Inheritance imbalance."),
        };

        unsafe { self.pending.nodes.set_unchecked(index, node) };

        node_ref
    }

    #[inline(always)]
    fn node(&mut self, node: Self::Node) -> NodeRef {
        let node_entry = self.pending.nodes.insert(node);

        NodeRef {
            id: self.id,
            cluster_entry: self.pending.cluster_entry,
            node_entry,
        }
    }

    fn lift_sibling(&mut self, sibling_ref: &NodeRef) {
        #[cfg(debug_assertions)]
        if sibling_ref.id != self.id {
            panic!("An attempt to lift external Node.");
        }

        let node_ref = self.node_ref();

        if self.pending.cluster_entry == sibling_ref.cluster_entry {
            if let Some(sibling) = self.pending.nodes.get_mut(&sibling_ref.node_entry) {
                sibling.set_parent_ref(node_ref);

                if let Some(updates) = &mut self.updates {
                    updates.insert(*sibling_ref);
                }
                return;
            }

            panic!("An attempt to lift non-sibling Node.");
        }

        if let Entry::Primary = &sibling_ref.node_entry {
            if let Some(cursor) = self
                .references
                .clusters_mut()
                .get_mut(&sibling_ref.cluster_entry)
            {
                if let Some(cache) = unsafe { cursor.cache_mut() } {
                    cache.cluster.primary.set_parent_ref(node_ref);

                    if let Some(updates) = &mut self.updates {
                        updates.insert(*sibling_ref);
                    }
                    return;
                }
            }
        }

        panic!("An attempt to lift non-sibling Node.");
    }

    #[inline(always)]
    fn node_ref(&self) -> NodeRef {
        match self.context.last() {
            Some(node_ref) => *node_ref,
            None => panic!("Inheritance imbalance."),
        }
    }

    #[inline(always)]
    fn parent_ref(&self) -> NodeRef {
        match self.context.len().checked_sub(2) {
            None => return NodeRef::nil(),
            Some(depth) => *unsafe { self.context.get_unchecked(depth) },
        }
    }

    #[inline(always)]
    fn failure(&mut self, error: impl Into<<Self::Node as Node>::Error>) -> ErrorRef {
        self.pending.successful = false;

        if !self.failing {
            self.failing = true;

            return ErrorRef {
                id: self.id,
                cluster_entry: self.pending.cluster_entry,
                error_entry: self.pending.errors.insert(error.into()),
            };
        }

        ErrorRef::nil()
    }
}

impl<'unit, N: Node> MutableSyntaxSession<'unit, N> {
    // Safety:
    // 1. `head` belongs to the `tree` instance.
    // 2. All references of the `tree` belong to `references` instance.
    pub(super) unsafe fn run(
        id: Id,
        tree: &'unit mut Tree<N>,
        references: &'unit mut References<N>,
        updates: Option<&'unit mut StdSet<NodeRef>>,
        rule: NodeRule,
        start: Site,
        head: ChildCursor<N>,
        cluster_entry: Entry,
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
            cluster_entry,
            nodes: Repository::default(),
            errors: Repository::default(),
            successful: true,
        };

        let context = {
            let parent_ref = match head.is_dangling() {
                true => NodeRef::nil(),
                false => match unsafe { head.cache() } {
                    None => NodeRef::nil(),
                    Some(cache) => cache.cluster.primary.parent_ref(),
                },
            };

            let node_ref = NodeRef {
                id,
                cluster_entry,
                node_entry: Entry::Primary,
            };

            let mut context = Vec::with_capacity(10);

            context.push(parent_ref);
            context.push(node_ref);

            context
        };

        let length = tree.length();

        let mut session = Self {
            id,
            tree,
            references,
            updates,
            context,
            pending,
            failing: false,
            next_chunk_cursor: head,
            next_site: start,
            peek_chunk_cursor: head,
            peek_distance: 0,
            peek_site: start,
            peek_caches: 0,
            end_site: length,
        };

        let primary = N::parse(&mut session, rule);
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
        match self.next_chunk_cursor.is_dangling() {
            false => {
                let chunk_entry_index = unsafe { self.next_chunk_cursor.chunk_entry_index() };
                let chunk_entry = unsafe { self.references.chunks().entry_of(chunk_entry_index) };

                TokenRef {
                    id: self.id,
                    chunk_entry,
                }
                .site_ref()
            }

            true => SiteRef::end_of(self.id),
        }
    }

    // Returns `true` if jump has failed.
    // Safety: `self.next_chunk_cursor` is not dangling.
    #[inline]
    unsafe fn jump(&mut self, target: TokenCount) -> bool {
        while self.peek_distance < target {
            self.peek_distance += 1;
            self.peek_site += unsafe { *self.peek_chunk_cursor.span() };

            let has_cache = unsafe { self.peek_chunk_cursor.cache().is_some() };

            if has_cache {
                self.peek_caches += 1;
            };

            unsafe { self.peek_chunk_cursor.next() };

            if unsafe { self.peek_chunk_cursor.is_dangling() } {
                self.peek_distance = 0;
                self.peek_site = self.next_site;
                self.peek_chunk_cursor = self.next_chunk_cursor;
                return true;
            }
        }

        if self.peek_distance > target * 2 {
            self.peek_distance = 0;
            self.peek_site = self.next_site;
            self.peek_chunk_cursor = self.next_chunk_cursor;
            self.peek_caches = 0;

            while self.peek_distance < target {
                self.peek_distance += 1;
                self.peek_site += unsafe { *self.peek_chunk_cursor.span() };

                let has_cache = unsafe { self.peek_chunk_cursor.cache().is_some() };

                if has_cache {
                    self.peek_caches += 1;
                };

                unsafe { self.peek_chunk_cursor.next() };

                debug_assert!(!self.peek_chunk_cursor.is_dangling(), "Dangling peek ref.");
            }

            return false;
        }

        while self.peek_distance > target {
            let has_cache = unsafe { self.peek_chunk_cursor.cache().is_some() };

            if has_cache {
                self.peek_caches -= 1;
            };

            unsafe { self.peek_chunk_cursor.back() }

            debug_assert!(!self.peek_chunk_cursor.is_dangling(), "Dangling peek ref.");

            self.peek_distance -= 1;
            self.peek_site -= unsafe { *self.peek_chunk_cursor.span() };
        }

        false
    }
}

struct Pending<N: Node> {
    lookahead_end_site: Site,
    leftmost: bool,
    cluster_entry: Entry,
    nodes: Repository<N>,
    errors: Repository<N::Error>,
    successful: bool,
}
