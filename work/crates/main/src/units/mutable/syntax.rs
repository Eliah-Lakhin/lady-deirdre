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
    arena::{Entry, EntryIndex, Id, Identifiable},
    lexis::{Length, Site, SiteRef, Token, TokenCount, TokenCursor, TokenRef},
    report::{debug_assert, debug_unreachable},
    std::*,
    syntax::{is_void_syntax, ErrorRef, Node, NodeRef, NodeRule, SyntaxSession, ROOT_RULE},
    units::{
        storage::{Cache, ChildCursor, Tree, TreeRefs},
        Watch,
    },
};

pub struct MutableSyntaxSession<'unit, N: Node, W: Watch> {
    tree: &'unit mut Tree<N>,
    refs: &'unit mut TreeRefs<N>,
    watch: &'unit mut W,
    context: Vec<Entry>,
    pending: Pending,
    failing: bool,
    next_chunk_cursor: ChildCursor<N>,
    next_site: Site,
    peek_chunk_cursor: ChildCursor<N>,
    peek_distance: TokenCount,
    peek_site: Site,
    peek_caches: usize,
    end_site: Site,
}

impl<'unit, N: Node, W: Watch> Identifiable for MutableSyntaxSession<'unit, N, W> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.refs.id
    }
}

impl<'unit, N: Node, W: Watch> TokenCursor<'unit> for MutableSyntaxSession<'unit, N, W> {
    type Token = N::Token;

    fn advance(&mut self) -> bool {
        if self.next_chunk_cursor.is_dangling() {
            return false;
        }

        self.next_site += unsafe { self.next_chunk_cursor.span() };

        let has_cache = unsafe { self.next_chunk_cursor.cache() }.is_some();

        if has_cache {
            let cache = unsafe { self.next_chunk_cursor.release_cache() };

            cache.free(self.refs, self.watch)
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
            self.pending.lookahead_end_site = self.tree.code_length();
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
            self.pending.lookahead_end_site = self.tree.code_length();
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
            self.pending.lookahead_end_site = self.tree.code_length();
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
            self.pending.lookahead_end_site = self.tree.code_length();
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
            self.pending.lookahead_end_site = self.tree.code_length();
            return TokenRef::nil();
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_cursor.span() });

        let entry_index = unsafe { self.peek_chunk_cursor.chunk_entry_index() };

        let chunk_entry = unsafe { self.refs.chunks.entry_of_unchecked(entry_index) };

        TokenRef {
            id: self.id(),
            entry: chunk_entry,
        }
    }

    #[inline(always)]
    fn site_ref(&mut self, distance: TokenCount) -> SiteRef {
        if self.next_chunk_cursor.is_dangling() {
            return self.end_site_ref();
        }

        if unsafe { self.jump(distance) } {
            self.pending.lookahead_end_site = self.tree.code_length();
            return self.end_site_ref();
        }

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(self.peek_site + unsafe { *self.peek_chunk_cursor.span() });

        let entry_index = unsafe { self.peek_chunk_cursor.chunk_entry_index() };

        let chunk_entry = unsafe { self.refs.chunks.entry_of_unchecked(entry_index) };

        TokenRef {
            id: self.id(),
            entry: chunk_entry,
        }
        .site_ref()
    }

    #[inline(always)]
    fn end_site_ref(&mut self) -> SiteRef {
        self.pending.lookahead_end_site = self.end_site;

        SiteRef::end_of(self.id())
    }
}

