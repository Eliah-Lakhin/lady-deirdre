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
    arena::{Id, Identifiable, Ref, RefIndex, Repository},
    lexis::{
        Length,
        Site,
        SiteRef,
        SiteSpan,
        SourceCode,
        ToSite,
        ToSpan,
        TokenCount,
        TokenCursor,
        TokenRef,
    },
    std::*,
    syntax::{
        buffer::BufferErrorIterator,
        Cluster,
        ErrorRef,
        Node,
        NodeRef,
        SyntaxRule,
        SyntaxSession,
        SyntaxTree,
        ROOT_RULE,
    },
};

/// An interface to transform the source code text into different representation.
///
/// Basically, Transducer is a function(a [map](Transducer::map) function) that temporary
/// interrupts Syntax Parser parse process by invoking on every parse rule application.
/// On every invocation the function manages parse results of the currently parsed rule and its
/// branch down the tree using mutable reference to the [ParseContext] object. This function
/// computes and returns a new representation of this parse rule results to be further utilized by
/// this function on the next invocation steps. In this sense the `map` function depth-first
/// traverses virtual parse tree.
///
/// The final invocation result of the `map` function is the result of the Transducer application.
///
/// Transducers framework provides an API user with the mechanism to implement such tools as
/// Code Formatters and the Parse Tree rewriters. In particular, an API user could utilize this
/// framework to construct custom forms of (Abstract) Syntax Tree that would be compatible with
/// the 3rd party code analysis libraries.
///
/// ```rust
/// use lady_deirdre::{
///     syntax::{SimpleNode, TransduceRef, ParseContext},
///     lexis::SourceCode,
///     Document,
/// };
///
/// // This example shows how to print a system of nested parens.
///
/// let doc = Document::<SimpleNode>::from("foo [bar] ({baz} aaa (bbb))");
///
/// // `SourceCode::transduce` is an entry point of the Transduce process. In particular, an FnMut
/// // function implements Transducer interface too.
///
/// let result = doc.transduce(|context: &mut ParseContext<_, _, String>| {
///     // `ParseContext::node` function returns a reference to the currently parsed Node.
///     let node = context.node();
///
///     let mut result = String::new();
///
///     match node {
///         SimpleNode::Root { .. } => (),
///         SimpleNode::Parenthesis { .. } => result.push('('),
///         SimpleNode::Brackets { .. } => result.push('['),
///         SimpleNode::Braces { .. } => result.push('{'),
///     }
///
///     for inner_node in node.inner() {
///         // `TransduceRef::get` function returns results of the `map` function invocation
///         // previously called for any inner node of the currently parsed Parse Tree branch.
///         result.push_str(inner_node.get(context).unwrap().as_str());
///     }
///
///     match node {
///         SimpleNode::Root { .. } => (),
///         SimpleNode::Parenthesis { .. } => result.push(')'),
///         SimpleNode::Brackets { .. } => result.push(']'),
///         SimpleNode::Braces { .. } => result.push('}'),
///     }
///
///     result
/// });
///
/// assert_eq!(result, "[]({}())");
/// ```
pub trait Transducer<N: Node, S: SourceCode<Token = N::Token>, R> {
    /// A function that transforms particular parse tree node into the target representation type.
    fn map(&mut self, context: &mut ParseContext<N, S, R>) -> R;
}

impl<N, S, R, F> Transducer<N, S, R> for F
where
    N: Node,
    S: SourceCode<Token = N::Token>,
    F: FnMut(&mut ParseContext<N, S, R>) -> R,
{
    #[inline(always)]
    fn map(&mut self, context: &mut ParseContext<N, S, R>) -> R {
        self(context)
    }
}

/// A Transducer's parse context.
///
/// This object passed to the [`Transducer::map`](Transducer::map) function.
///
/// ParseContext provides generic interface to inspect currently parsed node and its branch down the
/// parse tree. The context includes the reference of the [node](ParseContext::node),
/// the [span](ParseContext::node_span) of the node, and the [cursor](ParseContext::node_cursor)
/// into all tokens chunks covered by this parse rule.
///
/// Additionally, ParseContext implements [SourceCode](crate::lexis::SourceCode) and
/// [SyntaxTree](crate::syntax::SyntaxTree) traits such that an API user can use ParseContext to
/// dereference any weakly referred [NodeRef](crate::syntax::NodeRef),
/// [TokenRef](crate::lexis::TokenRef) or any other weakly referred object already parsed by the
/// Syntax Parser and the Lexis Scanner. In particular an API user can use this object to
/// dereference weak references inside [Nodes](crate::lexis::Node) of the currently parsed parse
/// tree branch.
///
/// Note, however, by design SourceCode and SyntaxTree implementations of the ParseContext object
/// provide immutable access capabilities of these interfaces only. As such an API user
/// cannot mutably dereference any of these weak references.
///
/// Finally, an API user could use ParseContext to obtain the computation results of the
/// [`Transducer::map`](Transducer::map) function applied to all nodes of currently parsed tree
/// branch node down the parse tree. For this purpose an API user utilizes [TransduceRef] interface
/// that auto-extends normal [NodeRef](crate::syntax::NodeRef) weak references.
pub struct ParseContext<'code, N: Node, S: SourceCode<Token = N::Token>, R> {
    code: &'code S,
    root: NodeRef,
    cluster: Option<(SiteSpan, Cluster<N>)>,
    data: Vec<(SiteSpan, R)>,
}

