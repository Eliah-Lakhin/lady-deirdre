////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use std::fmt::{Debug, Display, Formatter};

use crate::{
    arena::{Entry, Identifiable},
    lexis::{
        Chunk,
        Length,
        LineIndex,
        Site,
        SiteSpan,
        SourceCode,
        ToSpan,
        Token,
        TokenBuffer,
        TokenCount,
        TokenCursor,
    },
    syntax::{Capture, Node, NodeRef, PolyRef, PolyVariant, SyntaxError, SyntaxTree},
    units::{Document, ImmutableUnit, MutableUnit},
};

/// An object that grants access to the lexical and syntax structure of
/// an individual file within the compilation project.
///
/// [Document], [ImmutableUnit] and [MutableUnit] are compilation units
/// because they offer access to both components of the language grammar,
/// but, for instance, [TokenBuffer] is not because it only provides an access
/// to the lexical structure only.
///
/// CompilationUnit trait provides conventional functions to convert this unit
/// into other types of units and some syntax analysis functions
/// that require access to the full grammar structure of the file.
///
/// If you intend to implement this trait on your object, take a look at the
/// [Lexis] and the [Syntax] facade-interfaces; they will assist you in exposing
/// particular components of the grammar.
pub trait CompilationUnit:
    SourceCode<Token = <<Self as SyntaxTree>::Node as Node>::Token> + SyntaxTree
{
    /// Returns `true` if the compilation unit allows document write operations
    /// after creation.
    fn is_mutable(&self) -> bool;

    /// Returns `true` if the compilation unit does not have write capabilities
    /// after creation.
    #[inline(always)]
    fn is_immutable(&self) -> bool {
        !self.is_mutable()
    }

    /// Extracts lexical structure of the compilation unit.
    fn into_token_buffer(self) -> TokenBuffer<<Self as SourceCode>::Token>;

    /// Converts this compilation unit into [Document].
    ///
    /// The mutable capabilities of the returning document depend on the
    /// [CompilationUnit::is_mutable] value.
    ///
    /// Depending on the implementation this function may require full
    /// source code reparsing, but implementors typically make the best effort
    /// to reduce overhead. In particular, [Document]'s into_document is noop.
    #[inline(always)]
    fn into_document(self) -> Document<<Self as SyntaxTree>::Node>
    where
        Self: Sized,
    {
        match self.is_mutable() {
            true => Document::Mutable(self.into_mutable_unit()),
            false => Document::Immutable(self.into_immutable_unit()),
        }
    }

    /// Converts this compilation unit [MutableUnit].
    ///
    /// Depending on the implementation this function may require full
    /// source code reparsing, but implementors typically make the best effort
    /// to reduce overhead. In particular, [MutableUnit]'s into_mutable_unit
    /// is noop.
    #[inline(always)]
    fn into_mutable_unit(self) -> MutableUnit<<Self as SyntaxTree>::Node>
    where
        Self: Sized,
    {
        self.into_token_buffer().into_mutable_unit()
    }

    /// Converts this compilation unit [ImmutableUnit].
    ///
    /// Depending on the implementation this function may require full
    /// source code reparsing, but implementors typically make the best effort
    /// to reduce overhead. In particular, [ImmutableUnit]'s into_immutable_unit
    /// is noop.
    #[inline(always)]
    fn into_immutable_unit(self) -> ImmutableUnit<<Self as SyntaxTree>::Node>
    where
        Self: Sized,
    {
        self.into_token_buffer().into_immutable_unit()
    }

    /// Searches for the top-most node in the syntax tree that fully covers
    /// specified source code [span](ToSpan).
    ///
    /// For example, in the case of JSON `{"foo": [123]}`, the coverage of the
    /// "123" token could be the `[123]` array, and the coverage of the ":"
    /// token could be the `"foo": [bar]` entry of the JSON object.
    ///
    /// The result depends on the particular programming language grammar.
    ///
    /// In the worst case scenario, if the algorithm fails to find the top-most
    /// node, it returns the reference to the root node.
    ///
    /// **Panic**
    ///
    /// Panics if the specified span is not valid for this compilation unit.
    #[inline(always)]
    fn cover(&self, span: impl ToSpan) -> NodeRef
    where
        Self: Sized,
    {
        let span = match span.to_site_span(self) {
            None => panic!("Specified span is invalid."),

            Some(span) => span,
        };

        let root = self.root_node_ref();

        match NodeCoverage::cover(self, &root, &span) {
            NodeCoverage::Fit(result) => result,
            _ => root,
        }
    }

    /// Returns an object that prints the underlying grammar structure.
    ///
    /// The `poly_ref` parameter specifies a reference to a particular grammar
    /// component to debug. It could be a [NodeRef],
    /// [TokenRef](crate::lexis::TokenRef) or [PolyVariant].
    ///
    /// To print the entire syntax tree with all nodes and tokens metadata, you
    /// can obtain a root NodeRef using [SyntaxTree::root_node_ref] function.
    ///
    /// The default implementation is infallible regardless of the `poly_ref`
    /// validity.
    #[inline(always)]
    fn display(&self, poly_ref: &(impl PolyRef + ?Sized)) -> impl Debug + Display + '_
    where
        Self: Sized,
    {
        DisplayTree {
            unit: self,
            variant: poly_ref.as_variant(),
        }
    }
}

