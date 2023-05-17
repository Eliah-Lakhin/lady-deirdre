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
    arena::{Identifiable, Ref},
    lexis::{SiteRefSpan, ToSpan},
    std::*,
    syntax::{Cluster, ClusterRef, Node, NodeRef},
};

/// A low-level interface to access and inspect syntax structure of the compilation unit.
///
/// SyntaxTree by convenient should be implemented for the compilation unit management object such
/// as [Document](crate::Document) and [SyntaxBuffer](crate::syntax::SyntaxBuffer) objects that
/// supposed to manage code's syntax grammar structure.
///
/// This trait:
///   1. Specifies syntax grammar through the [Node](crate::syntax::SyntaxTree::Node) associative
///      type.
///   2. Provides a [root](crate::syntax::SyntaxTree::root) function to obtain a weak reference to
///      the root node of the syntax tree. An API uses utilizes this function to enter into the
///      the syntax tree structure, and uses received reference to further inspect and traverse this
///      syntax structure.
///   3. Provides an [errors](crate::syntax::SyntaxTree::errors) function to obtain an
///      [iterator](crate::syntax::SyntaxTree::ErrorIterator) over all syntax and semantic errors
///      associated with this compilation unit.
///   4. Provides low-level interface to resolve higher-level weak references(such as
///      [ClusterRef](crate::syntax::ClusterRef), [NodeRef](crate::syntax::NodeRef), or
///      [ErrorRef](crate::syntax::ErrorRef)).
///
/// In practice an API user interacts with a small subset of this functionality directly.
///
/// To implement an extension library to this Crate with the source code management of alternative
/// designs, you can implement this trait over these objects. In this case these new objects will be
/// able to interact with existing [Node](crate::syntax::Node) implementations, and the weak
/// references belong to them will work transparently with other conventional weak references.
pub trait SyntaxTree: Identifiable {
    /// Specifies programming language lexical grammar.
    ///
    /// See [Node](crate::syntax::Node) for details.
    type Node: Node;

    /// Specifies a finite iterator over the source code syntax and semantic errors belong
    /// to this unit of compilation.
    type ErrorIterator<'tree>: Identifiable
        + Iterator<Item = &'tree <Self::Node as Node>::Error>
        + FusedIterator
    where
        Self: 'tree;

    type ClusterIterator<'tree>: Identifiable
        + Iterator<Item = &'tree Cluster<Self::Node>>
        + FusedIterator
    where
        Self: 'tree;

    type ClusterIteratorMut<'tree>: Identifiable
        + Iterator<Item = &'tree mut Cluster<Self::Node>>
        + FusedIterator
    where
        Self: 'tree;

    /// Returns a [`weak reference`](crate::syntax::NodeRef) to the root Node of the syntax tree.
    fn root(&self) -> &NodeRef;

    fn cover(&self, span: impl ToSpan) -> Ref;

    /// Returns iterator over all syntax and semantic errors belong to this unit of compilation.
    fn errors(&self) -> Self::ErrorIterator<'_>;

    fn traverse(&self) -> Self::ClusterIterator<'_>;

    fn traverse_mut(&mut self) -> Self::ClusterIteratorMut<'_>;

    /// Returns `true` if the [`Node Cluster`](crate::syntax::ClusterRef) referred by specified
    /// low-level `cluster_ref` weak reference exists in this syntax tree instance.
    ///
    /// This is a low-level API used by the higher-level [ClusterRef](crate::syntax::ClusterRef),
    /// [NodeRef](crate::syntax::NodeRef) and [ErrorRef](crate::syntax::ErrorRef) weak references
    /// under the hood. An API user normally don't need to call this function directly.
    fn contains_cluster(&self, cluster_ref: &Ref) -> bool;

    /// Immutably dereferences a [Cluster](crate::syntax::Cluster) instance by specified low-level
    /// `cluster_ref` weak reference.
    ///
    /// Returns [None] if referred Cluster does not exist in this instance.
    ///
    /// This is a low-level API used by the higher-level [ClusterRef](crate::syntax::ClusterRef),
    /// [NodeRef](crate::syntax::NodeRef) and [ErrorRef](crate::syntax::ErrorRef) weak references
    /// under the hood. An API user normally don't need to call this function directly.
    fn get_cluster(&self, cluster_ref: &Ref) -> Option<&Cluster<Self::Node>>;

    /// Mutably dereferences a [Cluster](crate::syntax::Cluster) instance by specified low-level
    /// `cluster_ref` weak reference.
    ///
    /// Returns [None] if referred Cluster does not exist in this instance.
    ///
    /// This is a low-level API used by the higher-level [ClusterRef](crate::syntax::ClusterRef),
    /// [NodeRef](crate::syntax::NodeRef) and [ErrorRef](crate::syntax::ErrorRef) weak references
    /// under the hood. An API user normally don't need to call this function directly.
    fn get_cluster_mut(&mut self, cluster_ref: &Ref) -> Option<&mut Cluster<Self::Node>>;

    fn get_cluster_span(&self, cluster_ref: &Ref) -> SiteRefSpan;

    fn get_previous_cluster(&self, cluster_ref: &Ref) -> Ref;

    fn get_next_cluster(&self, cluster_ref: &Ref) -> Ref;

    fn remove_cluster(&mut self, cluster_ref: &Ref) -> Option<Cluster<Self::Node>>;
}
