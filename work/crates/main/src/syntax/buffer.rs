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
    lexis::TokenCursor,
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
///     syntax::{SyntaxBuffer, SimpleNode, SyntaxTree, NodeRef, Node},
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
/// assert_eq!("({}[])", format(&syntax_buffer, syntax_buffer.root()));
/// ```
pub struct SyntaxBuffer<N: Node> {
    id: Id,
    root: NodeRef,
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

    type ErrorIterator<'tree> = BufferErrorIterator<'tree, Self::Node>;

    type ClusterIterator<'tree> = BufferClusterIterator<'tree, Self::Node>;

    type ClusterIteratorMut<'tree> = BufferClusterIteratorMut<'tree, Self::Node>;

    #[inline(always)]
    fn root(&self) -> &NodeRef {
        &self.root
    }

    #[inline(always)]
    fn errors(&self) -> Self::ErrorIterator<'_> {
        BufferErrorIterator {
            id: self.id,
            inner: (&self.cluster.errors).into_iter(),
        }
    }

    #[inline(always)]
    fn traverse(&self) -> Self::ClusterIterator<'_> {
        BufferClusterIterator {
            id: self.id,
            cluster: Some(&self.cluster),
        }
    }

    #[inline(always)]
    fn traverse_mut(&mut self) -> Self::ClusterIteratorMut<'_> {
        BufferClusterIteratorMut {
            id: self.id,
            cluster: Some(&mut self.cluster),
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
}

impl<N: Node> SyntaxBuffer<N> {
    pub(super) fn new<'code>(
        token_cursor: impl TokenCursor<'code, Token = <N as Node>::Token>,
    ) -> Self {
        let id = Id::new();

        let mut session = SequentialSyntaxSession {
            id,
            primary: None,
            nodes: Repository::with_capacity(1),
            errors: Repository::default(),
            token_cursor,
            _code_lifetime: Default::default(),
        };

        let root = session.descend(ROOT_RULE);

        let cluster = Cluster {
            primary: unsafe { session.primary.unwrap_unchecked() },
            nodes: session.nodes,
            errors: session.errors,
        };

        Self { id, root, cluster }
    }
}

pub struct BufferErrorIterator<'tree, N: Node> {
    pub(super) id: Id,
    pub(super) inner: RepositoryIterator<'tree, N::Error>,
}

impl<'tree, N: Node> Identifiable for BufferErrorIterator<'tree, N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'tree, N: Node> Iterator for BufferErrorIterator<'tree, N> {
    type Item = &'tree N::Error;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'tree, N: Node> FusedIterator for BufferErrorIterator<'tree, N> {}

pub struct BufferClusterIterator<'tree, N: Node> {
    pub(super) id: Id,
    pub(super) cluster: Option<&'tree Cluster<N>>,
}

impl<'tree, N: Node> Identifiable for BufferClusterIterator<'tree, N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'tree, N: Node> Iterator for BufferClusterIterator<'tree, N> {
    type Item = &'tree Cluster<N>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        take(&mut self.cluster)
    }
}

impl<'tree, N: Node> FusedIterator for BufferClusterIterator<'tree, N> {}

pub struct BufferClusterIteratorMut<'tree, N: Node> {
    pub(super) id: Id,
    pub(super) cluster: Option<&'tree mut Cluster<N>>,
}

impl<'tree, N: Node> Identifiable for BufferClusterIteratorMut<'tree, N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'tree, N: Node> Iterator for BufferClusterIteratorMut<'tree, N> {
    type Item = &'tree mut Cluster<N>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        take(&mut self.cluster)
    }
}

impl<'tree, N: Node> FusedIterator for BufferClusterIteratorMut<'tree, N> {}
