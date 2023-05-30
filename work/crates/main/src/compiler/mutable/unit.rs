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
    arena::{Id, Identifiable, Ref},
    compiler::{
        mutable::{
            cursor::MutableCursor,
            lexis::MutableLexisSession,
            storage::{ChildRefIndex, ClusterCache, References, Tree},
            syntax::MutableSyntaxSession,
        },
        CompilationUnit,
    },
    lexis::{
        utils::{split_left, split_right},
        Length,
        Site,
        SiteRef,
        SiteRefInner,
        SiteRefSpan,
        SiteSpan,
        SourceCode,
        ToSpan,
        Token,
        TokenBuffer,
        TokenCount,
        TokenRef,
        CHUNK_SIZE,
    },
    report::{debug_assert, debug_assert_eq, debug_unreachable},
    std::*,
    syntax::{Cluster, ClusterRef, NoSyntax, Node, SyntaxTree, NON_ROOT_RULE, ROOT_RULE},
};

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
///      use lady_deirdre::{Document, syntax::SimpleNode};
///
///      let _ = Document::<SimpleNode>::from("test string");
///     ```
///  2. By initializing an empty Document, and using [write](Document::write) operation on
///     the instance.
///     ```rust
///      use lady_deirdre::{Document, syntax::SimpleNode};
///
///      let mut doc = Document::<SimpleNode>::default();
///      doc.write(.., "test string");
///     ```
///  3. And using dedicated [TokenBuffer](crate::lexis::Tokens) instance to preload large file.
///     ```rust
///      use lady_deirdre::{Document, syntax::SimpleNode, lexis::TokenBuffer};
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
/// use lady_deirdre::{Document, syntax::SimpleNode, lexis::CodeContent};
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
///     Document,
///     lexis::{SourceCode, CodeContent, SimpleToken},
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
///     Document,
///     lexis::{SourceCode, CodeContent, TokenCursor, SimpleToken},
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
///     Document,
///     syntax::{SimpleNode, SyntaxTree, NodeRef, TreeContent},
///     lexis::{CodeContent, ToSpan},
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
///     "[1:3]..[1:4]: Brackets format mismatch. Expected Braces, Brackets, Parenthesis, ']'.",
/// );
///
/// // Syntax Tree is a mutable structure.
/// // Adding artificial empty braces Node to the Root.
/// {
///     let new_node = SimpleNode::Braces { inner: vec![] };
///     let new_node_ref = root_ref.cluster().link_node(&mut doc, new_node);
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
}

// Safety: Tree instance stores data on the heap, and the References instance
//         refers Tree's heap objects only.
unsafe impl<N: Node + Send> Send for MutableUnit<N> {}

// Safety:
//   1. Tree and References data mutations can only happen through
//      the &mut Document exclusive interface that invalidates all other
//      references to the inner data of the Document's Tree.
//   2. All "weak" references are safe indexes into the Document's inner data.
unsafe impl<N: Node + Sync> Sync for MutableUnit<N> {}

impl<N: Node> Drop for MutableUnit<N> {
    fn drop(&mut self) {
        unsafe { self.tree.free() };
    }
}

impl<N: Node> Debug for MutableUnit<N> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("MutableUnit")
            .field("id", &self.id)
            .field("length", &self.tree.length())
            .finish_non_exhaustive()
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

    #[inline(always)]
    fn contains_chunk(&self, chunk_ref: &Ref) -> bool {
        self.references.chunks().contains(chunk_ref)
    }

    #[inline(always)]
    fn get_token(&self, chunk_ref: &Ref) -> Option<Self::Token> {
        let chunk_ref = self.references.chunks().get(chunk_ref)?;

        debug_assert!(
            !chunk_ref.is_dangling(),
            "Dangling chunk ref in the References repository."
        );

        Some(unsafe { chunk_ref.token() })
    }

    #[inline(always)]
    fn get_site(&self, chunk_ref: &Ref) -> Option<Site> {
        let chunk_ref = self.references.chunks().get(chunk_ref)?;

        Some(unsafe { self.tree.site_of(chunk_ref) })
    }

    #[inline(always)]
    fn get_string(&self, chunk_ref: &Ref) -> Option<&str> {
        let chunk_ref = self.references.chunks().get(chunk_ref)?;

        debug_assert!(
            !chunk_ref.is_dangling(),
            "Dangling chunk ref in the References repository."
        );

        Some(unsafe { chunk_ref.string() })
    }

    #[inline(always)]
    fn get_length(&self, chunk_ref: &Ref) -> Option<Length> {
        let chunk_ref = self.references.chunks().get(chunk_ref)?;

        debug_assert!(
            !chunk_ref.is_dangling(),
            "Dangling chunk ref in the References repository."
        );

        Some(*unsafe { chunk_ref.span() })
    }

    #[inline(always)]
    fn cursor(&self, span: impl ToSpan) -> Self::Cursor<'_> {
        let span = match span.to_span(self) {
            None => panic!("Specified span is invalid."),

            Some(span) => span,
        };

        Self::Cursor::new(self, span)
    }

    #[inline(always)]
    fn length(&self) -> Length {
        self.tree.length()
    }

    #[inline(always)]
    fn token_count(&self) -> TokenCount {
        self.token_count
    }
}