/// A facade of the lexical structure.
///
/// This trait auto-implements [SourceCode] on the target object by delegating
/// all calls to the required function [Lexis::lexis].
///
/// ```rust
/// use lady_deirdre::units::Lexis;
/// use lady_deirdre::arena::{Id, Identifiable};
/// use lady_deirdre::lexis::{Token, TokenBuffer};
///
/// struct MyDocument<T: Token> {
///     buf: TokenBuffer<T>,
/// }
///
/// impl<T: Token> Identifiable for MyDocument<T> {
///     fn id(&self) -> Id {
///         self.buf.id()
///     }
/// }
///
/// impl<T: Token> Lexis for MyDocument<T> {
///     type Lexis = TokenBuffer<T>;
///
///     fn lexis(&self) -> &Self::Lexis {
///         &self.buf
///     }
/// }
/// ```
pub trait Lexis: Identifiable {
    /// The target [SourceCode] delegation type.
    type Lexis: SourceCode;

    /// This function fully exposes underlying [SourceCode] interface
    /// of this object.
    fn lexis(&self) -> &Self::Lexis;
}

impl<F: Lexis> SourceCode for F {
    type Token = <F::Lexis as SourceCode>::Token;

    type Cursor<'code> = <F::Lexis as SourceCode>::Cursor<'code>
        where Self: 'code;

    type CharIterator<'code> = <F::Lexis as SourceCode>::CharIterator<'code>
    where
        Self: 'code;

    #[inline(always)]
    fn chars(&self, span: impl ToSpan) -> Self::CharIterator<'_> {
        self.lexis().chars(span)
    }

    #[inline(always)]
    fn has_chunk(&self, entry: &Entry) -> bool {
        self.lexis().has_chunk(entry)
    }

    #[inline(always)]
    fn get_token(&self, entry: &Entry) -> Option<Self::Token> {
        self.lexis().get_token(entry)
    }

    #[inline(always)]
    fn get_site(&self, entry: &Entry) -> Option<Site> {
        self.lexis().get_site(entry)
    }

    #[inline(always)]
    fn get_string(&self, entry: &Entry) -> Option<&str> {
        self.lexis().get_string(entry)
    }

    #[inline(always)]
    fn get_length(&self, entry: &Entry) -> Option<Length> {
        self.lexis().get_length(entry)
    }

    #[inline(always)]
    fn cursor(&self, span: impl ToSpan) -> Self::Cursor<'_> {
        self.lexis().cursor(span)
    }

    #[inline(always)]
    fn length(&self) -> Length {
        self.lexis().length()
    }

    #[inline(always)]
    fn tokens(&self) -> TokenCount {
        self.lexis().tokens()
    }

    #[inline(always)]
    fn lines(&self) -> &LineIndex {
        self.lexis().lines()
    }
}

