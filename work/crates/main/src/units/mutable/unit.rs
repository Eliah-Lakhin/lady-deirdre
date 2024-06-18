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
    fmt::{Debug, Display, Formatter},
    mem::{replace, take, transmute_copy},
};

use crate::{
    arena::{Entry, EntryIndex, Id, Identifiable},
    format::SnippetFormatter,
    lexis::{
        Length,
        LineIndex,
        Site,
        SiteRef,
        SiteSpan,
        SourceCode,
        ToSpan,
        Token,
        TokenBuffer,
        TokenCount,
        CHUNK_SIZE,
    },
    report::{ld_assert, ld_assert_eq, ld_unreachable, system_panic},
    syntax::{
        is_void_syntax,
        Node,
        NodeRef,
        SyntaxError,
        SyntaxTree,
        VoidSyntax,
        NON_RULE,
        ROOT_RULE,
    },
    units::{
        mutable::{
            cursor::MutableCursor,
            iters::{MutableCharIter, MutableErrorIter, MutableNodeIter},
            lexis::{MutableLexisSession, SessionOutput},
            syntax::MutableSyntaxSession,
            watcher::VoidWatcher,
        },
        storage::{Cache, ChildCursor, Tree, TreeRefs},
        CompilationUnit,
        Watcher,
    },
};

/// A compilation unit with reparse capabilities.
///
/// This serves as an inner component
/// of the mutable [Document](crate::units::Document).
///
/// MutableUnit implements the same set of interfaces and provides the same
/// set of features, including the ability to edit the source code after
/// creation.
///
/// You are encouraged to use this object if you don’t need a uniform
/// mutable and immutable API of the Document.
pub struct MutableUnit<N: Node> {
    root: Option<Cache>,
    tree: Tree<N>,
    refs: TreeRefs<N>,
    lines: LineIndex,
    tokens: TokenCount,
}

// Safety: Tree instance stores data on the heap, and the References instance
//         refers Tree's heap objects only.
unsafe impl<N: Node> Send for MutableUnit<N> {}

// Safety:
//   1. Tree and TreeRefs data mutations can only happen through
//      the &mut Document exclusive interface that invalidates all other
//      references to the inner data of the Document's Tree.
//   2. All "weak" references are safe indexes into the Document's inner data.
unsafe impl<N: Node> Sync for MutableUnit<N> {}

impl<N: Node> Drop for MutableUnit<N> {
    fn drop(&mut self) {
        unsafe { self.tree.free() };

        self.id().clear_name();
    }
}

impl<N: Node> Debug for MutableUnit<N> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter
            .debug_struct("MutableUnit")
            .field("id", &self.id())
            .field("length", &self.length())
            .finish_non_exhaustive()
    }
}

impl<N: Node> Display for MutableUnit<N> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter
            .snippet(self)
            .set_caption(format!("MutableUnit({})", self.id()))
            .finish()
    }
}

impl<N: Node> Identifiable for MutableUnit<N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.refs.id
    }
}

impl<N: Node> SourceCode for MutableUnit<N> {
    type Token = N::Token;

    type Cursor<'code> = MutableCursor<'code, N>;

    type CharIterator<'code> = MutableCharIter<'code, N>;

    fn chars(&self, span: impl ToSpan) -> Self::CharIterator<'_> {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),