impl<'code, N, S, R> Identifiable for ParseContext<'code, N, S, R>
where
    N: Node,
    S: SourceCode<Token = N::Token>,
{
    #[inline(always)]
    fn id(&self) -> &Id {
        self.code.id()
    }
}

impl<'code, N, S, R> SourceCode for ParseContext<'code, N, S, R>
where
    N: Node,
    S: SourceCode<Token = N::Token>,
{
    type Token = N::Token;

    type Cursor<'a> = S::Cursor<'a> where Self: 'a;

    #[inline(always)]
    fn contains(&self, chunk_ref: &Ref) -> bool {
        self.code.contains(chunk_ref)
    }

    #[inline(always)]
    fn get_token(&self, chunk_ref: &Ref) -> Option<&Self::Token> {
        self.code.get_token(chunk_ref)
    }

    #[inline(always)]
    fn get_token_mut(&mut self, _chunk_ref: &Ref) -> Option<&mut Self::Token> {
        None
    }

    #[inline(always)]
    fn get_site(&self, chunk_ref: &Ref) -> Option<Site> {
        self.code.get_site(chunk_ref)
    }

    #[inline(always)]
    fn get_string(&self, chunk_ref: &Ref) -> Option<&str> {
        self.code.get_string(chunk_ref)
    }

    #[inline(always)]
    fn get_length(&self, chunk_ref: &Ref) -> Option<Length> {
        self.code.get_length(chunk_ref)
    }

    #[inline(always)]
    fn cursor(&self, span: impl ToSpan) -> Self::Cursor<'_> {
        self.code.cursor(span)
    }

    #[inline(always)]
    fn length(&self) -> Length {
        self.code.length()
    }

    #[inline(always)]
    fn token_count(&self) -> TokenCount {
        self.code.token_count()
    }
}

impl<'code, N, S, R> SyntaxTree for ParseContext<'code, N, S, R>
where
    N: Node,
    S: SourceCode<Token = N::Token>,
{
    type Node = N;

    type ErrorIterator<'a> = BufferErrorIterator<'a, Self::Node> where Self: 'a;

    #[inline(always)]
    fn root(&self) -> &NodeRef {
        &self.root
    }

    #[inline(always)]
    fn errors(&self) -> Self::ErrorIterator<'_> {
        let (_, cluster) = unsafe { self.cluster.as_ref().unwrap_unchecked() };

        BufferErrorIterator {
            id: self.code.id(),
            inner: (&cluster.errors).into_iter(),
        }
    }

    #[inline(always)]
    fn contains(&self, cluster_ref: &Ref) -> bool {
        match cluster_ref {
            Ref::Primary => true,
            _ => false,
        }
    }

    #[inline(always)]
    fn get_cluster(&self, cluster_ref: &Ref) -> Option<&Cluster<Self::Node>> {
        match cluster_ref {
            Ref::Primary => {
                let (_, cluster) = unsafe { self.cluster.as_ref().unwrap_unchecked() };
                Some(cluster)
            }

            _ => None,
        }
    }

    #[inline(always)]
    fn get_cluster_mut(&mut self, _cluster_ref: &Ref) -> Option<&mut Cluster<Self::Node>> {
        None
    }
}

impl<'code, N, S, R> ParseContext<'code, N, S, R>
where
    N: Node,
    S: SourceCode<Token = N::Token>,
{
    /// Returns a reference of the [Node](crate::syntax::Node) belong to the currently parsed rule.
    #[inline(always)]
    pub fn node(&self) -> &N {
        let (_, cluster) = unsafe { self.cluster.as_ref().unwrap_unchecked() };

        &cluster.primary
    }

    /// Returns a [SiteSpan](crate::lexis::SiteSpan) covered by the currently parsed rule.
    #[inline(always)]
    pub fn node_span(&self) -> SiteSpan {
        let (span, _) = unsafe { self.cluster.as_ref().unwrap_unchecked() };

        span.clone()
    }

    /// Returns a [TokenCursor](crate::lexis::TokenCursor) through the all
    /// [`token chunks`](crate::lexis::Chunk) covered by the currently parsed rule.
    #[inline(always)]
    pub fn node_cursor(&self) -> <S as SourceCode>::Cursor<'code> {
        self.code.cursor(self.node_span())
    }
}

