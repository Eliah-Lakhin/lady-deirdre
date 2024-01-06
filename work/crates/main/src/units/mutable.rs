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

mod chars;
mod cursor;
mod lexis;
mod syntax;

use crate::{
    arena::{Entry, Id, Identifiable},
    format::SnippetFormatter,
    lexis::{
        Length,
        LineIndex,
        Site,
        SiteRefInner,
        SiteSpan,
        SourceCode,
        ToSpan,
        Token,
        TokenBuffer,
        TokenCount,
        CHUNK_SIZE,
    },
    report::{debug_assert, debug_assert_eq, debug_unreachable},
    std::*,
    syntax::{
        Cluster,
        ClusterRef,
        ErrorRef,
        NoSyntax,
        Node,
        NodeRef,
        SyntaxTree,
        NON_RULE,
        ROOT_RULE,
    },
    units::{
        mutable::{
            chars::MutableCharIter,
            cursor::MutableCursor,
            lexis::{MutableLexisSession, SessionOutput},
            syntax::MutableSyntaxSession,
        },
        storage::{ChildCursor, ClusterCache, References, Tree},
        unit::NodeCoverage,
        CompilationUnit,
    },
};

const REPORT_CAPACITY: usize = 100;

/// An incrementally managed compilation unit.
///
/// Document is a storage of a compilation unit(a source code of the file) with incremental update
/// operations. Document object stores the source code, the lexical structure of the code, and the
/// syntax structure of the code. This is the main entry point of the crate API.
///
/// Document is responsible to load the source code, to parse the source code grammar and to
/// construct lexical and syntax structure of the code, and to perform update operations in
/// incremental way keeping the code, lexis and syntax structures in sync with the changes.
///
/// Depending on the end compilation system needs there could be several instances of this object
/// per each compilation unit(per each file of the file structure of compiled project).
///
/// ## Instantiation.
///
/// An API user specifies Document grammar using generic type parameter `N` of the
/// [Node](crate::syntax::Node) type.
///
/// To opt out syntax analysis stage(e.g. if the syntax grammar unknown or not needed in particular
/// case), an API user uses special implementation of the Node called
/// [`NoSyntax<T: Token>`](crate::syntax::NoSyntax) that enforces Document to skip syntax analysis
/// and the Syntax Tree construction, but persists lexical structure only.
///
/// There are three ways to initially load the source code text into the Document:
///  1. By loading from the relatively small string snippet.
///     ```rust
///      use lady_deirdre::{units::Document, syntax::SimpleNode};
///
///      let _ = Document::<SimpleNode>::from("test string");
///     ```
///  2. By initializing an empty Document, and using [write](Document::write) operation on
///     the instance.
///     ```rust
///      use lady_deirdre::{units::Document, syntax::SimpleNode};
///
///      let mut doc = Document::<SimpleNode>::default();
///      doc.write(.., "test string");
///     ```
///  3. And using dedicated [TokenBuffer](crate::lexis::Tokens) instance to preload large file.
///     ```rust
///      use lady_deirdre::{units::Document, syntax::SimpleNode, lexis::TokenBuffer};
///
///      let mut buffer = TokenBuffer::default();
///      buffer.append("First line.\n");
///      buffer.append("Second line.\nThird line.\n");
///
///      let _doc = buffer.into_document::<SimpleNode>();
///     ```
///
/// As the TokenBuffer provides functionality for fast line-by-line lexis pre-parsing the last
/// option is the most preferable(but the most verbose) way for production use.
///
/// ## Updating.
///
/// An API user performs write operations into the Document using [write](Document::write)
/// function specifying a [Span](crate::lexis::ToSpan) of the code to rewrite(possibly empty span),
/// and a string to insert in place of this spanned test. Document performs update operations in
/// time relative to the user changes, so it is totally fine to call this function on every end-user
/// input action even on large documents.
///
/// ```rust
/// use lady_deirdre::{units::Document, syntax::SimpleNode, lexis::SourceCode};
///
/// let mut doc = Document::<SimpleNode>::from("test string");
///
/// // Writing another string in the begin of the Document.
/// doc.write(0..0, "Foo ");
/// assert_eq!(doc.substring(..), "Foo test string");
///
/// // Removing "test " substring.
/// doc.write(4..9, "");
/// assert_eq!(doc.substring(..), "Foo string");
///
/// // Surrounding substring "str" with parenthesis.
/// doc.write(4..7, "(str)");
/// assert_eq!(doc.substring(..), "Foo (str)ing");
/// ```
///
/// There are several ways to specify this Span. In particular, an API use can utilize simple ranges
/// of character absolute indices([Sites](crate::lexis::Site) as in the example above), ranges of
/// the column-row [Positions](crate::lexis::Position), or ranges of the
/// [token weak references](crate::lexis::TokenRef).
///
/// ## Inspecting Lexis Structure.
///
/// Document implements the [SourceCode](crate::lexis::SourceCode) trait and the
/// [CodeContent](crate::lexis::CodeContent) extension trait that provide lexical structure
/// inspection features.
///
/// ```rust
/// use lady_deirdre::{
///     units::Document,
///     lexis::{SourceCode, SimpleToken},
///     syntax::SimpleNode,
/// };
///
/// let doc = Document::<SimpleNode>::from("foo bar baz");
///
/// // A number of characters in the Document.
/// assert_eq!(doc.length(), 11);
///
/// // A number of tokens in the Document(including whitespace tokens).
/// assert_eq!(doc.token_count(), 5);
///
/// // A substring from the Document source code.
/// assert_eq!(doc.substring(1..6), "oo ba");
///
/// // A set of lengths of the tokens that "touch" specified span.
/// assert_eq!(doc.chunks(5..7).map(|chunk| chunk.length).collect::<Vec<_>>(), vec![3, 1]);
///
/// // A set of strings of the tokens that "touch" specified span.
/// assert_eq!(doc.chunks(5..7).map(|chunk| chunk.string).collect::<Vec<_>>(), vec!["bar", " "]);
/// ```
///
/// An API users utilizes lower-level [TokenCursor](crate::lexis::TokenCursor) API to traverse and
/// to inspect individual tokens metadata.
///
/// ```rust
/// use lady_deirdre::{
///     units::Document,
///     lexis::{SourceCode, TokenCursor, SimpleToken},
///     syntax::SimpleNode
/// };
///
/// let mut doc = Document::<SimpleNode>::from("foo bar baz");
///
/// // A generic "iterator" over the tokens at the specified Site(token "bar").
/// let mut cursor = doc.cursor(5..5);
///
/// // A reference of the first token "bar" from this cursor.
/// let token_ref = cursor.token_ref(0);
///
/// // "bar" is of "Identifier" type.
/// assert_eq!(token_ref.deref(&doc), Some(SimpleToken::Identifier));
/// assert_eq!(token_ref.string(&doc), Some("bar"));
///
/// // Write something at the beginning of the Document.
/// doc.write(0..0, "123");
/// assert_eq!(doc.substring(..), "123foo bar baz");
///
/// // TokenRef is still dereferencable after the Document changes, because the token was not
/// // affected by these changes.
/// assert_eq!(token_ref.string(&doc), Some("bar"));
///
/// // And we can write something at the token start Site too.
/// let token_start_site_ref = token_ref.site_ref();
/// doc.write(token_start_site_ref..token_start_site_ref, "X");
/// assert_eq!(doc.substring(..), "123foo Xbar baz");
///
/// // However, the TokenRef is no longer valid because the token has been rewritten after
/// // the previous write action.
/// assert_eq!(token_ref.string(&doc), None);
/// ```
///
/// ## Inspecting Syntax Structure.
///
/// Document implements the [SyntaxTree](crate::syntax::SyntaxTree) trait that provides
/// Syntax Tree and Syntax Errors access features.
///
/// ```rust
/// use lady_deirdre::{
///     units::Document,
///     syntax::{SimpleNode, SyntaxTree, NodeRef},
///     lexis::{SourceCode, ToSpan},
/// };
///
/// let mut doc = Document::<SimpleNode>::from("foo ([bar] {baz})");
///
/// // Returns a weak reference to the root os the SyntaxTree.
/// // It is OK to copy this reference and reuse the copy many times.
/// let root_ref = doc.root_node_ref();
///
/// // A simple parens structure formatter that traverses the Syntax Tree.
/// fn fmt(doc: &Document<SimpleNode>, node_ref: &NodeRef) -> String {
///     let node = match node_ref.deref(doc) {
///         Some(node) => node,
///         // If the NodeRef is invalid it means that the syntax parser failed
///         // to parse particular part of the source code due to syntax errors.
///         None => return format!("?"),
///     };
///
///     let children = match node {
///         SimpleNode::Root { inner } => inner,
///         SimpleNode::Braces { inner } => inner,
///         SimpleNode::Brackets { inner } => inner,
///         SimpleNode::Parenthesis { inner } => inner,
///     };
///
///     let children_fmt = children
///         .iter()
///         .map(|node_ref| fmt(doc, node_ref))
///         .collect::<Vec<_>>().join(", ");
///
///     match node {
///         SimpleNode::Root { .. } => children_fmt,
///         SimpleNode::Braces { .. } => format!("{{{}}}", children_fmt),
///         SimpleNode::Brackets { .. } => format!("[{}]", children_fmt),
///         SimpleNode::Parenthesis { .. } => format!("({})", children_fmt),
///     }
/// }
///
/// assert_eq!(fmt(&doc, &root_ref).as_str(), "([], {})");
///
/// // Writing another bracket snippet at the begin of the Document.
/// doc.write(0..0, "[{x} [y] (z)]");
/// assert_eq!(doc.substring(..), "[{x} [y] (z)]foo ([bar] {baz})");
/// assert_eq!(fmt(&doc, &root_ref).as_str(), "[{}, [], ()], ([], {})");
///
/// // The Document is resistant to the syntax errors preserving original Tree structure.
/// // Removing the second char "{".
/// doc.write(1..2, "");
/// assert_eq!(doc.substring(..), "[x} [y] (z)]foo ([bar] {baz})");
/// assert_eq!(fmt(&doc, &root_ref).as_str(), "[[], ()], ([], {})");
///
/// // Collecting syntax errors.
/// let errors = doc.errors()
///     .map(|error| error.display(&doc).to_string())
///     .collect::<Vec<_>>()
///     .join("\n");
/// assert_eq!(
///     errors.as_str(),
///     "1:2 (3 chars): Unexpected input in Brackets.",
/// );
///
/// // Syntax Tree is a mutable structure.
/// // Adding artificial empty braces Node to the Root.
/// {
///     let new_node = SimpleNode::Braces { inner: vec![] };
///     let new_node_ref = root_ref.cluster_ref().link_node(&mut doc, new_node);
///
///     match root_ref.deref_mut(&mut doc).unwrap() {
///         SimpleNode::Root { inner } => { inner.push(new_node_ref) }
///         _ => unreachable!()
///     }
/// }
///
/// assert_eq!(doc.substring(..), "[x} [y] (z)]foo ([bar] {baz})");
/// assert_eq!(fmt(&doc, &root_ref).as_str(), "[[], ()], ([], {}), {}");
/// ```
pub struct MutableUnit<N: Node> {
    id: Id,
    root_cluster: Cluster<N>,
    tree: Tree<N>,
    token_count: TokenCount,
    pub(super) references: References<N>,
    watch: Option<WatchReport>,
    lines: LineIndex,
}

