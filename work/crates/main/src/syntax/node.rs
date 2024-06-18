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

extern crate lady_deirdre_derive;

use std::fmt::{Debug, Formatter};

pub use lady_deirdre_derive::Node;

use crate::{
    arena::{Entry, Id, Identifiable, SubId},
    lexis::{Site, SiteSpan, SourceCode, Token, TokenBuffer, TokenRef, NIL_TOKEN_REF},
    syntax::{
        Capture,
        CapturesIter,
        ChildrenIter,
        DebugObserver,
        ImmutableSyntaxTree,
        Key,
        NodeRule,
        PolyRef,
        PolyVariant,
        RefKind,
        SyntaxSession,
        SyntaxTree,
        NON_RULE,
    },
    units::CompilationUnit,
};

/// A [NodeRef] reference that does not point to any node.
///
/// The value of this static equals to the [NodeRef::nil] value.
pub static NIL_NODE_REF: NodeRef = NodeRef::nil();

/// A type of the syntax tree node.
///
/// Typically, this trait should be implemented on enum types, where each enum
/// variant represents an individual node kind. The variant fields would
/// include references to the parent and children nodes.
///
/// The interface provides language-agnostic functions to reveal
/// node's structure, such as [children_iter](AbstractNode::children_iter)
/// to iterate through all children of the node instance, or
/// [name](AbstractNode::name) to get the node's variant name.
///
/// The [Node::parse] function serves as the syntax parser of the programming
/// language and the constructor of the node instance.
///
/// Essentially, this interface defines the syntax component of the programming
/// language grammar.
///
/// The node interface is split into the [Node] trait, which includes
/// object-unsafe API, and its super-trait [AbstractNode] which includes
/// object-safe API.
///
/// You are encouraged to use the companion [Node](lady_deirdre_derive::Node)
/// derive macro to implement all required components on enum types in terms
/// of the LL(1) grammar.
pub trait Node: AbstractNode + Sized {
    /// Specifies the lexical structure of the language.
    ///
    /// This associated type is required because the syntax grammar of the
    /// language includes the lexical grammar as well.
    ///
    /// When using the derive macro, this type is specified through the
    /// `#[token(...)]` attribute:
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// #[token(MyToken)]
    /// struct MyNode {
    ///     //...
    /// }
    /// ```
    type Token: Token;

    /// Parses the programming language syntax tree node.
    ///
    /// The `session` parameter of type [SyntaxSession] provides access
    /// to the token stream that needs to be parsed and offers an API to descend
    /// into the sub-rules as needed.
    ///
    /// The `rule` is a numeric index of the syntax parse rule that needs to be
    /// parsed.
    ///
    /// The exact set of valid values for the `rule` argument, and the mapping
    /// between these values and the Node types, is language-specific.
    ///
    /// The calling side doesn't need to know this mapping upfront,
    /// except for the following `rule` cases:
    ///
    ///   - [ROOT_RULE](crate::syntax::ROOT_RULE) parses the root rule of the
    ///     syntax tree. The parse function should always be able to parse
    ///     at least this rule.
    ///   - [NON_RULE] does not represent any parsing rule within any
    ///     programming language. This function should never be called with this
    ///     rule value.
    ///
    /// If the `rule` value is within the valid set of the programming language
    /// rule set, **the function is infallible** regardless of the input token
    /// stream.
    ///
    /// In the event of syntax errors in the input stream, the function attempts
    /// to recover from these errors, and it **always consumes at least one token**
    /// from the input token stream (if the stream is not empty).
    ///
    /// Finally, the underlying parsing algorithm is deterministic
    /// and context-free: the parse function always returns the same result
    /// from the same set of input tokens and requested `rule`, and the
    /// function always returns the same kind of syntax tree nodes for
    /// the same `rule` value.
    ///
    /// Typically, you don't need to call this function manually. It is the
    /// responsibility of the compilation unit manager
    /// (e.g., [Document](crate::units::Document)) to decide when to call this
    /// function.
    ///
    /// To debug the parser, use the [Node::debug] function.
    ///
    /// For a detailed specification of the syntax parsing process,
    /// refer to the [SyntaxSession] documentation.
    ///
    /// **Safety**
    ///
    /// This function **is safe**. Violations of any of the above rules is an
    /// implementation bug, not undefined behavior.
    ///
    /// **Panic**
    ///
    /// The function may panic if the `rule` parameter value is not valid for
    /// this programming language.
    fn parse<'code>(session: &mut impl SyntaxSession<'code, Node = Self>, rule: NodeRule) -> Self;

    /// Debugs the syntax parsing algorithm for this node type.
    ///
    /// This function runs the parsing algorithm on the `text` source code
    /// and prints parsing steps to the terminal (stdout).
    fn debug(text: impl AsRef<str>) {
        let tokens = TokenBuffer::<Self::Token>::from(text);

        ImmutableSyntaxTree::<Self>::parse_with_id_and_observer(
            SubId::fork(tokens.id()),
            tokens.cursor(..),
            &mut DebugObserver::default(),
        );
    }
}