impl<N: Node> SyntaxTree for MutableUnit<N> {
    type Node = N;

    #[inline(always)]
    fn cover(&self, span: impl ToSpan) -> ClusterRef {
        let span = match span.to_span(self) {
            None => panic!("Specified span is invalid."),

            Some(span) => span,
        };

        let mut chunk_ref = match span.start == 0 && span.end == self.tree.length() {
            true => ChildRefIndex::dangling(),

            false => {
                let mut start = span.start;

                let mut chunk_ref = self.tree.lookup(&mut start);

                if start == 0 && !chunk_ref.is_dangling() {
                    unsafe { chunk_ref.back() }
                }

                chunk_ref
            }
        };

        while !chunk_ref.is_dangling() {
            if let Some(cache) = unsafe { chunk_ref.cache() } {
                if let Some(end) = unsafe { cache.end_site(self) } {
                    if end >= span.end {
                        let cluster_index = unsafe { chunk_ref.cache_index() };

                        return ClusterRef {
                            id: self.id,
                            cluster_ref: unsafe {
                                self.references.clusters().make_ref(cluster_index)
                            },
                        };
                    }
                }
            }

            unsafe { chunk_ref.back() };
        }

        ClusterRef {
            id: self.id,
            cluster_ref: Ref::Primary,
        }
    }

    #[inline(always)]
    fn contains_cluster(&self, cluster_ref: &Ref) -> bool {
        match cluster_ref {
            Ref::Primary => true,

            Ref::Repository { .. } => self.references.clusters().contains(cluster_ref),

            _ => false,
        }
    }

    #[inline(always)]
    fn get_cluster(&self, cluster_ref: &Ref) -> Option<&Cluster<Self::Node>> {
        match cluster_ref {
            Ref::Primary => Some(&self.root_cluster),

            Ref::Repository { .. } => {
                let chunk_ref = self.references.clusters().get(cluster_ref)?;

                let cluster_cache = unsafe { chunk_ref.cache()? };

                Some(&cluster_cache.cluster)
            }

            _ => None,
        }
    }

    #[inline(always)]
    fn get_cluster_mut(&mut self, cluster_ref: &Ref) -> Option<&mut Cluster<Self::Node>> {
        match cluster_ref {
            Ref::Primary => Some(&mut self.root_cluster),

            Ref::Repository { .. } => {
                let mut chunk_ref = *self.references.clusters().get(cluster_ref)?;

                let cluster_cache = unsafe { chunk_ref.cache_mut()? };

                Some(&mut cluster_cache.cluster)
            }

            _ => None,
        }
    }

    #[inline]
    fn get_cluster_span(&self, cluster_ref: &Ref) -> SiteRefSpan {
        match cluster_ref {
            Ref::Primary => {
                let first_chunk = self.tree.first();

                let end = self.end_site_ref();

                let start = match first_chunk.is_dangling() {
                    true => end,

                    false => {
                        let ref_index = unsafe { first_chunk.chunk_ref_index() };

                        TokenRef {
                            id: self.id,
                            chunk_ref: unsafe { self.references.chunks().make_ref(ref_index) },
                        }
                        .site_ref()
                    }
                };

                return start..self.end_site_ref();
            }

            Ref::Repository { .. } => {
                if let Some(child_ref) = self.references.clusters().get(cluster_ref) {
                    if let Some(cluster_cache) = unsafe { child_ref.cache() } {
                        let start = {
                            let ref_index = unsafe { child_ref.chunk_ref_index() };

                            TokenRef {
                                id: self.id,
                                chunk_ref: unsafe { self.references.chunks().make_ref(ref_index) },
                            }
                            .site_ref()
                        };

                        return start..cluster_cache.parsed_end;
                    }
                }
            }

            _ => (),
        }

        SiteRef::nil()..SiteRef::nil()
    }