// Safety: Tree instance stores data on the heap, and the References instance
//         refers Tree's heap objects only.
unsafe impl<N: Node> Send for MutableUnit<N> {}

// Safety:
//   1. Tree and References data mutations can only happen through
//      the &mut Document exclusive interface that invalidates all other
//      references to the inner data of the Document's Tree.
//   2. All "weak" references are safe indexes into the Document's inner data.
unsafe impl<N: Node> Sync for MutableUnit<N> {}

impl<N: Node> Drop for MutableUnit<N> {
    fn drop(&mut self) {
        unsafe { self.tree.free() };

        self.id.clear_name();
    }
}

impl<N: Node> Debug for MutableUnit<N> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter
            .debug_struct("MutableUnit")
            .field("id", &self.id)
            .field("length", &self.length())
            .finish_non_exhaustive()
    }
}

impl<N: Node> Display for MutableUnit<N> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter
            .snippet(self)
            .set_caption(format!("MutableUnit({})", self.id))
            .finish()
    }
}

impl<N: Node> Identifiable for MutableUnit<N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
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
        self.references.chunks().contains(chunk_entry)
    }

    #[inline(always)]
    fn get_token(&self, chunk_entry: &Entry) -> Option<Self::Token> {
        let chunk_cursor = self.references.chunks().get(chunk_entry)?;

        debug_assert!(
            !chunk_cursor.is_dangling(),
            "Dangling chunk ref in the References repository."
        );

        Some(unsafe { chunk_cursor.token() })
    }

    #[inline(always)]
    fn get_site(&self, chunk_entry: &Entry) -> Option<Site> {
        let chunk_cursor = self.references.chunks().get(chunk_entry)?;

        Some(unsafe { self.tree.site_of(chunk_cursor) })
    }

    #[inline(always)]
    fn get_string(&self, chunk_entry: &Entry) -> Option<&str> {
        let chunk_cursor = self.references.chunks().get(chunk_entry)?;

        debug_assert!(
            !chunk_cursor.is_dangling(),
            "Dangling chunk ref in the References repository."
        );

        Some(unsafe { chunk_cursor.string() })
    }

    #[inline(always)]
    fn get_length(&self, chunk_entry: &Entry) -> Option<Length> {
        let chunk_cursor = self.references.chunks().get(chunk_entry)?;

        debug_assert!(
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
        debug_assert_eq!(
            self.tree.code_length(),
            self.lines.code_length(),
            "LineIndex and Tree resynchronization.",
        );

        self.tree.code_length()
    }

    #[inline(always)]
    fn token_count(&self) -> TokenCount {
        self.token_count
    }

    #[inline(always)]
    fn lines(&self) -> &LineIndex {
        &self.lines
    }
}