/// A facade of the syntax structure.
///
/// This trait auto-implements [SyntaxTree] on the target object by delegating
/// all calls to the required functions [Syntax::syntax]
/// and [Syntax::syntax_mut].
///
/// ```rust
/// use lady_deirdre::units::Syntax;
/// use lady_deirdre::arena::{Id, Identifiable};
/// use lady_deirdre::syntax::{Node, ImmutableSyntaxTree};
///
/// struct MyDocument<N: Node> {
///     tree: ImmutableSyntaxTree<N>,
/// }
///
/// impl<N: Node> Identifiable for MyDocument<N> {
///     fn id(&self) -> Id {
///         self.tree.id()
///     }
/// }
///
/// impl<N: Node> Syntax for MyDocument<N> {
///     type Syntax = ImmutableSyntaxTree<N>;
///
///     fn syntax(&self) -> &Self::Syntax {
///         &self.tree
///     }
///
///     fn syntax_mut(&mut self) -> &mut Self::Syntax {
///         &mut self.tree
///     }
/// }
/// ```
pub trait Syntax: Identifiable {
    /// The target [SyntaxTree] delegation type.
    type Syntax: SyntaxTree;

    /// This function fully exposes immutable access to the underlying
    /// [SyntaxTree] interface of this object.
    fn syntax(&self) -> &Self::Syntax;

    /// This function fully exposes mutable access to the underlying
    /// [SyntaxTree] interface of this object.
    fn syntax_mut(&mut self) -> &mut Self::Syntax;
}

impl<F: Syntax> SyntaxTree for F {
    type Node = <F::Syntax as SyntaxTree>::Node;

    type NodeIterator<'tree> = <F::Syntax as SyntaxTree>::NodeIterator<'tree> where Self: 'tree;

    type ErrorIterator<'tree> = <F::Syntax as SyntaxTree>::ErrorIterator<'tree> where Self: 'tree;

    #[inline(always)]
    fn root_node_ref(&self) -> NodeRef {
        self.syntax().root_node_ref()
    }

    #[inline(always)]
    fn node_refs(&self) -> Self::NodeIterator<'_> {
        self.syntax().node_refs()
    }

    #[inline(always)]
    fn error_refs(&self) -> Self::ErrorIterator<'_> {
        self.syntax().error_refs()
    }

    #[inline(always)]
    fn has_node(&self, entry: &Entry) -> bool {
        self.syntax().has_node(entry)
    }

    #[inline(always)]
    fn get_node(&self, entry: &Entry) -> Option<&Self::Node> {
        self.syntax().get_node(entry)
    }

    #[inline(always)]
    fn get_node_mut(&mut self, entry: &Entry) -> Option<&mut Self::Node> {
        self.syntax_mut().get_node_mut(entry)
    }

    #[inline(always)]
    fn has_error(&self, entry: &Entry) -> bool {
        self.syntax().has_error(entry)
    }

    #[inline(always)]
    fn get_error(&self, entry: &Entry) -> Option<&SyntaxError> {
        self.syntax().get_error(entry)
    }
}

struct DisplayTree<
    'unit,
    N: Node,
    C: TokenCursor<'unit>,
    U: CompilationUnit<Cursor<'unit> = C, Node = N>,
> {
    unit: &'unit U,
    variant: PolyVariant,
}

impl<'unit, N, C, U> Debug for DisplayTree<'unit, N, C, U>
where
    N: Node,
    C: TokenCursor<'unit>,
    U: CompilationUnit<Cursor<'unit> = C, Node = N>,
{
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        Display::fmt(self, formatter)
    }
}