            Some(span) => span,
        };

        unsafe { MutableCharIter::new(self, span) }
    }

    #[inline(always)]
    fn has_chunk(&self, chunk_entry: &Entry) -> bool {
        self.refs.chunks.contains(chunk_entry)
    }

    #[inline(always)]
    fn get_token(&self, chunk_entry: &Entry) -> Option<Self::Token> {
        let chunk_cursor = self.refs.chunks.get(chunk_entry)?;

        ld_assert!(
            !chunk_cursor.is_dangling(),
            "Dangling chunk ref in the TreeRefs repository."
        );

        Some(unsafe { chunk_cursor.token() })
    }

    #[inline(always)]
    fn get_site(&self, chunk_entry: &Entry) -> Option<Site> {
        let chunk_cursor = self.refs.chunks.get(chunk_entry)?;

        Some(unsafe { self.tree.site_of(chunk_cursor) })
    }

    #[inline(always)]
    fn get_string(&self, chunk_entry: &Entry) -> Option<&str> {
        let chunk_cursor = self.refs.chunks.get(chunk_entry)?;

        ld_assert!(
            !chunk_cursor.is_dangling(),
            "Dangling chunk ref in the TreeRefs repository."
        );

        Some(unsafe { chunk_cursor.string() })
    }

    #[inline(always)]
    fn get_length(&self, chunk_entry: &Entry) -> Option<Length> {
        let chunk_cursor = self.refs.chunks.get(chunk_entry)?;

        ld_assert!(
            !chunk_cursor.is_dangling(),
            "Dangling chunk ref in the References repository."
        );

        Some(*unsafe { chunk_cursor.span() })
    }

    #[inline(always)]
    fn cursor(&self, span: impl ToSpan) -> Self::Cursor<'_> {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),

            Some(span) => span,
        };

        Self::Cursor::new(self, span)
    }

    #[inline(always)]
    fn length(&self) -> Length {
        ld_assert_eq!(
            self.tree.code_length(),
            self.lines.code_length(),
            "LineIndex and Tree resynchronization.",
        );

        self.tree.code_length()
    }

    #[inline(always)]
    fn tokens(&self) -> TokenCount {
        self.tokens
    }

    #[inline(always)]
    fn lines(&self) -> &LineIndex {
        &self.lines
    }
}

impl<N: Node> SyntaxTree for MutableUnit<N> {
    type Node = N;

    type NodeIterator<'tree> = MutableNodeIter<'tree, N>;

    type ErrorIterator<'tree> = MutableErrorIter<'tree>;

    #[inline(always)]
    fn root_node_ref(&self) -> NodeRef {
        let Some(root) = &self.root else {
            unsafe { ld_unreachable!("Root cache unset.") };
        };

        #[cfg(debug_assertions)]
        if root.primary_node != 0 {
            system_panic!("Root node moved.");
        }

        let entry = unsafe { self.refs.nodes.entry_of_unchecked(root.primary_node) };

        #[cfg(debug_assertions)]
        if entry.version != 1 {
            system_panic!("Root node moved.");
        }

        NodeRef {
            id: self.id(),
            entry,
        }
    }

    #[inline(always)]
    fn node_refs(&self) -> Self::NodeIterator<'_> {
        MutableNodeIter {
            id: self.id(),
            inner: self.refs.nodes.entries(),
        }
    }

    #[inline(always)]
    fn error_refs(&self) -> Self::ErrorIterator<'_> {
        MutableErrorIter {
            id: self.id(),
            inner: self.refs.errors.entries(),
        }
    }

    #[inline(always)]
    fn has_node(&self, entry: &Entry) -> bool {
        self.refs.nodes.contains(entry)
    }

    #[inline(always)]
    fn get_node(&self, entry: &Entry) -> Option<&Self::Node> {
        self.refs.nodes.get(entry)
    }

    #[inline(always)]
    fn get_node_mut(&mut self, entry: &Entry) -> Option<&mut Self::Node> {
        self.refs.nodes.get_mut(entry)
    }

    #[inline(always)]
    fn has_error(&self, entry: &Entry) -> bool {
        self.refs.errors.contains(entry)
    }

    #[inline(always)]
    fn get_error(&self, entry: &Entry) -> Option<&SyntaxError> {
        self.refs.errors.get(entry)
    }
}

impl<N: Node> Default for MutableUnit<N> {
    #[inline(always)]
    fn default() -> Self {
        let mut tree = Tree::default();
        let mut refs = TreeRefs::new(Id::new());

        let root = Self::initial_parse(&mut tree, &mut refs);

        Self {
            root: Some(root),
            tree,
            refs,
            lines: LineIndex::new(),
            tokens: 0,
        }
    }
}

impl<N: Node, S: AsRef<str>> From<S> for MutableUnit<N> {
    #[inline(always)]
    fn from(string: S) -> Self {
        Self::new(string)
    }
}

impl<N: Node> CompilationUnit for MutableUnit<N> {
    #[inline(always)]
    fn is_mutable(&self) -> bool {
        true
    }

    fn into_token_buffer(self) -> TokenBuffer<N::Token> {
        let mut buffer = TokenBuffer::with_capacity(self.tokens, self.length());

        let mut chunk_cursor = self.tree.first();

        while !chunk_cursor.is_dangling() {
            unsafe {
                chunk_cursor.take_lexis(
                    &mut buffer.spans,
                    &mut buffer.tokens,
                    &mut buffer.indices,
                    &mut buffer.text,
                )
            };

            unsafe { chunk_cursor.next() }
        }

        buffer.update_line_index();

        let _ = self;

        buffer
    }