impl<N: Node> SyntaxTree for MutableUnit<N> {
    type Node = N;

    #[inline(always)]
    fn has_cluster(&self, cluster_entry: &Entry) -> bool {
        match cluster_entry {
            Entry::Primary => true,

            Entry::Repo { .. } => self.references.clusters().contains(cluster_entry),

            _ => false,
        }
    }

    #[inline(always)]
    fn get_cluster(&self, cluster_entry: &Entry) -> Option<&Cluster<Self::Node>> {
        match cluster_entry {
            Entry::Primary => Some(&self.root_cluster),

            Entry::Repo { .. } => {
                let chunk_cursor = self.references.clusters().get(cluster_entry)?;

                let cluster_cache = unsafe { chunk_cursor.cache()? };

                Some(&cluster_cache.cluster)
            }

            _ => None,
        }
    }

    #[inline(always)]
    fn get_cluster_mut(&mut self, cluster_entry: &Entry) -> Option<&mut Cluster<Self::Node>> {
        match cluster_entry {
            Entry::Primary => Some(&mut self.root_cluster),

            Entry::Repo { .. } => {
                let mut chunk_cursor = *self.references.clusters().get(cluster_entry)?;

                let cluster_cache = unsafe { chunk_cursor.cache_mut()? };

                Some(&mut cluster_cache.cluster)
            }

            _ => None,
        }
    }