impl<'unit, N, C, U> Display for DisplayTree<'unit, N, C, U>
where
    N: Node,
    C: TokenCursor<'unit>,
    U: CompilationUnit<Cursor<'unit> = C, Node = N>,
{
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        match &self.variant {
            PolyVariant::Token(variant) => {
                let chunk: Chunk<U::Token> = match variant.chunk(self.unit) {
                    None => return Debug::fmt(variant, formatter),
                    Some(chunk) => chunk,
                };

                let name = chunk.token.name().unwrap_or("TokenRef");

                let mut debug_struct =
                    formatter.debug_struct(&format!("${name}(chunk_entry: {:?})", variant.entry));

                debug_struct.field("string", &chunk.string);
                debug_struct.field("length", &chunk.length);

                if let Some(site_span) = chunk.to_site_span(self.unit) {
                    debug_struct.field("site_span", &site_span);

                    debug_struct.field(
                        "position_span",
                        &format_args!("{}", chunk.display(self.unit)),
                    );
                }

                debug_struct.finish()
            }

            PolyVariant::Node(variant) => {
                let node: &N = match variant.deref(self.unit) {
                    None => return Debug::fmt(variant, formatter),
                    Some(node) => node,
                };

                let name = node.name().unwrap_or("NodeRef");

                let alternate = formatter.alternate();

                let mut debug_struct =
                    formatter.debug_struct(&format!("{name}(entry: {:?})", variant.entry));

                for key in node.capture_keys() {
                    let Some(capture) = node.capture(*key) else {
                        continue;
                    };

                    let key = key.to_string();

                    match capture {
                        Capture::SingleNode(capture) => match alternate {
                            true => debug_struct
                                .field(&key, &format_args!("{:#}", self.unit.display(capture))),
                            false => debug_struct
                                .field(&key, &format_args!("{}", self.unit.display(capture))),
                        },

                        Capture::ManyNodes(capture) => {
                            let poly_refs = capture
                                .into_iter()
                                .map(|poly_ref| self.unit.display(poly_ref))
                                .collect::<Vec<_>>();

                            debug_struct.field(&key, &poly_refs)
                        }

                        Capture::SingleToken(capture) => match alternate {
                            true => debug_struct
                                .field(&key, &format_args!("{:#}", self.unit.display(capture))),
                            false => debug_struct
                                .field(&key, &format_args!("{}", self.unit.display(capture))),
                        },

                        Capture::ManyTokens(capture) => {
                            let poly_refs = capture
                                .into_iter()
                                .map(|poly_ref| self.unit.display(poly_ref))
                                .collect::<Vec<_>>();

                            debug_struct.field(&key, &poly_refs)
                        }
                    };
                }

                debug_struct.finish()
            }
        }
    }
}

#[derive(Debug)]
pub(super) enum NodeCoverage {
    Nil,
    Fit(NodeRef),
    Misfit(Site),
}

impl NodeCoverage {
    pub(super) fn cover<
        'unit,
        N: Node,
        C: TokenCursor<'unit>,
        U: CompilationUnit<Cursor<'unit> = C, Node = N>,
    >(
        unit: &'unit U,
        node_ref: &NodeRef,
        span: &SiteSpan,
    ) -> Self {
        let node: &N = match node_ref.deref(unit) {
            None => return Self::Nil,
            Some(node) => node,
        };

        let node_span = match node.span(unit) {
            None => return Self::Nil,
            Some(span) => span,
        };

        if node_span.start > span.start || node_span.end < span.end {
            return Self::Misfit(node_span.start);
        }

        for child in node.children_iter() {
            if !child.kind().is_node() {
                continue;
            }

            match Self::cover(unit, child.as_node_ref(), span) {
                Self::Nil => continue,
                Self::Misfit(start) => {
                    if start > span.start {
                        break;
                    }
                }
                other => return other,
            }
        }

        Self::Fit(*node_ref)
    }
}