/// An object-safe part of the syntax tree node interface.
///
/// This trait is a super-trait of the [Node] trait, which is not object-safe.
///
/// The entire interface is separated into two traits so that an API user
/// can use most parts of the whole interface from the object-safe trait.
///
/// The AbstractNode trait consists of language-agnostic functions to
/// read individual syntax tree node structure, whereas the Node trait provides
/// node's parser, essentially the node constructor.
pub trait AbstractNode: Send + Sync + 'static {
    /// A syntax parse rule that parses this kind of node.
    ///
    /// When using the [Node](lady_deirdre_derive::Node) macro, this value
    /// is either generated by the macro program or overridden through the
    /// `#[denote(...)]` attribute:
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     #[denote(100)] // self.rule() == 100
    ///     #[rule()]
    ///     Variant1 {},
    ///
    ///     #[denote(V2)] // self.rule() == Self::V2
    ///     #[rule()]
    ///     Variant2 {},
    ///
    ///     #[denote(V3, 300)] // self.rule() == Self::V3 && Self::V3 == 300
    ///     #[rule()]
    ///     Variant3 {},
    ///
    ///     #[rule()] // self.rule() value generated by the macro
    ///     Variant4 {},
    /// }
    /// ```
    fn rule(&self) -> NodeRule;

    /// A debug name of this node.
    ///
    /// Returns None if this feature is disabled for this node instance.
    ///
    /// When using the [Node](lady_deirdre_derive::Node) macro, this function
    /// returns the stringified variant's name:
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     #[rule()]
    ///     Variant {}, // self.name() == Some("Variant")
    ///
    ///     NonParsable {}, // self.name() == None
    /// }
    /// ```
    fn name(&self) -> Option<&'static str>;

    /// An end-user display description of this node.
    ///
    /// Returns None if this feature is disabled for this node instance.
    ///
    /// This function is intended to be used for the syntax errors formatting.
    ///
    /// When using the [Node](lady_deirdre_derive::Node) macro, this function
    /// returns what you have specified with the `#[describe(...)]` attribute:
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     // self.describe(false) == Some("short")
    ///     // self.describe(true) == Some("verbose")
    ///     #[rule()]
    ///     #[describe("short", "verbose")]
    ///     Variant {},
    ///
    ///     NonParsable {}, // self.name() == None
    /// }
    /// ```
    ///
    /// The difference between the short (`verbose` is false) and verbose
    /// (verbose is `true`) descriptions is that the short version represents
    /// a "class" of the node, while the verbose version provides a more
    /// detailed text specific to this particular node.
    ///
    /// For example, a short description of the Sum and Mul binary operators
    /// would simply be "operator", whereas, for verbose versions
    /// this function might returns something like "<a + b>" and "<a * b>".
    fn describe(&self, verbose: bool) -> Option<&'static str>;

    /// Returns a [NodeRef] reference of this node.
    ///
    /// The returning value resolves to self when borrowing a node from
    /// the [SyntaxTree].
    ///
    /// This function may return [nil](NodeRef::nil) reference, if the feature
    /// is disabled for this node instance.
    ///
    /// When using the [Node](lady_deirdre_derive::Node) macro, this function
    /// returns what you have annotated with the `#[node(...)]` attribute:
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     // self.node_ref() returns `node` value
    ///     #[rule()]
    ///     Variant {
    ///         #[node]
    ///         node: NodeRef,
    ///     },
    ///
    ///     // self.node_ref() returns NodeRef::nil()
    ///     #[rule()]
    ///     VariantWithoutNodeRef {
    ///         // #[node]
    ///         node: NodeRef,
    ///     },
    /// }
    /// ```
    fn node_ref(&self) -> NodeRef;

    /// Returns a [NodeRef] reference of the parent node of this node.
    ///
    /// The returning value resolves to the parent node when borrowing a node
    /// from the [SyntaxTree].
    ///
    /// This function may return [nil](NodeRef::nil) reference, if the feature
    /// is disabled for this node instance.
    ///
    /// When using the [Node](lady_deirdre_derive::Node) macro, this function
    /// returns what you have annotated with the `#[parent(...)]` attribute:
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     // self.parent_ref() returns `parent` value
    ///     #[rule()]
    ///     Variant {
    ///         #[parent]
    ///         parent: NodeRef,
    ///     },
    ///
    ///     // self.parent_ref() returns NodeRef::nil()
    ///     #[rule()]
    ///     VariantWithoutParentRef {
    ///         // #[parent]
    ///         parent: NodeRef,
    ///     },
    /// }
    /// ```
    fn parent_ref(&self) -> NodeRef;

    /// Updates the parent node reference of this node.
    ///
    /// This function updates the value returned by the
    /// [parent_ref](Self::parent_ref) function.
    ///
    /// The compilation unit managers
    /// (e.g., mutable [Document](crate::units::Document)) may use this function
    /// to "transplant" the syntax tree branch to another branch.
    ///
    /// This function could ignore the provided [NodeRef] reference
    /// if the "parent_ref" feature is not available for this node instance.
    fn set_parent_ref(&mut self, parent_ref: NodeRef);

    /// Returns a set of children of this node associated with the specified
    /// `key`.
    ///
    /// When using the [Node](lady_deirdre_derive::Node) macro, this function
    /// returns what you have annotated with the `#[child]` attribute.
    ///
    /// The string `key` denotes the field name, and the numeric `key`
    /// denotes the index of the `#[child]` attribute in order.
    ///
    /// The function returns None if there is no capture associated
    /// with specified key.
    ///
    /// ```ignore
    /// #[derive(Node)]
    /// enum MyNode {
    ///     #[rule()]
    ///     Variant {
    ///         #[child] // self.capture(Key::Index(0))
    ///         capture_1: NodeRef,
    ///
    ///         #[child] // self.capture(Key::Name("capture_2"))
    ///         capture_2: Vec<NodeRef>,
    ///
    ///         #[child] // self.capture(Key::Index(2))
    ///         capture_3: TokenRef,
    ///     },
    /// }
    /// ```
    fn capture(&self, key: Key) -> Option<Capture>;

    /// Returns the first set of children of this node.
    ///
    /// Returns None if there are no known captures in this node instance.
    #[inline(always)]
    fn first_capture(&self) -> Option<Capture> {
        self.capture(Key::Index(0))
    }

    /// Returns the last set of children of this node.
    ///
    /// Returns None if there are no known captures in this node instance.
    #[inline(always)]
    fn last_capture(&self) -> Option<Capture> {
        self.capture(Key::Index(self.captures_len().checked_sub(1)?))
    }

    /// Returns all valid [capture](Self::capture) keys.
    ///
    /// The keys in the returning array come in order such as the index of the
    /// [Key] in this array corresponds to the [Key::Index] with this index.
    ///
    /// However the function prefers to return an array of [Key::Name] so that
    /// the calling side gains both the capture number and the capture name
    /// metadata.
    fn capture_keys(&self) -> &'static [Key<'static>];

    /// Returns a total number of [captures](Self::capture) of this node instance.
    #[inline(always)]
    fn captures_len(&self) -> usize {
        self.capture_keys().len()
    }

    /// Returns an iterator over all capture values.
    #[inline(always)]
    fn captures_iter(&self) -> CapturesIter<Self>
    where
        Self: Sized,
    {
        CapturesIter::new(self)
    }

    /// Returns an iterator over all children of this node.
    ///
    /// This is a version of the [captures_iter](Self::captures_iter) that
    /// subsequently iterates each child inside the [Capture] and flattens
    /// the result.
    #[inline(always)]
    fn children_iter(&self) -> ChildrenIter<Self>
    where
        Self: Sized,
    {
        ChildrenIter::new(self)
    }

    /// Returns a [NodeRef] reference of a child node that precedes
    /// the `current` child node.
    ///
    /// Returns None if the `current` node is the first child, or if
    /// the `current` is not a reference to a child node.
    fn prev_child_node(&self, current: &NodeRef) -> Option<&NodeRef>
    where
        Self: Sized,
    {
        let mut nodes = self
            .children_iter()
            .rev()
            .filter(|child| child.kind().is_node())
            .map(|child| child.as_node_ref());

        loop {
            let probe = nodes.next()?;

            if probe == current {
                return nodes.next();
            }
        }
    }

    /// Returns a [NodeRef] reference of a child node that follows after
    /// the `current` child node.
    ///
    /// Returns None if the `current` node is the last child, or if
    /// the `current` is not a reference to a child node.
    fn next_child_node(&self, current: &NodeRef) -> Option<&NodeRef>
    where
        Self: Sized,
    {
        let mut nodes = self
            .children_iter()
            .filter(|child| child.kind().is_node())
            .map(|child| child.as_node_ref());

        loop {
            let probe = nodes.next()?;

            if probe == current {
                return nodes.next();
            }
        }
    }

    /// Infers the [site span](SiteSpan) of this node.
    ///
    /// The underlying algorithm infers the span based on the leftmost captured
    /// token (or the leftmost token of the leftmost descendant node)
    /// start site, and the rightmost token end site correspondingly.
    ///
    /// If the underlying syntax captures the leftmost and the rightmost tokens
    /// of the corresponding parse rules, this span matches the parsed segment
    /// span.
    ///
    /// Returns None if the span cannot be inferred based on the node captures
    /// (e.g., if the syntax does not have [TokenRef] captures).
    ///
    /// The `unit` parameter is the compilation unit
    /// (e.g., [Document](crate::units::Document)) to which this Node instance
    /// belongs.
    fn span(&self, unit: &impl CompilationUnit) -> Option<SiteSpan>
    where
        Self: Sized,
    {
        let start = self.start(unit)?;
        let end = self.end(unit)?;

        Some(start..end)
    }

    /// Infers the start [site](Site) of this node.
    ///
    /// The underlying algorithm infers the site based on the leftmost captured
    /// token (or the leftmost token of the leftmost descendant node)
    /// start site.
    ///
    /// If the underlying syntax captures the leftmost tokens of
    /// the corresponding parse rules, this span matches the parsed segment
    /// start site.
    ///
    /// Returns None if the site cannot be inferred based on the node captures
    /// (e.g., if the syntax does not have [TokenRef] captures).
    ///
    /// The `unit` parameter is the compilation unit
    /// (e.g., [Document](crate::units::Document)) to which this Node instance
    /// belongs.
    fn start(&self, unit: &impl CompilationUnit) -> Option<Site>
    where
        Self: Sized,
    {
        for child in self.captures_iter() {
            match child.start(unit) {
                None => continue,
                Some(site) => return Some(site),
            }
        }

        None
    }

    /// Infers the end [site](Site) of this node.
    ///
    /// The underlying algorithm infers the site based on the rightmost captured
    /// token (or the rightmost token of the rightmost descendant node)
    /// end site.
    ///
    /// If the underlying syntax captures the rightmost tokens of
    /// the corresponding parse rules, this span matches the parsed segment
    /// end site.
    ///
    /// Returns None if the site cannot be inferred based on the node captures
    /// (e.g., if the syntax does not have [TokenRef] captures).
    ///
    /// The `unit` parameter is the compilation unit
    /// (e.g., [Document](crate::units::Document)) to which this Node instance
    /// belongs.
    fn end(&self, unit: &impl CompilationUnit) -> Option<Site>
    where
        Self: Sized,
    {
        for child in self.captures_iter().rev() {
            match child.end(unit) {
                None => continue,
                Some(site) => return Some(site),
            }
        }

        None
    }

    /// A debug name of the parse rule.
    ///
    /// The returning value is the same as `self.name(self.rule())`.
    ///
    /// See [name](Self::name) for details.
    fn rule_name(rule: NodeRule) -> Option<&'static str>
    where
        Self: Sized;

    /// An end-user display description of the parse rule.
    ///
    /// The returning value is the same as `self.describe(self.rule(), verbose)`.
    ///
    /// See [describe](Self::describe) for details.
    fn rule_description(rule: NodeRule, verbose: bool) -> Option<&'static str>
    where
        Self: Sized;
}