    #[inline(always)]
    fn get_previous_cluster(&self, cluster_entry: &Entry) -> Entry {
        if let Some(mut chunk_cursor) = self.references.clusters().get(cluster_entry).copied() {
            while !chunk_cursor.is_dangling() {
                unsafe { chunk_cursor.back() }

                if chunk_cursor.is_dangling() {
                    return Entry::Primary;
                }

                if unsafe { chunk_cursor.cache() }.is_some() {
                    let entry_index = unsafe { chunk_cursor.cache_index() };

                    return unsafe { self.references.clusters().entry_of(entry_index) };
                }
            }
        }

        Entry::Nil
    }

    #[inline(always)]
    fn get_next_cluster(&self, cluster_entry: &Entry) -> Entry {
        let mut chunk_cursor = match cluster_entry {
            Entry::Primary => self.tree.first(),

            Entry::Repo { .. } => match self.references.clusters().get(cluster_entry).copied() {
                None => return Entry::Nil,
                Some(child_cursor) => child_cursor,
            },

            _ => return Entry::Nil,
        };

        while !chunk_cursor.is_dangling() {
            unsafe { chunk_cursor.next() };

            if chunk_cursor.is_dangling() {
                break;
            }

            if unsafe { chunk_cursor.cache() }.is_some() {
                let entry_index = unsafe { chunk_cursor.cache_index() };

                return unsafe { self.references.clusters().entry_of(entry_index) };
            }
        }

        Entry::Nil
    }

    #[inline(always)]
    fn remove_cluster(&mut self, cluster_entry: &Entry) -> Option<Cluster<Self::Node>> {
        let chunk_cursor = self.references.clusters_mut().remove(cluster_entry)?;

        let cache = unsafe { chunk_cursor.take_cache() };

        Some(cache.cluster)
    }
}

impl<N: Node> Default for MutableUnit<N> {
    #[inline(always)]
    fn default() -> Self {
        let id = Id::new();
        let mut tree = Tree::default();
        let mut references = References::default();

        let root_cluster = Self::initial_parse(id, &mut tree, &mut references, &mut None);

        Self {
            id,
            root_cluster,
            tree,
            token_count: 0,
            references,
            watch: None,
            lines: LineIndex::new(),
        }
    }
}

impl<N: Node, S: AsRef<str>> From<S> for MutableUnit<N> {
    #[inline(always)]
    fn from(string: S) -> Self {
        Self::new(string, false)
    }
}