impl<'unit, N: Node, W: Watch> SyntaxSession<'unit> for MutableSyntaxSession<'unit, N, W> {
    type Node = N;

    fn descend(&mut self, rule: NodeRule) -> NodeRef {
        if self.pending.leftmost {
            let _ = self.enter(rule);
            let node = N::parse(self, rule);
            return self.leave(node);
        }

        if self.next_chunk_cursor.is_dangling() {
            return NodeRef::nil();
        }

        if let Some(cache) = unsafe { self.next_chunk_cursor.cache() } {
            if cache.errors.is_empty() && cache.rule == rule {
                let (end_site, end_chunk_cursor) =
                    unsafe { cache.jump_to_end(self.tree, self.refs) };

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

                {
                    let parent_ref = self.node_ref();

                    unsafe { self.refs.nodes.get_unchecked_mut(cache.primary_node) }
                        .set_parent_ref(parent_ref);
                }

                let result = NodeRef {
                    id: self.id(),
                    entry: unsafe { self.refs.nodes.entry_of_unchecked(cache.primary_node) },
                };

                self.watch.report_node(&result);

                return result;
            }

            let _ = cache;

            let cache = unsafe { self.next_chunk_cursor.release_cache() };

            cache.free(self.refs, self.watch);
        };

        let inner_start_cursor = self.next_chunk_cursor;

        let entry_index = self.refs.nodes.reserve_entry();
        let entry = unsafe { self.refs.nodes.entry_of_unchecked(entry_index) };

        let outer = replace(
            &mut self.pending,
            Pending {
                lookahead_end_site: self.next_site,
                leftmost: true,
                primary_node: entry_index,
                secondary_nodes: Vec::new(),
                errors: Vec::new(),
            },
        );

        let node_ref = NodeRef {
            id: self.id(),
            entry,
        };

        self.watch.report_node(&node_ref);

        self.context.push(entry);

        let node = N::parse(self, rule);

        #[allow(unused)]
        let last = self.context.pop();

        #[cfg(debug_assertions)]
        if last != Some(entry) {
            panic!("Nesting imbalance.");
        }

        let inner = replace(&mut self.pending, outer);

        self.pending.lookahead_end_site = self
            .pending
            .lookahead_end_site
            .max(inner.lookahead_end_site);

        let parse_end = self.parse_end();
        let next_site = self.next_site;

        let cache = unsafe { inner.into_cache(self.refs, rule, node, parse_end, next_site) };

        unsafe { inner_start_cursor.install_cache(cache) };

        node_ref
    }

    #[inline(always)]
    fn enter(&mut self, _rule: NodeRule) -> NodeRef {
        let entry_index = self.refs.nodes.reserve_entry();

        self.pending.secondary_nodes.push(entry_index);

        let entry = unsafe { self.refs.nodes.entry_of_unchecked(entry_index) };

        self.context.push(entry);

        let node_ref = NodeRef {
            id: self.id(),
            entry,
        };

        self.watch.report_node(&node_ref);

        node_ref
    }

    #[inline(always)]
    fn leave(&mut self, node: Self::Node) -> NodeRef {
        #[cfg(debug_assertions)]
        if self.context.len() <= 2 {
            panic!("Nesting imbalance.");
        }

        let Some(entry) = self.context.pop() else {
            #[cfg(debug_assertions)]
            {
                panic!("Nesting imbalance.");
            }

            #[cfg(not(debug_assertions))]
            {
                return NodeRef::nil();
            }
        };

        unsafe { self.refs.nodes.set_unchecked(entry.index, node) };

        NodeRef {
            id: self.id(),
            entry,
        }
    }

    #[inline(always)]
    fn lift(&mut self, node_ref: &NodeRef) {
        if node_ref.id != self.id() {
            #[cfg(debug_assertions)]
            {
                panic!("Cannot lift a node that does not belong to this compilation session.");
            }

            #[cfg(not(debug_assertions))]
            {
                return;
            }
        }

        let parent_ref = self.node_ref();

        let Some(node) = self.refs.nodes.get_mut(&node_ref.entry) else {
            #[cfg(debug_assertions)]
            {
                panic!("Cannot lift a node that does not belong to this compilation session.");
            }

            #[cfg(not(debug_assertions))]
            {
                return;
            }
        };

        node.set_parent_ref(parent_ref);

        self.watch.report_node(node_ref);
    }

    #[inline(always)]
    fn node_ref(&self) -> NodeRef {
        let Some(entry) = self.context.last() else {
            #[cfg(debug_assertions)]
            {
                panic!("Nesting imbalance.");
            }

            #[cfg(not(debug_assertions))]
            {
                return NodeRef::nil();
            }
        };

        if entry.is_nil() {
            return NodeRef::nil();
        }

        NodeRef {
            id: self.id(),
            entry: *entry,
        }
    }

    #[inline(always)]
    fn parent_ref(&self) -> NodeRef {
        let Some(depth) = self.context.len().checked_sub(2) else {
            #[cfg(debug_assertions)]
            {
                panic!("Nesting imbalance.");
            }

            #[cfg(not(debug_assertions))]
            {
                return NodeRef::nil();
            }
        };

        let entry = unsafe { self.context.get_unchecked(depth) };

        if entry.is_nil() {
            return NodeRef::nil();
        }

        NodeRef {
            id: self.id(),
            entry: *entry,
        }
    }

    #[inline(always)]
    fn failure(&mut self, error: impl Into<<Self::Node as Node>::Error>) -> ErrorRef {
        if self.failing {
            return ErrorRef::nil();
        }

        self.failing = true;

        let entry_index = self.refs.errors.insert_raw(error.into());

        self.pending.errors.push(entry_index);

        let error_ref = ErrorRef {
            id: self.id(),
            entry: unsafe { self.refs.errors.entry_of_unchecked(entry_index) },
        };

        self.watch.report_error(&error_ref);

        error_ref
    }
}