/// A globally unique reference of the [node](Node) in the syntax tree.
///
/// Each [syntax tree](crate::syntax::SyntaxTree) node could be uniquely
/// addressed within a pair of the [Id] and [Entry], where the identifier
/// uniquely addresses a specific compilation unit instance (syntax tree), and
/// the entry part addresses a node within this tree.
///
/// Essentially, NodeRef is a composite index.
///
/// Both components of this index form a unique pair
/// (within the current process), because each compilation unit has a unique
/// identifier, and the nodes within the syntax tree always receive unique
/// [Entry] indices within the syntax tree.
///
/// If the node instance has been removed from the syntax tree over time,
/// new nodes within this syntax tree will never occupy the same NodeRef object,
/// but the NodeRef referred to the removed Node would become _invalid_.
///
/// The [nil](NodeRef::nil) NodeRefs are special references that are considered
/// to be always invalid (they intentionally don't refer to any node within
/// any syntax tree).
///
/// Two distinct instances of the nil NodeRef are always equal.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeRef {
    /// An identifier of the syntax tree.
    pub id: Id,

    /// A versioned index of the node instance within the syntax tree.
    pub entry: Entry,
}

impl Debug for NodeRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        match self.is_nil() {
            false => formatter.write_fmt(format_args!(
                "NodeRef(id: {:?}, entry: {:?})",
                self.id, self.entry,
            )),
            true => formatter.write_str("NodeRef(Nil)"),
        }
    }
}