    #[inline(always)]
    fn get_previous_cluster(&self, cluster_ref: &Ref) -> Ref {
        if let Some(mut chunk_ref) = self.references.clusters().get(cluster_ref).copied() {
            while !chunk_ref.is_dangling() {
                unsafe { chunk_ref.back() }

                if chunk_ref.is_dangling() {
                    return Ref::Primary;
                }

                if unsafe { chunk_ref.cache() }.is_some() {
                    let ref_index = unsafe { chunk_ref.cache_index() };

                    return unsafe { self.references.clusters().make_ref(ref_index) };
                }
            }
        }

        Ref::Nil
    }

    #[inline(always)]
    fn get_next_cluster(&self, cluster_ref: &Ref) -> Ref {
        let mut chunk_ref = match cluster_ref {
            Ref::Primary => self.tree.first(),

            Ref::Repository { .. } => match self.references.clusters().get(cluster_ref).copied() {
                None => return Ref::Nil,
                Some(child_ref) => child_ref,
            },

            _ => return Ref::Nil,
        };

        while !chunk_ref.is_dangling() {
            unsafe { chunk_ref.next() };

            if chunk_ref.is_dangling() {
                break;
            }

            if unsafe { chunk_ref.cache() }.is_some() {
                let ref_index = unsafe { chunk_ref.cache_index() };

                return unsafe { self.references.clusters().make_ref(ref_index) };
            }
        }

        Ref::Nil
    }

    #[inline(always)]
    fn remove_cluster(&mut self, cluster_ref: &Ref) -> Option<Cluster<Self::Node>> {
        let chunk_ref = self.references.clusters_mut().remove(cluster_ref)?;

        let cache = unsafe { chunk_ref.take_cache() };

        Some(cache.cluster)
    }
}

impl<N: Node> Default for MutableUnit<N> {
    #[inline(always)]
    fn default() -> Self {
        let id = Id::new();
        let mut tree = Tree::default();
        let mut references = References::default();

        let root_cluster = Self::initial_parse(id, &mut tree, &mut references);

        Self {
            id,
            root_cluster,
            tree,
            token_count: 0,
            references,
        }
    }
}

impl<N, S> From<S> for MutableUnit<N>
where
    N: Node,
    S: Borrow<str>,
{
    #[inline(always)]
    fn from(string: S) -> Self {
        let mut buffer = TokenBuffer::<N::Token>::default();

        buffer.append(string.borrow());

        buffer.into_mutable_unit()
    }
}

impl<N: Node> CompilationUnit for MutableUnit<N> {
    #[inline(always)]
    fn is_mutable(&self) -> bool {
        true
    }

    fn into_token_buffer(self) -> TokenBuffer<N::Token> {
        let mut buffer = TokenBuffer::with_capacity(self.token_count);

        buffer.set_length(self.length());

        let mut chunk_ref = self.tree.first();

        while !chunk_ref.is_dangling() {
            unsafe {
                chunk_ref.take_lexis(&mut buffer.spans, &mut buffer.strings, &mut buffer.tokens)
            };

            unsafe { chunk_ref.next() }
        }

        let _ = self;

        buffer
    }

    #[inline(always)]
    fn into_mutable_unit(self) -> MutableUnit<N> {
        self
    }
}

impl<N: Node> MutableUnit<N> {
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
    /// use lady_deirdre::{Document, lexis::CodeContent, syntax::SimpleNode};
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
    pub fn write(&mut self, span: impl ToSpan, text: impl AsRef<str>) {
        let span = match span.to_span(self) {
            None => panic!("Specified span is invalid."),

            Some(span) => span,
        };

        let text = text.as_ref();

        if span.is_empty() && text.is_empty() {
            return;
        }

        let cursor = self.update_lexis(span, text);

        if TypeId::of::<N>() == TypeId::of::<NoSyntax<<N as Node>::Token>>() {
            return;
        }

        self.update_syntax(cursor);
    }