impl<'unit, N: Node, W: Watch> MutableSyntaxSession<'unit, N, W> {
    // Safety:
    // 1. `head` belongs to the `tree` instance.
    // 2. All references of the `tree` belong to `refs` instance.
    // 3. This function is never run with void syntax.
    // 4. If `rule != ROOT_RULE` then `primary_node` points to Occupied node.
    // 5. If `rule == ROOT_RULE` then `primary_node` points to Reserved or Occupied node.
    pub(super) unsafe fn run(
        tree: &'unit mut Tree<N>,
        refs: &'unit mut TreeRefs<N>,
        watch: &'unit mut W,
        start: Site,
        head: ChildCursor<N>,
        rule: NodeRule,
        primary_node: EntryIndex,
    ) -> (Cache, Site) {
        if is_void_syntax::<N>() {
            unsafe { debug_unreachable!("An attempt to reparse void syntax") }
        }

        let context = {
            let parent_entry;
            let node_entry;

            match rule == ROOT_RULE {
                true => {
                    parent_entry = Entry::nil();
                    node_entry = Entry {
                        index: 0,
                        version: 1,
                    };
                }

                false => {
                    parent_entry = unsafe { refs.nodes.get_unchecked(primary_node) }
                        .parent_ref()
                        .entry;

                    node_entry = unsafe { refs.nodes.entry_of_unchecked(primary_node) };
                }
            }

            let mut context = Vec::new();

            context.push(parent_entry);
            context.push(node_entry);

            context
        };

        let pending = Pending {
            lookahead_end_site: start,
            leftmost: rule != ROOT_RULE,
            primary_node,
            secondary_nodes: Vec::new(),
            errors: Vec::new(),
        };

        let length = tree.code_length();

        let mut session = Self {
            tree,
            refs,
            watch,
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

        let node = N::parse(&mut session, rule);

        let parse_end = session.parse_end();
        let pending = session.pending;
        let parsed_end_site = session.next_site;
        let refs = session.refs;

        let cache = unsafe { pending.into_cache(refs, rule, node, parse_end, parsed_end_site) };

        (cache, parsed_end_site)
    }

    #[inline(always)]
    fn parse_end(&self) -> SiteRef {
        match self.next_chunk_cursor.is_dangling() {
            false => {
                let chunk_entry_index = unsafe { self.next_chunk_cursor.chunk_entry_index() };
                let chunk_entry = unsafe { self.refs.chunks.entry_of_unchecked(chunk_entry_index) };

                TokenRef {
                    id: self.id(),
                    entry: chunk_entry,
                }
                .site_ref()
            }

            true => SiteRef::end_of(self.id()),
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

struct Pending {
    lookahead_end_site: Site,
    leftmost: bool,
    primary_node: EntryIndex,
    secondary_nodes: Vec<EntryIndex>,
    errors: Vec<EntryIndex>,
}

impl Pending {
    // Safety: `self.primary_node` points to occupied or reserved node.
    #[inline(always)]
    unsafe fn into_cache<N: Node>(
        self,
        refs: &mut TreeRefs<N>,
        rule: NodeRule,
        node: N,
        parse_end: SiteRef,
        parse_end_site: Site,
    ) -> Cache {
        unsafe { refs.nodes.set_unchecked(self.primary_node, node) };

        Cache {
            rule,
            parse_end,
            lookahead: self.lookahead_end_site - parse_end_site,
            primary_node: self.primary_node,
            secondary_nodes: self.secondary_nodes,
            errors: self.errors,
        }
    }
}