impl<N: Node> CompilationUnit for MutableUnit<N> {
    #[inline(always)]
    fn is_mutable(&self) -> bool {
        true
    }

    fn into_token_buffer(self) -> TokenBuffer<N::Token> {
        let mut buffer = TokenBuffer::with_capacity(self.token_count);

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

    #[inline(always)]
    fn cover(&self, span: impl ToSpan) -> NodeRef {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),

            Some(span) => span,
        };

        let node_ref = self.cluster_cover(&span).primary_node_ref();

        match NodeCoverage::cover(self, &node_ref, &span) {
            NodeCoverage::Fit(result) => result,
            _ => node_ref,
        }
    }
}

impl<N: Node> MutableUnit<N> {
    #[inline(always)]
    pub fn new(text: impl Into<TokenBuffer<N::Token>>, watch: bool) -> Self {
        Self::from_token_buffer(text.into(), watch)
    }

    fn from_token_buffer(mut buffer: TokenBuffer<N::Token>, watch: bool) -> Self {
        let id = Id::new();

        let token_count = buffer.token_count();
        let spans = take(&mut buffer.spans).into_vec().into_iter();
        let indices = take(&mut buffer.indices).into_vec().into_iter();
        let tokens = take(&mut buffer.tokens).into_vec().into_iter();
        let lines = take(&mut buffer.lines);
        let mut references = References::with_capacity(token_count);

        let mut tree = unsafe {
            Tree::from_chunks(
                &mut references,
                token_count,
                spans,
                indices,
                tokens,
                buffer.text.as_str(),
            )
        };

        let mut watch = match watch {
            true => Some(WatchReport::new()),
            false => None,
        };

        let root_cluster = MutableUnit::initial_parse(id, &mut tree, &mut references, &mut watch);

        Self {
            id,
            root_cluster,
            tree,
            token_count,
            references,
            watch,
            lines,
        }
    }

    /// Replaces a spanned substring of the source code with provided `text` string, and re-parses
    /// Document's lexical and syntax structure relatively to these changes.
    ///
    /// Operation performance complexity is relative to the `span` and the `text` size. As such it
    /// is fine to call this function frequently for relatively small changes even for the Documents
    /// that hold large source codes. For example, it is fine to call this function on every end
    /// user keyboard typing actions.
    ///
    /// The amount of original lexis and syntax structure of the Document to be re-created after
    /// this operation completion is not specified. The implementation tends to re-use as much
    /// data from the original structures as possible. However, some weak references into the
    /// Document [tokens](crate::lexis::TokenRef), [sites](crate::lexis::SiteRef),
    /// [nodes](crate::syntax::NodeRef), [clusters](crate::syntax::Cluster) and
    /// [errors](crate::syntax::ErrorRef) may obsolete.  
    ///
    /// There are many ways to specify the `span` of the source code. The most trivial way is
    /// to use a [Range](std::ops::Range) of characters absolute indices(`120..128`). Another way
    /// is to specify a range of the column-row [positions](crate::lexis::Position):
    /// `Position::new(10, 20)..Position::new(10..28)`. For details, see
    /// [ToSpan](crate::lexis::ToSpan) documentation.
    ///
    /// Note, that the Span range could be an empty range. In this case the `span` object will
    /// specify just a cursor inside the code, and the Write operation becomes an Insertion
    /// operation of specified `text`. If `text` is an empty string, Write operation becomes
    /// a Deletion operation.
    ///
    /// ```rust
    /// use lady_deirdre::{units::Document, lexis::SourceCode, syntax::SimpleNode};
    ///
    /// let mut doc = Document::<SimpleNode>::from("foo bar baz");
    ///
    /// doc.write(4..7, "BaR");
    ///
    /// assert_eq!(doc.substring(..), "foo BaR baz");
    /// ```
    ///
    /// Write operation will panic if the `span` cannot be turned into a
    /// [SiteSpan](crate::lexis::SiteSpan). In other words, if the Span is not a valid span for this
    /// Document instance. This is practically impossible when an API user uses arbitrary numeric
    /// values such as ranges of character absolute indices or ranges of Positions, but it could
    /// happen, for example, if the user provides a range of [SiteRef](crate::lexis::SiteRef).
    /// Because Site weak references could obsolete. In this case an API user could preliminary
    /// check span's validity using [is_valid_span](crate::lexis::ToSpan::is_valid_span) function.
    ///
    #[inline(never)]
    pub fn write(&mut self, span: impl ToSpan, text: impl AsRef<str>) -> NodeRef {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),