/// An out-implemented extension of the [NodeRef](crate::syntax::NodeRef) interface for Transducers
/// parse metadata access.
///
/// This interface provides an API user of access functions to the [Node](crate::syntax::Node)'s
/// parse rule metadata constructed during the previous invocation steps of the
/// [`Transducer::map`](Transducer::map) function. An API user utilizes [ParseContext] object
/// to dereference this metadata from the NodeRef weak references.
///
/// See ParseContext [documentation](ParseContext) for details.
pub trait TransduceRef {
    /// Immutably dereferences parse rule metadata received from the
    /// [`Transducer::map`](Transducer::map) function.
    ///
    /// Returns [None] if this NodeRef object does not belong to the parse tree branch specified by
    /// the [`context`](ParseContext) argument.
    fn get<'context, N: Node, S: SourceCode<Token = N::Token>, R>(
        &self,
        context: &'context ParseContext<N, S, R>,
    ) -> Option<&'context R>;

    /// Mutably dereferences parse rule metadata received from the
    /// [`Transducer::map`](Transducer::map) function.
    ///
    /// Returns [None] if this NodeRef object does not belong to the parse tree branch specified by
    /// the [`context`](ParseContext) argument.
    fn get_mut<'context, N: Node, S: SourceCode<Token = N::Token>, R>(
        &self,
        context: &'context mut ParseContext<N, S, R>,
    ) -> Option<&'context mut R>;

    /// Returns a [SiteSpan](crate::lexis::SiteSpan) of the tokens covered by the parse rule this
    /// NodeRef object belongs to.
    ///
    /// Returns [None] if this NodeRef object does not belong to the parse tree branch specified by
    /// the [`context`](ParseContext) argument.
    fn span<N: Node, S: SourceCode<Token = N::Token>, R>(
        &self,
        context: &ParseContext<N, S, R>,
    ) -> Option<SiteSpan>;
}

impl TransduceRef for NodeRef {
    #[inline]
    fn get<'context, N: Node, S: SourceCode<Token = N::Token>, R>(
        &self,
        context: &'context ParseContext<N, S, R>,
    ) -> Option<&'context R> {
        if &self.id != context.id() {
            return None;
        }

        match &self.node_ref {
            Ref::Repository { index, .. } if *index < context.data.len() => unsafe {
                Some(&context.data.get_unchecked(*index).1)
            },

            _ => None,
        }
    }

    #[inline]
    fn get_mut<'context, N: Node, S: SourceCode<Token = N::Token>, R>(
        &self,
        context: &'context mut ParseContext<N, S, R>,
    ) -> Option<&'context mut R> {
        if &self.id != context.id() {
            return None;
        }

        match &self.node_ref {
            Ref::Repository { index, .. } if *index < context.data.len() => unsafe {
                Some(&mut context.data.get_unchecked_mut(*index).1)
            },

            _ => None,
        }
    }

    #[inline]
    fn span<N: Node, S: SourceCode<Token = N::Token>, R>(
        &self,
        context: &ParseContext<N, S, R>,
    ) -> Option<SiteSpan> {
        if &self.id != context.id() {
            return None;
        }

        match &self.node_ref {
            Ref::Repository { index, .. } if *index < context.data.len() => unsafe {
                Some(context.data.get_unchecked(*index).0.clone())
            },

            _ => None,
        }
    }
}

struct TransduceSyntaxSession<
    'context,
    'code,
    N: Node,
    S: SourceCode<Token = N::Token>,
    R,
    Tr: Transducer<N, S, R>,
> {
    transducer: &'context mut Tr,
    token_cursor: S::Cursor<'code>,
    context: &'context mut ParseContext<'code, N, S, R>,
    pending_node_index: RefIndex,
    pending_errors: Option<Repository<N::Error>>,
}

impl<'context, 'code, N, S, R, Tr> Identifiable
    for TransduceSyntaxSession<'context, 'code, N, S, R, Tr>
where
    N: Node,
    S: SourceCode<Token = N::Token>,
    Tr: Transducer<N, S, R>,
{
    #[inline(always)]
    fn id(&self) -> &Id {
        self.context.id()
    }
}