impl Identifiable for NodeRef {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl Default for NodeRef {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

impl PolyRef for NodeRef {
    #[inline(always)]
    fn kind(&self) -> RefKind {
        RefKind::Node
    }

    #[inline(always)]
    fn is_nil(&self) -> bool {
        self.id.is_nil() || self.entry.is_nil()
    }

    #[inline(always)]
    fn as_variant(&self) -> PolyVariant {
        PolyVariant::Node(*self)
    }

    #[inline(always)]
    fn as_token_ref(&self) -> &TokenRef {
        &NIL_TOKEN_REF
    }

    #[inline(always)]
    fn as_node_ref(&self) -> &NodeRef {
        self
    }

    #[inline(always)]
    fn span(&self, unit: &impl CompilationUnit) -> Option<SiteSpan> {
        self.deref(unit)?.span(unit)
    }
}

impl NodeRef {
    /// Returns a NodeRef that intentionally does not refer to any node within
    /// any syntax tree.
    ///
    /// If you need just a static reference to the nil NodeRef, use
    /// the predefined [NIL_NODE_REF] static.
    #[inline(always)]
    pub const fn nil() -> Self {
        Self {
            id: Id::nil(),
            entry: Entry::nil(),
        }
    }

    /// Immutably borrows a syntax tree node referred to by this NodeRef.
    ///
    /// Returns None if this NodeRef is not valid for the specified `tree`.
    #[inline(always)]
    pub fn deref<'tree, N: Node>(
        &self,
        tree: &'tree impl SyntaxTree<Node = N>,
    ) -> Option<&'tree N> {
        if self.id != tree.id() {
            return None;
        }