            Some(span) => span,
        };

        let text = text.as_ref();

        if span.is_empty() && text.is_empty() {
            return NodeRef::nil();
        }

        unsafe { self.lines.write_unchecked(span.clone(), text) };

        let cover = self.update_lexis(span, text);

        debug_assert_eq!(
            self.tree.code_length(),
            self.lines.code_length(),
            "LineIndex and Tree resynchronization.",
        );

        if TypeId::of::<N>() == TypeId::of::<NoSyntax<<N as Node>::Token>>() {
            return NodeRef::nil();
        }

        let cluster_entry = self.update_syntax(cover);

        let cover = NodeRef {
            id: self.id,
            cluster_entry,
            node_entry: Entry::Primary,
        };

        if let Some(watch) = &mut self.watch {
            watch.node_refs.push(cover);
        }

        cover
    }

    pub fn watch(&mut self, enabled: bool) {
        if enabled && self.watch.is_none() {
            self.watch = Some(WatchReport::new());
            return;
        }

        if !enabled && self.watch.is_some() {
            self.watch = None;
            return;
        }
    }

    pub fn watches(&self) -> bool {
        self.watch.is_some()
    }

    #[inline(always)]
    pub fn report(&mut self) -> Option<WatchReport> {
        let report = self.watch.as_mut()?;

        Some(replace(report, WatchReport::new()))
    }

    #[inline(always)]
    pub(super) fn tree(&self) -> &Tree<N> {
        &self.tree
    }

    fn cluster_cover(&self, span: &SiteSpan) -> ClusterRef {
        let mut chunk_cursor = match span.start == 0 && span.end == self.tree.code_length() {
            true => ChildCursor::dangling(),

            false => {
                let mut start = span.start;

                let mut chunk_cursor = self.tree.lookup(&mut start);

                if start == 0 && !chunk_cursor.is_dangling() {
                    unsafe { chunk_cursor.back() }
                }

                chunk_cursor
            }
        };

        while !chunk_cursor.is_dangling() {
            if let Some(cache) = unsafe { chunk_cursor.cache() } {
                if let Some(end) = unsafe { cache.end_site(self) } {
                    if end >= span.end {
                        let cluster_index = unsafe { chunk_cursor.cache_index() };

                        return ClusterRef {
                            id: self.id,
                            cluster_entry: unsafe {
                                self.references.clusters().entry_of(cluster_index)
                            },
                        };
                    }
                }
            }

            unsafe { chunk_cursor.back() };
        }

        ClusterRef {
            id: self.id,
            cluster_entry: Entry::Primary,
        }
    }

    fn update_lexis(&mut self, mut span: SiteSpan, text: &str) -> Cover<N> {
        let mut head;
        let mut head_offset;
        let mut tail;
        let mut tail_offset;

        match span.start == span.end {
            false => {
                head_offset = span.start;
                head = self.tree.lookup(&mut head_offset);
                tail_offset = span.end;
                tail = self.tree.lookup(&mut tail_offset);
            }

            true => {
                head_offset = span.start;
                head = self.tree.lookup(&mut head_offset);
                tail_offset = head_offset;
                tail = head;
            }
        }

        let mut input = Vec::with_capacity(3);

        if head_offset > 0 {
            debug_assert!(
                !head.is_dangling(),
                "Dangling reference with non-zero offset.",
            );

            input.push(split_left(unsafe { head.string() }, head_offset));

            span.start -= head_offset;
        } else {
            let moved = match head.is_dangling() {
                false => match unsafe { !head.is_first() } {
                    true => {
                        unsafe { head.back() }
                        true
                    }

                    false => false,
                },

                true => {
                    head = self.tree.last();

                    !head.is_dangling()
                }
            };

            if moved {
                let head_string = unsafe { head.string() };
                let head_span = unsafe { *head.span() };

                input.push(head_string);

                span.start -= head_span;
            }
        }

        if !text.is_empty() {
            input.push(text);
        }

        if tail_offset > 0 {
            debug_assert!(
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
            debug_assert!(
                product.tail.is_dangling(),
                "Dangling head and non-dangling tail.",
            );

            let token_count = product.count() - skip;

            let tail_tree = unsafe {
                Tree::from_chunks(
                    &mut self.references,
                    token_count,
                    product.spans.into_iter().skip(skip),
                    product.indices.into_iter().skip(skip),
                    product.tokens.into_iter().skip(skip),
                    product.text.as_str(),
                )
            };

            let insert_span = tail_tree.code_length();

            unsafe { self.tree.join(&mut self.references, tail_tree) };

            self.token_count += token_count;

            let chunk_cursor = {
                let mut point = span.start;

                let chunk_cursor = self.tree.lookup(&mut point);

                debug_assert_eq!(point, 0, "Bad span alignment.");

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
                        &mut self.references,
                        head,
                        remove_count,
                        insert_count,
                        product.spans.into_iter().skip(skip),
                        unsafe { product.indices.get_unchecked(skip..) },
                        product.tokens.into_iter().skip(skip),
                        product.text.as_str(),
                    )
                };

                self.token_count += insert_count;
                self.token_count -= remove_count;

                return Cover {
                    chunk_cursor,
                    span: span.start..(span.start + insert_span),
                };
            }
        }

        let mut middle = unsafe { self.tree.split(&mut self.references, head) };

        let middle_split_point = {
            let mut point = span.end - span.start;

            let chunk_cursor = middle.lookup(&mut point);

            debug_assert_eq!(point, 0, "Bad span alignment.");

            chunk_cursor
        };

        let right = unsafe { middle.split(&mut self.references, middle_split_point) };

        let remove_count;
        let insert_span;

        {
            let replacement = unsafe {
                Tree::from_chunks(
                    &mut self.references,
                    insert_count,
                    product.spans.into_iter().skip(skip),
                    product.indices.into_iter().skip(skip),
                    product.tokens.into_iter().skip(skip),
                    product.text.as_str(),
                )
            };

            insert_span = replacement.code_length();

            remove_count =
                unsafe { replace(&mut middle, replacement).free_as_subtree(&mut self.references) };
        };

        unsafe { self.tree.join(&mut self.references, middle) };
        unsafe { self.tree.join(&mut self.references, right) };

        self.token_count += insert_count;
        self.token_count -= remove_count;

        head = {
            let mut point = span.start;

            let chunk_cursor = self.tree.lookup(&mut point);

            debug_assert_eq!(point, 0, "Bad span alignment.");

            chunk_cursor
        };

        Cover {
            chunk_cursor: head,
            span: span.start..(span.start + insert_span),
        }
    }

    fn update_syntax(&mut self, mut cover: Cover<N>) -> Entry {
        #[allow(unused_variables)]
        let mut cover_lookahead = 0;

        loop {
            let mut shift;
            let mut rule;

            match cover.chunk_cursor.is_dangling() {
                false => match unsafe { cover.chunk_cursor.is_first() } {
                    true => {
                        shift = 0;
                        rule = ROOT_RULE;
                    }

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

                            Some(cache_cluster) => {
                                let parse_end_site = unsafe { cache_cluster.end_site(self) };

                                if let Some(parse_end_site) = parse_end_site {
                                    if parse_end_site + cache_cluster.lookahead < cover.span.start {
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
                                            cover_lookahead = cache_cluster.lookahead;
                                        }
                                        rule = cache_cluster.rule;
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    let entry_index = unsafe { cover.chunk_cursor.remove_cache() };

                    unsafe { self.references.clusters_mut().remove_unchecked(entry_index) };
                }
            }

            match rule == ROOT_RULE {
                false => {
                    let cluster_entry = unsafe {
                        let cluster_entry_index = cover.chunk_cursor.cache_index();

                        self.references.clusters_mut().entry_of(cluster_entry_index)
                    };

                    let (cluster_cache, parsed_end_site, _lookahead) = unsafe {
                        MutableSyntaxSession::run(
                            self.id,
                            &mut self.tree,
                            &mut self.references,
                            self.watch.as_mut(),
                            rule,
                            cover.span.start,
                            cover.chunk_cursor,
                            cluster_entry,
                        )
                    };

                    unsafe { cover.chunk_cursor.update_cache(cluster_cache) };

                    //todo check lookahead too
                    if cover.span.end == parsed_end_site {
                        return cluster_entry;
                    }

                    cover.span.end = cover.span.end.max(parsed_end_site);
                }

                true => {
                    let head = self.tree.first();

                    let (cluster_cache, mut parsed_end_site, _lookahead) = unsafe {
                        MutableSyntaxSession::run(
                            self.id,
                            &mut self.tree,
                            &mut self.references,
                            self.watch.as_mut(),
                            ROOT_RULE,
                            0,
                            head,
                            Entry::Primary,
                        )
                    };

                    self.root_cluster = cluster_cache.cluster;

                    let mut tail = self.tree.lookup(&mut parsed_end_site);

                    debug_assert_eq!(parsed_end_site, 0, "Incorrect span alignment.");

                    while !tail.is_dangling() {
                        let has_cache = unsafe { tail.cache().is_some() };

                        if has_cache {
                            let entry_index = unsafe { tail.remove_cache() };

                            unsafe { self.references.clusters_mut().remove_unchecked(entry_index) };
                        }

                        unsafe { tail.next() }
                    }

                    return Entry::Primary;
                }
            }
        }
    }

    // Safety:
    // 1. All references of the `tree` belong to `references` instance.
    #[inline(always)]
    fn initial_parse<'unit>(
        id: Id,
        tree: &'unit mut Tree<N>,
        references: &'unit mut References<N>,
        watch: &'unit mut Option<WatchReport>,
    ) -> Cluster<N> {
        let head = tree.first();

        let (cluster_cache, _parsed_end_site, _lookahead) = unsafe {
            MutableSyntaxSession::run(
                id,
                tree,
                references,
                watch.as_mut(),
                ROOT_RULE,
                0,
                head,
                Entry::Primary,
            )
        };

        if let Some(watch) = watch {
            let cover = NodeRef {
                id,
                cluster_entry: Entry::Primary,
                node_entry: Entry::Primary,
            };

            watch.node_refs.push(cover);
        }

        cluster_cache.cluster
    }
}