    #[inline(always)]
    pub(super) fn tree(&self) -> &Tree<N> {
        &self.tree
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

        let mut product =
            unsafe { MutableLexisSession::run(text.len() / CHUNK_SIZE + 2, &input, tail) };

        span.end += product.tail_length;

        let mut skip = 0;

        loop {
            if head.is_dangling() {
                break;
            }

            if unsafe { head.same_chunk_as(&product.tail_ref) } {
                break;
            }

            let product_string = match product.strings.get(skip) {
                Some(string) => string.as_str(),
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

            if unsafe { head.same_chunk_as(&product.tail_ref) } {
                break;
            }

            let last = match product.tail_ref.is_dangling() {
                false => {
                    let mut previous = product.tail_ref;

                    unsafe { previous.back() };

                    previous
                }

                true => self.tree.last(),
            };

            if last.is_dangling() {
                break;
            }

            let product_string = match product.strings.last() {
                Some(string) => string.as_str(),
                None => break,
            };

            let last_string = unsafe { last.string() };

            if product_string == last_string {
                let last_span = unsafe { *last.span() };

                span.end -= last_span;

                let _ = product.spans.pop();
                let _ = product.strings.pop();
                let _ = product.tokens.pop();

                product.length -= last_span;
                product.tail_ref = last;

                continue;
            }

            break;
        }

        if head.is_dangling() {
            debug_assert!(
                product.tail_ref.is_dangling(),
                "Dangling head and non-dangling tail.",
            );

            let token_count = product.count() - skip;

            let tail_tree = unsafe {
                Tree::from_chunks(
                    &mut self.references,
                    token_count,
                    product.spans.into_iter().skip(skip),
                    product.strings.into_iter().skip(skip),
                    product.tokens.into_iter().skip(skip),
                )
            };

            let insert_span = tail_tree.length();

            unsafe { self.tree.join(&mut self.references, tail_tree) };

            self.token_count += token_count;

            let chunk_ref = {
                let mut point = span.start;

                let chunk_ref = self.tree.lookup(&mut point);

                debug_assert_eq!(point, 0, "Bad span alignment.");

                chunk_ref
            };

            return Cover {
                chunk_ref,
                span: span.start..(span.start + insert_span),
                lookahead: 0,
            };
        }

        let insert_count = product.count() - skip;

        if let Some(remove_count) = unsafe { head.continuous_to(&product.tail_ref) } {
            if unsafe { self.tree.is_writeable(&head, remove_count, insert_count) } {
                let (chunk_ref, insert_span) = unsafe {
                    self.tree.write(
                        &mut self.references,
                        head,
                        remove_count,
                        insert_count,
                        product.spans.into_iter().skip(skip),
                        product.strings.into_iter().skip(skip),
                        product.tokens.into_iter().skip(skip),
                    )
                };

                self.token_count += insert_count;
                self.token_count -= remove_count;

                return Cover {
                    chunk_ref,
                    span: span.start..(span.start + insert_span),
                    lookahead: 0,
                };
            }
        }

        let mut middle = unsafe { self.tree.split(&mut self.references, head) };

        let middle_split_point = {
            let mut point = span.end - span.start;

            let chunk_ref = middle.lookup(&mut point);

            debug_assert_eq!(point, 0, "Bad span alignment.");

            chunk_ref
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
                    product.strings.into_iter().skip(skip),
                    product.tokens.into_iter().skip(skip),
                )
            };

            insert_span = replacement.length();

            remove_count =
                unsafe { replace(&mut middle, replacement).free_as_subtree(&mut self.references) };
        };

        unsafe { self.tree.join(&mut self.references, middle) };
        unsafe { self.tree.join(&mut self.references, right) };

        self.token_count += insert_count;
        self.token_count -= remove_count;

        head = {
            let mut point = span.start;

            let chunk_ref = self.tree.lookup(&mut point);

            debug_assert_eq!(point, 0, "Bad span alignment.");

            chunk_ref
        };