    #[inline(always)]
    fn into_mutable_unit(self) -> MutableUnit<N> {
        self
    }
}

impl<N: Node> MutableUnit<N> {
    /// Creates a MutableUnit from the source code `text`.
    ///
    /// The parameter could be a [TokenBuffer] or just an arbitrary string.
    #[inline(always)]
    pub fn new(text: impl Into<TokenBuffer<N::Token>>) -> Self {
        let mut buffer = text.into();

        let count = buffer.tokens();
        let spans = take(&mut buffer.spans).into_iter();
        let indices = take(&mut buffer.indices).into_iter();
        let tokens = take(&mut buffer.tokens).into_iter();
        let lines = take(&mut buffer.lines);
        let mut refs = TreeRefs::with_capacity(Id::new(), count);

        let mut tree = unsafe {
            Tree::from_chunks(
                &mut refs,
                count,
                spans,
                indices,
                tokens,
                buffer.text.as_str(),
            )
        };

        let root = MutableUnit::initial_parse(&mut tree, &mut refs);

        Self {
            root: Some(root),
            tree,
            refs,
            lines,
            tokens: count,
        }
    }

    /// Writes user-input edit into this unit.
    ///
    /// See [Document::write](crate::units::Document::write) for details.
    ///
    /// **Panic**
    ///
    /// Panics if the specified span is not valid for this unit.
    #[inline(always)]
    pub fn write(&mut self, span: impl ToSpan, text: impl AsRef<str>) {
        self.write_and_watch(span, text, &mut VoidWatcher)
    }

    /// Writes user-input edit into this unit, and collects all syntax tree
    /// components that have been affected by this edit.
    ///
    /// See [Document::write_and_watch](crate::units::Document::write_and_watch)
    /// for details.
    ///
    /// **Panic**
    ///
    /// Panics if the specified span is not valid for this unit.
    #[inline(never)]
    pub fn write_and_watch(
        &mut self,
        span: impl ToSpan,
        text: impl AsRef<str>,
        watcher: &mut impl Watcher,
    ) {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),

