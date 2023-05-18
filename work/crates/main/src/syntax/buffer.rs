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
    arena::{Id, Identifiable, Ref, Repository, RepositoryIterator},
    lexis::{SiteRef, SiteRefSpan, ToSpan, TokenCursor},
    std::*,
    syntax::{
        session::SequentialSyntaxSession,
        Cluster,
        Node,
        NodeRef,
        SyntaxSession,
        SyntaxTree,
        ROOT_RULE,
    },
};

/// A non-incrementally managed syntax structure of a compilation unit.
///
/// SyntaxBuffer is a simple implementation of the [SyntaxTree](crate::syntax::SyntaxTree) interface
/// that runs a syntax grammar parser over the sequence of tokens just once to produce and to store
/// a syntax structure of a compilation unit. In contrast to [Document](crate::Document),
/// SyntaxBuffer does not provide source code mutation operations(incremental re-parsing
/// operations). However the syntax structure stored by this object is still a mutable structure by
/// itself, an API user can mutate its nodes manually using [Cluster](crate::syntax::Cluster) and
/// similar mutation operations.
///
/// The syntax grammar of the programming language and the syntax structure type specified by the
/// SyntaxBuffer's generic parameter of [Node](crate::syntax::Node) type.
///
/// To crate a SyntaxBuffer use [`Node::parse`](crate::syntax::Node::parse) function.
///
/// ```rust
/// use lady_deirdre::{
///     lexis::{TokenBuffer, SimpleToken, SourceCode, Token},
///     syntax::{SyntaxBuffer, SimpleNode, SyntaxTree, NodeRef, Node, TreeContent},
/// };
///
/// let token_buffer = SimpleToken::parse("foo({bar}[baz])");
/// let syntax_buffer = SimpleNode::parse(token_buffer.cursor(..));
///
/// fn format(tree: &impl SyntaxTree<Node = SimpleNode>, node: &NodeRef) -> String {
///     let node = node.deref(tree).unwrap();
///
///     let inner = node
///         .inner()
///         .iter()
///         .map(|inner_node_ref: &NodeRef| format(tree, inner_node_ref))
///         .collect::<Vec<_>>()
///         .join("");
///
///     match node {
///         SimpleNode::Root { .. } => inner,
///         SimpleNode::Parenthesis { .. } => format!("({})", inner),
///         SimpleNode::Brackets { .. } => format!("[{}]", inner),
///         SimpleNode::Braces { .. } => format!("{{{}}}", inner),
///     }
/// }
///
/// assert_eq!("({}[])", format(&syntax_buffer, &syntax_buffer.root_node_ref()));
/// ```
pub struct SyntaxBuffer<N: Node> {
    id: Id,
    span: SiteRefSpan,
    cluster: Cluster<N>,
}

impl<N: Node> PartialEq for SyntaxBuffer<N> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl<N: Node> Eq for SyntaxBuffer<N> {}

impl<N: Node> Debug for SyntaxBuffer<N> {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("SyntaxBuffer")
            .field("id", &self.id())
            .finish_non_exhaustive()
    }
}

impl<N: Node> Identifiable for SyntaxBuffer<N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<N: Node> SyntaxTree for SyntaxBuffer<N> {
    type Node = N;

    #[inline(always)]
    fn cover(&self, _span: impl ToSpan) -> Ref {
        Ref::Primary
    }

    #[inline(always)]
    fn contains_cluster(&self, cluster_ref: &Ref) -> bool {
        match cluster_ref {
            Ref::Primary => true,
            _ => false,
        }
    }

    #[inline(always)]
    fn get_cluster(&self, cluster_ref: &Ref) -> Option<&Cluster<Self::Node>> {
        match cluster_ref {
            Ref::Primary => Some(&self.cluster),

            _ => None,
        }
    }

    #[inline(always)]
    fn get_cluster_mut(&mut self, cluster_ref: &Ref) -> Option<&mut Cluster<Self::Node>> {
        match cluster_ref {
            Ref::Primary => Some(&mut self.cluster),

            _ => None,
        }
    }

    #[inline(always)]
    fn get_cluster_span(&self, cluster_ref: &Ref) -> SiteRefSpan {
        match cluster_ref {
            Ref::Primary => self.span.clone(),

            _ => SiteRef::nil()..SiteRef::nil(),
        }
    }

    #[inline(always)]
    fn get_previous_cluster(&self, _cluster_ref: &Ref) -> Ref {
        Ref::Nil
    }

    #[inline(always)]
    fn get_next_cluster(&self, _cluster_ref: &Ref) -> Ref {
        Ref::Nil
    }

    #[inline(always)]
    fn remove_cluster(&mut self, _cluster_ref: &Ref) -> Option<Cluster<Self::Node>> {
        None
    }
}

impl<N: Node> SyntaxBuffer<N> {
    pub(crate) fn new<'code>(
        id: Id,
        mut token_cursor: impl TokenCursor<'code, Token = <N as Node>::Token>,
    ) -> Self {
        let start = token_cursor.site_ref(0);

        let mut session = SequentialSyntaxSession {
            id,
            primary: None,
            nodes: Repository::with_capacity(1),
            errors: Repository::default(),
            token_cursor,
            _code_lifetime: Default::default(),
        };

        let _ = session.descend(ROOT_RULE);

        let end = session.token_cursor.site_ref(0);

        let cluster = Cluster {
            primary: unsafe { session.primary.unwrap_unchecked() },
            nodes: session.nodes,
            errors: session.errors,
        };

        Self {
            id,
            span: start..end,
            cluster,
        }
    }
}