        Cover {
            chunk_ref: head,
            span: span.start..(span.start + insert_span),
            lookahead: 0,
        }
    }

    fn update_syntax(&mut self, mut cover: Cover<N>) {
        loop {
            let mut shift;
            let mut rule;

            match cover.chunk_ref.is_dangling() {
                false => match unsafe { cover.chunk_ref.is_first() } {
                    true => {
                        shift = 0;
                        rule = ROOT_RULE;
                    }

                    false => {
                        unsafe { cover.chunk_ref.back() };

                        shift = unsafe { *cover.chunk_ref.span() };

                        rule = NON_ROOT_RULE;
                    }
                },

                true => match self.tree.length() == 0 {
                    true => {
                        shift = 0;
                        rule = ROOT_RULE;
                    }

                    false => {
                        cover.chunk_ref = self.tree.last();

                        shift = unsafe { *cover.chunk_ref.span() };

                        rule = NON_ROOT_RULE;
                    }
                },
            }

            if rule != ROOT_RULE {
                loop {
                    {
                        match unsafe { cover.chunk_ref.cache() } {
                            None => {
                                unsafe { cover.chunk_ref.back() };

                                match cover.chunk_ref.is_dangling() {
                                    false => {
                                        shift += unsafe { *cover.chunk_ref.span() };
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
                                        unsafe { cover.chunk_ref.back() };

                                        match cover.chunk_ref.is_dangling() {
                                            false => {
                                                shift += unsafe { *cover.chunk_ref.span() };
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
                                        cover.lookahead = cache_cluster.lookahead;
                                        rule = cache_cluster.rule;
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    let ref_index = unsafe { cover.chunk_ref.remove_cache() };

                    unsafe { self.references.clusters_mut().remove_unchecked(ref_index) };
                }
            }

            match rule == ROOT_RULE {
                false => {
                    let cluster_ref = unsafe {
                        let cluster_ref_index = cover.chunk_ref.cache_index();

                        self.references.clusters_mut().make_ref(cluster_ref_index)
                    };

                    let (cluster_cache, parsed_end_site, _lookahead) = unsafe {
                        MutableSyntaxSession::run(
                            self.id,
                            &mut self.tree,
                            &mut self.references,
                            rule,
                            cover.span.start,
                            cover.chunk_ref,
                            cluster_ref,
                        )
                    };

                    unsafe { cover.chunk_ref.update_cache(cluster_cache) };

                    //todo check lookahead too
                    if cover.span.end == parsed_end_site {
                        break;
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
                            ROOT_RULE,
                            0,
                            head,
                            Ref::Primary,
                        )
                    };

                    self.root_cluster = cluster_cache.cluster;

                    let mut tail = self.tree.lookup(&mut parsed_end_site);

                    debug_assert_eq!(parsed_end_site, 0, "Incorrect span alignment.");

                    while !tail.is_dangling() {
                        let has_cache = unsafe { tail.cache().is_some() };

                        if has_cache {
                            let ref_index = unsafe { tail.remove_cache() };

                            unsafe { self.references.clusters_mut().remove_unchecked(ref_index) };
                        }

                        unsafe { tail.next() }
                    }

                    break;
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
    ) -> Cluster<N> {
        let head = tree.first();

        let (cluster_cache, _parsed_end_site, _lookahead) = unsafe {
            MutableSyntaxSession::run(id, tree, references, ROOT_RULE, 0, head, Ref::Primary)
        };

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
    /// use lady_deirdre::{Document, lexis::{TokenBuffer, SimpleToken}, syntax::SimpleNode};
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
        let id = Id::new();

        let token_count = self.token_count();
        let spans = self.spans.into_vec().into_iter();
        let strings = self.strings.into_vec().into_iter();
        let tokens = self.tokens.into_vec().into_iter();

        let mut references = References::with_capacity(token_count);

        let mut tree =
            unsafe { Tree::from_chunks(&mut references, token_count, spans, strings, tokens) };

        let root_cluster = MutableUnit::initial_parse(id, &mut tree, &mut references);

        MutableUnit {
            id,
            root_cluster,
            tree,
            token_count,
            references,
        }
    }
}

struct Cover<N: Node> {
    chunk_ref: ChildRefIndex<N>,
    span: SiteSpan,
    lookahead: Length,
}

impl<N: Node> ClusterCache<N> {
    // Safety:
    // 1. ClusterCache belongs to specified `document` instance.
    #[inline(always)]
    pub(super) unsafe fn jump_to_end(
        &self,
        tree: &Tree<N>,
        references: &References<N>,
    ) -> (Site, ChildRefIndex<N>) {
        match self.parsed_end.inner() {
            SiteRefInner::ChunkStart(token_ref) => {
                let chunk_ref_index = match &token_ref.chunk_ref {
                    Ref::Repository { index, .. } => *index,

                    // Safety: Document chunks stored in Repository.
                    _ => unsafe {
                        debug_unreachable!("Incorrect cluster cache end site Ref type.")
                    },
                };

                let chunk_ref = unsafe { references.chunks().get_unchecked(chunk_ref_index) };

                let site = unsafe { tree.site_of(chunk_ref) };

                (site, *chunk_ref)
            }

            SiteRefInner::CodeEnd(_) => (tree.length(), ChildRefIndex::dangling()),
        }
    }

    // Safety:
    // 1. ClusterCache belongs to specified `document` instance.
    #[inline(always)]
    unsafe fn end_site(&self, unit: &MutableUnit<N>) -> Option<Site> {
        match self.parsed_end.inner() {
            SiteRefInner::ChunkStart(token_ref) => {
                let chunk_ref = unit.references.chunks().get(&token_ref.chunk_ref)?;

                Some(unsafe { unit.tree.site_of(chunk_ref) })
            }

            SiteRefInner::CodeEnd(_) => Some(unit.tree.length()),
        }
    }
}
