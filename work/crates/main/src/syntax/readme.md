# Syntax analysis features.

This module contains a set of features to construct and analyze syntax
structure of the source code.

The syntax structure of the source code is represented by a syntax tree. The
syntax tree could serve as a Parse Tree, Abstract Syntax Tree, Semantic
resolution structure, and it could also contain syntax and semantic errors
information at the same time.

The Syntax Tree is an abstract mutable structure that could be altered by an
API user at any stage of the end compilation system.

The Tree consists of a set of Nodes connected to each other through the system
of weak references. It is assumed that the [Node](crate::syntax::Node) interface
would be implemented on the Rust enum type with variants
representing kinds of the parse/syntax/semantic tree nodes, and with the variant
fields that would contain weak references and other semantic resolution metadata
to other nodes related to this node(to the child nodes in particular).

The [`Node::new`](crate::syntax::Node::new) function defines a Programming
language syntax grammar parser, an algorithm that constructs the syntax tree
by a sequence of the source code tokens. Under the hood this function performs
parsing of the source code tokens by interacting with the low-level
[SyntaxSession](crate::syntax::SyntaxSession) interface. These two interfaces
could express a parsing algorithm of the `LL(*)` class(unlimited lookahead)
with syntax error recovery capabilities.

An API user is encouraged to utilize a [Node](::lady_deirdre_derive::Node)
derive macro on the enum type to define an `LL(1)` syntax parser. Using this
macro an API user specifies parse rule through the macro attributes directly
on the enum variants. This macro implements a parsing algorithm with error
recovery capabilities using heuristic techniques automatically.

Object that stores compilation unit syntax structure should implement
a [SyntaxTree](crate::syntax::SyntaxTree) trait. This interface provides an API
user with access to the Syntax Tree root node, and to iterator through all
syntax errors of this unit. Unless you work on a Crate extension, you don't need
to implement this trait manually.

[SyntaxBuffer](crate::syntax::SyntaxBuffer) is default implementation of the
SyntaxTree trait. This object supposed to be used for non-incremental parsing
scenario. For incremental parsing one can use [Document](crate::Document) which
is also a SyntaxTree implementation.

The Crate does not propose a unified way to traverse syntax structure of a
compilation unit. An API user receives a weak reference to the root node of the
syntax tree using [`SyntaxTree::root`](crate::syntax::SyntaxTree::root)
function. Actual traversing approaches are up to the user-defined Node type
structure.

The instances of the syntax tree nodes and the instances of the syntax/semantic
errors related to these nodes reside in memory in so called
[Clusters](crate::syntax::Cluster) that own these instances in undetermined
order. A set of such clusters build up the entire syntax structure of the
compilation unit. It is up to the [SyntaxTree](crate::syntax::SyntaxTree)
implementation on how to split the syntax structure between a set of clusters.
For example, the SyntaxBuffer stores the entire syntax structure in a single
cluster, whereas the Document splits syntax structure between a set of clusters
more granularly using each cluster as a unit of incremental caching. The
Document tends to group a set of nodes in a single cluster when all nodes
of this cluster has been produced by a single syntax parser rule(e.g. all of
these nodes lexically start from the same token), but this is not a strict
rule an API user could relay on. In general, an API user should assume that
the nodes of a cluster are logically "close" to each other, and during the
incremental reparsing nodes of a single cluster could obsolete altogether.

A Cluster is a mutable structure. An API user could add, remove or modify nodes
and errors inside the cluster at any stage of the compilation system as long as
the user has mutable access to particular cluster.

This module provides a system of high level weak references to deal with the
syntax structure instances. This includes [NodeRef](crate::syntax::NodeRef),
[ClusterRef](crate::syntax::ClusterRef) and [ErrorRef](crate::syntax::ErrorRef).
See [Arena](crate::arena) module documentation to read more about the weak
reference framework.