        tree.get_node(&self.entry)
    }

    /// Mutably borrows a syntax tree node referred to by this NodeRef.
    ///
    /// Returns None if this NodeRef is not valid for the specified `tree`.
    #[inline(always)]
    pub fn deref_mut<'tree, N: Node>(
        &self,
        tree: &'tree mut impl SyntaxTree<Node = N>,
    ) -> Option<&'tree mut N> {
        if self.id != tree.id() {
            return None;
        }

        tree.get_node_mut(&self.entry)
    }

    /// Returns a syntax parse rule that parses referred node.
    ///
    /// Returns [NON_RULE] if this NodeRef is not valid for the specified `tree`.
    ///
    /// See [AbstractNode::rule] for details.
    #[inline(always)]
    pub fn rule(&self, tree: &impl SyntaxTree) -> NodeRule {
        self.deref(tree).map(AbstractNode::rule).unwrap_or(NON_RULE)
    }

    /// Returns a debug name of the referred node.
    ///
    /// Returns None if this NodeRef is not valid for the specified `tree`,
    /// or if the node instance does not have a name.
    ///
    /// See [AbstractNode::name] for details.
    #[inline(always)]
    pub fn name<N: Node>(&self, tree: &impl SyntaxTree<Node = N>) -> Option<&'static str> {
        self.deref(tree).map(AbstractNode::name).flatten()
    }

    /// Returns an end-user display description of the referred node.
    ///
    /// Returns None if this NodeRef is not valid for the specified `tree`,
    /// or if the node instance does not have a description.
    ///
    /// See [AbstractNode::describe] for details.
    #[inline(always)]
    pub fn describe<N: Node>(
        &self,
        tree: &impl SyntaxTree<Node = N>,
        verbose: bool,
    ) -> Option<&'static str> {
        self.deref(tree)
            .map(|node| node.describe(verbose))
            .flatten()
    }

    /// Returns a reference of the parent node of the referred node.
    ///
    /// Returns [nil](NodeRef::nil) if this NodeRef is not valid for
    /// the specified `tree`, or if the node instance does not have a parent.
    ///
    /// See [AbstractNode::parent_ref] for details.
    #[inline(always)]
    pub fn parent(&self, tree: &impl SyntaxTree) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        node.parent_ref()
    }

    /// Returns a reference to the first child node of the referred node.
    ///
    /// Returns [nil](NodeRef::nil) if this NodeRef is not valid for
    /// the specified `tree`, or if the node instance does not have child nodes.
    pub fn first_child(&self, tree: &impl SyntaxTree) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        node.children_iter()
            .filter(|child| child.kind().is_node())
            .map(|child| child.as_node_ref())
            .next()
            .copied()
            .unwrap_or_default()
    }

    /// Returns a reference to the last child node of the referred node.
    ///
    /// Returns [nil](NodeRef::nil) if this NodeRef is not valid for
    /// the specified `tree`, or if the node instance does not have child nodes.
    pub fn last_child(&self, tree: &impl SyntaxTree) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        node.children_iter()
            .rev()
            .filter(|child| child.kind().is_node())
            .map(|child| child.as_node_ref())
            .next()
            .copied()
            .unwrap_or_default()
    }

    /// Returns a child node by the capture `key`.
    ///
    /// Returns [nil](NodeRef::nil) if this NodeRef is not valid for
    /// the specified `tree`, or if the specified `key` parameter does not
    /// address a NodeRef capture.
    ///
    /// If the capture referred to by the `key` parameter addresses multiple
    /// nodes, the function returns the first one.
    ///
    /// See [AbstractNode::capture] for details.
    pub fn get_child<'a>(&self, tree: &impl SyntaxTree, key: impl Into<Key<'a>>) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        let Some(child) = node.capture(key.into()) else {
            return NodeRef::nil();
        };

        let Some(first) = child.first() else {
            return NodeRef::nil();
        };

        *first.as_node_ref()
    }

    /// Returns a child token by the capture `key`.
    ///
    /// Returns [nil](TokenRef::nil) if this NodeRef is not valid for
    /// the specified `tree`, or if the specified `key` parameter does not
    /// address a [TokenRef] capture.
    ///
    /// If the capture referred to by the `key` parameter addresses multiple
    /// tokens, the function returns the first one.
    ///
    /// See [AbstractNode::capture] for details.
    pub fn get_token(&self, tree: &impl SyntaxTree, key: &'static str) -> TokenRef {
        let Some(node) = self.deref(tree) else {
            return TokenRef::nil();
        };

        let Some(child) = node.capture(key.into()) else {
            return TokenRef::nil();
        };

        let Some(first) = child.first() else {
            return TokenRef::nil();
        };

        *first.as_token_ref()
    }

    /// Returns a previous sibling node of the node referred to by this NodeRef
    /// within the node's parent.
    ///
    /// Returns [nil](NodeRef::nil) if this NodeRef is not valid for
    /// the specified `tree`, or if the referred node does not have a preceded
    /// sibling.
    pub fn prev_sibling(&self, tree: &impl SyntaxTree) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        let Some(parent) = node.parent_ref().deref(tree) else {
            return NodeRef::nil();
        };

        let Some(sibling) = parent.prev_child_node(self) else {
            return NodeRef::nil();
        };

        *sibling
    }

    /// Returns a next sibling node of the node referred to by this NodeRef
    /// within the node's parent.
    ///
    /// Returns [nil](NodeRef::nil) if this NodeRef is not valid for
    /// the specified `tree`, or if the referred node does not have a successive
    /// sibling.
    pub fn next_sibling(&self, tree: &impl SyntaxTree) -> NodeRef {
        let Some(node) = self.deref(tree) else {
            return NodeRef::nil();
        };

        let Some(parent) = node.parent_ref().deref(tree) else {
            return NodeRef::nil();
        };

        let Some(sibling) = parent.next_child_node(self) else {
            return NodeRef::nil();
        };

        *sibling
    }

    /// Returns true if the node referred to by this NodeRef exists in the specified
    /// `tree`.
    #[inline(always)]
    pub fn is_valid_ref(&self, tree: &impl SyntaxTree) -> bool {
        if self.id != tree.id() {
            return false;
        }

        tree.has_node(&self.entry)
    }
}