impl<'context, 'code, N, S, R, Tr> TokenCursor<'code>
    for TransduceSyntaxSession<'context, 'code, N, S, R, Tr>
where
    N: Node,
    S: SourceCode<Token = N::Token>,
    Tr: Transducer<N, S, R>,
{
    type Token = <N as Node>::Token;

    #[inline(always)]
    fn advance(&mut self) -> bool {
        self.token_cursor.advance()
    }

    #[inline(always)]
    fn token(&mut self, distance: TokenCount) -> Option<&'code Self::Token> {
        self.token_cursor.token(distance)
    }

    #[inline(always)]
    fn site(&mut self, distance: TokenCount) -> Option<Site> {
        self.token_cursor.site(distance)
    }

    #[inline(always)]
    fn length(&mut self, distance: TokenCount) -> Option<Length> {
        self.token_cursor.length(distance)
    }

    #[inline(always)]
    fn string(&mut self, distance: TokenCount) -> Option<&'code str> {
        self.token_cursor.string(distance)
    }

    #[inline(always)]
    fn token_ref(&mut self, distance: TokenCount) -> TokenRef {
        self.token_cursor.token_ref(distance)
    }

    #[inline(always)]
    fn site_ref(&mut self, distance: TokenCount) -> SiteRef {
        self.token_cursor.site_ref(distance)
    }

    #[inline(always)]
    fn end_site_ref(&mut self) -> SiteRef {
        self.token_cursor.end_site_ref()
    }
}

impl<'context, 'code, N, S, R, Tr> SyntaxSession<'code>
    for TransduceSyntaxSession<'context, 'code, N, S, R, Tr>
where
    N: Node,
    S: SourceCode<Token = N::Token>,
    Tr: Transducer<N, S, R>,
{
    type Node = N;

    fn descend(&mut self, rule: SyntaxRule) -> NodeRef {
        let start = self
            .site_ref(0)
            .to_site(self.context.code)
            .expect("Start SiteRef dereference failure.");
        let node = N::new(rule, self);
        let end = self
            .site_ref(0)
            .to_site(self.context.code)
            .expect("End SiteRef dereference failure.");

        {
            let cluster = match take(&mut self.context.cluster) {
                Some((_, mut cluster)) => {
                    let pending = replace(&mut cluster.primary, node);

                    unsafe {
                        cluster
                            .nodes
                            .set_unchecked(self.pending_node_index, pending)
                    };

                    cluster
                }

                None => {
                    let errors = unsafe { take(&mut self.pending_errors).unwrap_unchecked() };

                    Cluster {
                        primary: node,
                        nodes: Repository::default(),
                        errors,
                    }
                }
            };

            self.context.cluster = Some((start..end, cluster))
        }

        let data = self.transducer.map(self.context);

        let (span, cluster) = unsafe { self.context.cluster.as_mut().unwrap_unchecked() };

        self.pending_node_index = cluster.nodes.reserve();

        assert_eq!(
            self.pending_node_index,
            self.context.data.len(),
            "Internal error. Node repository index and data vector index inconsistency",
        );

        self.context.data.push((span.clone(), data));

        let node_ref = unsafe { cluster.nodes.make_ref(self.pending_node_index) };

        NodeRef {
            id: *self.context.id(),
            cluster_ref: Ref::Primary,
            node_ref,
        }
    }

    #[inline]
    fn error(&mut self, error: <Self::Node as Node>::Error) -> ErrorRef {
        match &mut self.pending_errors {
            None => {
                let id = *self.context.id();

                let (_, cluster) = unsafe { self.context.cluster.as_mut().unwrap_unchecked() };

                ErrorRef {
                    id,
                    cluster_ref: Ref::Primary,
                    error_ref: cluster.errors.insert(error),
                }
            }

            Some(errors) => ErrorRef {
                id: *self.context.id(),
                cluster_ref: Ref::Primary,
                error_ref: errors.insert(error),
            },
        }
    }
}

#[inline]
pub(crate) fn transduce<N, S, R, Tr>(code: &S, mut transducer: Tr) -> R
where
    N: Node,
    S: SourceCode<Token = N::Token>,
    Tr: Transducer<N, S, R>,
{
    let mut context = ParseContext {
        code,
        root: NodeRef::nil(),
        cluster: None,
        data: Vec::with_capacity(1),
    };

    let mut session = TransduceSyntaxSession {
        transducer: &mut transducer,
        token_cursor: code.cursor(..),
        context: &mut context,
        pending_node_index: 0,
        pending_errors: Some(Repository::default()),
    };

    let _ = session.descend(ROOT_RULE);

    let (_, last) = unsafe { session.context.data.pop().unwrap_unchecked() };

    last
}