            Some(span) => span,
        };

        let text = text.as_ref();

        if span.is_empty() && text.is_empty() {
            return;
        }

        unsafe { self.lines.write_unchecked(span.clone(), text) };

        let cover = self.update_lexis(watcher, span, text);

        ld_assert_eq!(
            self.tree.code_length(),
            self.lines.code_length(),
            "LineIndex and Tree resynchronization.",
        );

        if is_void_syntax::<N>() {
            return;
        }

        //todo consider removing Self::update_syntax return as it is currently unused
        let _entry = self.update_syntax(watcher, cover);
    }

    #[inline(always)]
    pub(super) fn tree(&self) -> &Tree<N> {
        &self.tree
    }

    #[inline(always)]
    pub(super) fn refs(&self) -> &TreeRefs<N> {
        &self.refs
    }

    fn update_lexis(
        &mut self,
        watcher: &mut impl Watcher,
        mut span: SiteSpan,
        text: &str,
    ) -> Cover<N> {
        let mut head;
        let mut lookback;
        let mut tail;
        let mut tail_offset;

        match span.start == span.end {
            false => {
                lookback = span.start;
                head = self.tree.lookup(&mut lookback);
                tail_offset = span.end;
                tail = self.tree.lookup(&mut tail_offset);
            }

            true => {
                lookback = span.start;
                head = self.tree.lookup(&mut lookback);
                tail_offset = lookback;
                tail = head;
            }
        }

        let mut input = Vec::with_capacity(3);

        match lookback > 0 {
            true => {
                ld_assert!(
                    !head.is_dangling(),
                    "Dangling reference with non-zero offset.",
                );

                input.push(split_left(unsafe { head.string() }, lookback));

                span.start -= lookback;
            }

            false => {
                if head.is_dangling() {
                    head = self.tree.last();

                    if !head.is_dangling() {
                        let head_string = unsafe { head.string() };
                        let head_span = unsafe { *head.span() };

                        input.push(head_string);

                        span.start -= head_span;
                        lookback = head_span;
                    }
                }
            }
        }

        if !head.is_dangling() {
            while lookback < <N::Token as Token>::LOOKBACK {
                ld_assert!(!head.is_dangling(), "Dangling head.",);

                if unsafe { head.is_first() } {
                    break;
                }

                unsafe { head.back() };

                let head_string = unsafe { head.string() };
                let head_span = unsafe { *head.span() };

                input.insert(0, head_string);

                span.start -= head_span;
                lookback += head_span;
            }
        }

        if !text.is_empty() {
            input.push(text);
        }

        if tail_offset > 0 {
            ld_assert!(
                !tail.is_dangling(),
                "Dangling reference with non-zero offset.",
            );

            let length = unsafe { *tail.span() };

            input.push(split_right(unsafe { tail.string() }, tail_offset));

            span.end += length - tail_offset;

            unsafe { tail.next() }
        }

        let mut product = match input.is_empty() {
            false => unsafe { MutableLexisSession::run(text.len() / CHUNK_SIZE + 2, &input, tail) },

            true => SessionOutput {
                length: 0,
                spans: Vec::new(),
                indices: Vec::new(),
                tokens: Vec::new(),
                text: String::new(),
                tail,
                overlap: 0,
            },
        };

        span.end += product.overlap;

        let mut skip = 0;

        loop {
            if head.is_dangling() {
                break;
            }

            if unsafe { head.same_chunk_as(&product.tail) } {
                break;
            }

            let product_string = match product.indices.get(skip) {
                Some(start_byte) => {
                    let next_index = skip + 1;

                    match next_index < product.indices.len() {
                        true => {
                            let end_byte = unsafe { product.indices.get_unchecked(next_index) };

                            unsafe { product.text.get_unchecked(*start_byte..*end_byte) }
                        }

                        false => unsafe { product.text.get_unchecked(*start_byte..) },
                    }
                }
                None => break,
            };

            let head_string = unsafe { head.string() };

            if product_string == head_string {
                let head_span = unsafe { *head.span() };

                span.start += head_span;
                product.length -= head_span;
                skip += 1;

                unsafe { head.next() };

                continue;
            }

            break;
        }

        loop {
            if product.count() == skip {
                break;
            }

            if unsafe { head.same_chunk_as(&product.tail) } {
                break;
            }

            let last = match product.tail.is_dangling() {
                false => {
                    let mut previous = product.tail;

                    unsafe { previous.back() };

                    previous
                }

                true => self.tree.last(),
            };

            if last.is_dangling() {
                break;
            }

            let product_string = match product.indices.last() {
                Some(start_byte) => unsafe { product.text.as_str().get_unchecked(*start_byte..) },
                None => break,
            };

            let last_string = unsafe { last.string() };

            if product_string == last_string {
                let last_span = unsafe { *last.span() };

                span.end -= last_span;

                let _ = product.spans.pop();
                let index = product.indices.pop();
                let _ = product.tokens.pop();

                if let Some(index) = index {
                    unsafe { product.text.as_mut_vec().set_len(index) };
                }
                product.length -= last_span;
                product.tail = last;

                continue;
            }

            break;
        }

        if head.is_dangling() {
            ld_assert!(
                product.tail.is_dangling(),
                "Dangling head and non-dangling tail.",
            );

            let token_count = product.count() - skip;

            let tail_tree = unsafe {
                Tree::from_chunks(
                    &mut self.refs,
                    token_count,
                    product.spans.into_iter().skip(skip),
                    product.indices.into_iter().skip(skip),
                    product.tokens.into_iter().skip(skip),
                    product.text.as_str(),
                )
            };

            let insert_span = tail_tree.code_length();

            unsafe { self.tree.join(&mut self.refs, tail_tree) };

            self.tokens += token_count;

            let chunk_cursor = {
                let mut point = span.start;

                let chunk_cursor = self.tree.lookup(&mut point);

                ld_assert_eq!(point, 0, "Bad span alignment.");

                chunk_cursor
            };

            return Cover {
                chunk_cursor,
                span: span.start..(span.start + insert_span),
            };
        }

        let insert_count = product.count() - skip;

        if let Some(remove_count) = unsafe { head.continuous_to(&product.tail) } {
            if unsafe { self.tree.is_writeable(&head, remove_count, insert_count) } {
                let (chunk_cursor, insert_span) = unsafe {
                    self.tree.write(
                        &mut self.refs,
                        watcher,
                        head,
                        remove_count,
                        insert_count,
                        product.spans.into_iter().skip(skip),
                        unsafe { product.indices.get_unchecked(skip..) },
                        product.tokens.into_iter().skip(skip),
                        product.text.as_str(),
                    )
                };

                self.tokens += insert_count;
                self.tokens -= remove_count;

                return Cover {
                    chunk_cursor,
                    span: span.start..(span.start + insert_span),
                };
            }
        }

        let mut middle = unsafe { self.tree.split(&mut self.refs, head) };

        let middle_split_point = {
            let mut point = span.end - span.start;

            let chunk_cursor = middle.lookup(&mut point);

            ld_assert_eq!(point, 0, "Bad span alignment.");

            chunk_cursor
        };

        let right = unsafe { middle.split(&mut self.refs, middle_split_point) };

        let remove_count;
        let insert_span;

        {
            let replacement = unsafe {
                Tree::from_chunks(
                    &mut self.refs,
                    insert_count,
                    product.spans.into_iter().skip(skip),
                    product.indices.into_iter().skip(skip),
                    product.tokens.into_iter().skip(skip),
                    product.text.as_str(),
                )
            };

            insert_span = replacement.code_length();

            remove_count = unsafe {
                replace(&mut middle, replacement).free_as_subtree(&mut self.refs, watcher)
            };
        };

        unsafe { self.tree.join(&mut self.refs, middle) };
        unsafe { self.tree.join(&mut self.refs, right) };

        self.tokens += insert_count;
        self.tokens -= remove_count;

        head = {
            let mut point = span.start;

            let chunk_cursor = self.tree.lookup(&mut point);

            ld_assert_eq!(point, 0, "Bad span alignment.");

            chunk_cursor
        };

        Cover {
            chunk_cursor: head,
            span: span.start..(span.start + insert_span),
        }
    }

    fn update_syntax(&mut self, watcher: &mut impl Watcher, mut cover: Cover<N>) -> EntryIndex {
        #[allow(unused_variables)]
        let mut cover_lookahead = 0;

        loop {
            let mut shift;
            let mut rule;

            match cover.chunk_cursor.is_dangling() {
                false => match unsafe { cover.chunk_cursor.is_first() } {
                    true => match unsafe { cover.chunk_cursor.cache().is_some() } {
                        false => {
                            shift = 0;
                            rule = ROOT_RULE;
                        }

                        true => {
                            shift = 0;
                            rule = NON_RULE
                        }
                    },

                    false => {
                        unsafe { cover.chunk_cursor.back() };

                        shift = unsafe { *cover.chunk_cursor.span() };

                        rule = NON_RULE;
                    }
                },

                true => match self.tree.code_length() == 0 {
                    true => {
                        shift = 0;
                        rule = ROOT_RULE;
                    }

                    false => {
                        cover.chunk_cursor = self.tree.last();

                        shift = unsafe { *cover.chunk_cursor.span() };

                        rule = NON_RULE;
                    }
                },
            }

            if rule != ROOT_RULE {
                loop {
                    {
                        match unsafe { cover.chunk_cursor.cache() } {
                            None => {
                                unsafe { cover.chunk_cursor.back() };

                                match cover.chunk_cursor.is_dangling() {
                                    false => {
                                        shift += unsafe { *cover.chunk_cursor.span() };
                                        continue;
                                    }

                                    true => {
                                        rule = ROOT_RULE;
                                        break;
                                    }
                                }
                            }

                            Some(cache) => {
                                let parse_end_site =
                                    unsafe { cache.end_site(&self.tree, &self.refs) };

                                if let Some(parse_end_site) = parse_end_site {
                                    if parse_end_site + cache.lookahead < cover.span.start {
                                        unsafe { cover.chunk_cursor.back() };

                                        match cover.chunk_cursor.is_dangling() {
                                            false => {
                                                shift += unsafe { *cover.chunk_cursor.span() };
                                                continue;
                                            }

                                            true => {
                                                rule = ROOT_RULE;
                                                break;
                                            }
                                        }
                                    }

                                    if parse_end_site >= cover.span.end {
                                        cover.span.start -= shift;
                                        cover.span.end = parse_end_site;

                                        #[allow(unused_assignments)]
                                        {
                                            cover_lookahead = cache.lookahead;
                                        }

                                        rule = cache.rule;
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    let cache = unsafe { cover.chunk_cursor.release_cache() };

                    cache.free(&mut self.refs, watcher);
                }
            }

            if rule == ROOT_RULE {
                let head = self.tree.first();

                let Some(root_cache) = take(&mut self.root) else {
                    unsafe { ld_unreachable!("Missing root cache.") }
                };

                let (rule, primary_node) = root_cache.free_inner(&mut self.refs, watcher);

                #[cfg(debug_assertions)]
                if rule != ROOT_RULE {
                    system_panic!("Root cache refers non-root rule.");
                }

                let (root_cache, mut parse_end_site) = unsafe {
                    MutableSyntaxSession::run(
                        &mut self.tree,
                        &mut self.refs,
                        watcher,
                        0,
                        head,
                        rule,
                        primary_node,
                    )
                };

                self.root = Some(root_cache);

                if self.tree.code_length() > 0 {
                    let mut tail = self.tree.lookup(&mut parse_end_site);

                    ld_assert_eq!(parse_end_site, 0, "Incorrect span alignment.");

                    while !tail.is_dangling() {
                        let has_cache = unsafe { tail.cache().is_some() };

                        if has_cache {
                            unsafe { tail.release_cache() }.free(&mut self.refs, watcher);
                        }

                        unsafe { tail.next() }
                    }
                }

                return primary_node;
            }

            let cache = unsafe { cover.chunk_cursor.release_cache() };

            let (rule, primary_node) = cache.free_inner(&mut self.refs, watcher);

            let (cache, parse_end_site) = unsafe {
                MutableSyntaxSession::run(
                    &mut self.tree,
                    &mut self.refs,
                    watcher,
                    cover.span.start,
                    cover.chunk_cursor,
                    rule,
                    primary_node,
                )
            };

            unsafe { cover.chunk_cursor.install_cache(cache) }

            //todo check lookahead too
            if cover.span.end == parse_end_site {
                return primary_node;
            }

            cover.span.end = cover.span.end.max(parse_end_site);
        }
    }

    // Safety:
    // 1. All references of the `tree` belong to `refs` instance.
    #[inline(always)]
    fn initial_parse<'unit>(tree: &'unit mut Tree<N>, refs: &'unit mut TreeRefs<N>) -> Cache {
        if is_void_syntax::<N>() {
            let primary_node = refs.nodes.insert_raw(unsafe {
                transmute_copy::<VoidSyntax<<N as Node>::Token>, N>(&VoidSyntax::default())
            });

            return Cache {
                rule: ROOT_RULE,
                parse_end: SiteRef::nil(),
                lookahead: 0,
                primary_node,
                secondary_nodes: Vec::new(),
                errors: Vec::new(),
            };
        }

        let head = tree.first();

        let primary_node = refs.nodes.reserve_entry();

        let (root_cache, _parsed_end_site) = unsafe {
            MutableSyntaxSession::run(
                tree,
                refs,
                &mut VoidWatcher,
                0,
                head,
                ROOT_RULE,
                primary_node,
            )
        };

        root_cache
    }
}

impl<T: Token> TokenBuffer<T> {
    /// Turns this token buffer into MutableUnit.
    ///
    /// The `N` generic parameter specifies a type of the syntax tree [Node]
    /// with the `T` [lexis](Node::Token).
    #[inline(always)]
    pub fn into_mutable_unit<N>(self) -> MutableUnit<N>
    where
        N: Node<Token = T>,
    {
        MutableUnit::new(self)
    }
}

struct Cover<N: Node> {
    chunk_cursor: ChildCursor<N>,
    span: SiteSpan,
}

#[inline]
fn split_left(string: &str, mut site: Site) -> &str {
    if site == 0 {
        return "";
    }

    for (index, _) in string.char_indices() {
        if site == 0 {
            return unsafe { string.get_unchecked(0..index) };
        }

        site -= 1;
    }

    string
}

#[inline]
fn split_right(string: &str, mut site: Site) -> &str {
    if site == 0 {
        return string;
    }

    for (index, _) in string.char_indices() {
        if site == 0 {
            return unsafe { string.get_unchecked(index..string.len()) };
        }

        site -= 1;
    }

    ""
}