impl<T: Token> TokenBuffer<T> {
    /// Turns this buffer into incremental [Document](crate::Document) instance.
    ///
    /// Generic parameter `N` of type [Node](crate::syntax::Node) specifies source code syntax
    /// grammar. Node's [Token](crate::syntax::Node::Token) associative type must be compatible with
    /// the TokenBuffer Token type. In other words, Document's syntax structure must be compatible
    /// with the TokenBuffer's lexical structure.
    ///
    /// ```rust
    /// use lady_deirdre::{units::Document, lexis::{TokenBuffer, SimpleToken}, syntax::SimpleNode};
    ///
    /// let buf = TokenBuffer::<SimpleToken>::from("foo [bar]");
    ///
    /// // SimpleNode syntax uses SimpleToken's lexis.
    /// let _doc = buf.into_document::<SimpleNode>();
    /// ```
    #[inline(always)]
    pub fn into_mutable_unit<N>(self) -> MutableUnit<N>
    where
        N: Node<Token = T>,
    {
        MutableUnit::from_token_buffer(self, false)
    }
}

pub struct WatchReport {
    pub node_refs: Vec<NodeRef>,
    pub error_refs: Vec<ErrorRef>,
}

impl WatchReport {
    #[inline(always)]
    fn new() -> Self {
        Self {
            node_refs: Vec::with_capacity(REPORT_CAPACITY),
            error_refs: Vec::with_capacity(REPORT_CAPACITY),
        }
    }
}

struct Cover<N: Node> {
    chunk_cursor: ChildCursor<N>,
    span: SiteSpan,
}

impl<N: Node> ClusterCache<N> {
    // Safety:
    // 1. ClusterCache belongs to specified `document` instance.
    #[inline(always)]
    pub(super) unsafe fn jump_to_end(
        &self,
        tree: &Tree<N>,
        references: &References<N>,
    ) -> (Site, ChildCursor<N>) {
        match self.parsed_end.inner() {
            SiteRefInner::ChunkStart(token_ref) => {
                let chunk_entry_index = match &token_ref.chunk_entry {
                    Entry::Repo { index, .. } => *index,

                    // Safety: Document chunks stored in Repository.
                    _ => unsafe {
                        debug_unreachable!("Incorrect cluster cache end site Ref type.")
                    },
                };

                let chunk_cursor = unsafe { references.chunks().get_unchecked(chunk_entry_index) };

                let site = unsafe { tree.site_of(chunk_cursor) };

                (site, *chunk_cursor)
            }

            SiteRefInner::CodeEnd(_) => (tree.code_length(), ChildCursor::dangling()),
        }
    }

    // Safety:
    // 1. ClusterCache belongs to specified `document` instance.
    #[inline(always)]
    unsafe fn end_site(&self, unit: &MutableUnit<N>) -> Option<Site> {
        match self.parsed_end.inner() {
            SiteRefInner::ChunkStart(token_ref) => {
                let chunk_cursor = unit.references.chunks().get(&token_ref.chunk_entry)?;

                Some(unsafe { unit.tree.site_of(chunk_cursor) })
            }

            SiteRefInner::CodeEnd(_) => Some(unit.tree.code_length()),
        }
    }
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
