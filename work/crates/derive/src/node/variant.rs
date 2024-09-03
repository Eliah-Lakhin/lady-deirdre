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

use std::mem::take;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{spanned::Spanned, AttrStyle, Error, Expr, Fields, Meta, Result, Variant};

use crate::{
    node::{
        constructor::Constructor,
        globals::{GlobalVar, Globals},
        index::Index,
        inheritance::Inheritance,
        input::NodeInput,
        recovery::Recovery,
        rule::Rule,
    },
    utils::{error, expect_some, Description, Dump},
};

pub(super) struct NodeVariant {
    pub(super) ident: Ident,
    pub(super) root: Option<Span>,
    pub(super) index: Option<Index>,
    pub(super) rule: Option<Rule>,
    pub(super) trivia: VariantTrivia,
    pub(super) recovery: Option<Recovery>,
    pub(super) inheritance: Inheritance,
    pub(super) constructor: Option<Constructor>,
    pub(super) parser: Option<Expr>,
    pub(super) secondary: Option<Span>,
    pub(super) scope: bool,
    pub(super) description: Description,
    pub(super) dump: Dump,
}

impl TryFrom<Variant> for NodeVariant {
    type Error = Error;

    fn try_from(mut variant: Variant) -> Result<Self> {
        match &variant.fields {
            Fields::Unnamed(fields) => {
                return Err(error!(
                    fields.span(),
                    "Variants with unnamed fields not supported.",
                ));
            }

            _ => (),
        }

        let ident = variant.ident.clone();

        let mut root = None;
        let mut index = None;
        let mut rule = None;
        let mut trivia = VariantTrivia::Inherited;
        let mut recovery = None;
        let mut constructor = None;
        let mut parser = None;
        let mut secondary = None;
        let mut scope = None;
        let mut description = Description::Unset;
        let mut dump = Dump::None;

        for attr in take(&mut variant.attrs) {
            match attr.style {
                AttrStyle::Inner(_) => continue,
                AttrStyle::Outer => (),
            }

            let name = match attr.meta.path().get_ident() {
                Some(ident) => ident.to_string(),
                None => continue,
            };

            let span = attr.span();

            match name.as_str() {
                "root" => {
                    if root.is_some() {
                        return Err(error!(span, "Duplicate Root attribute.",));
                    }

                    root = Some(span);
                }

                "denote" => {
                    if index.is_some() {
                        return Err(error!(span, "Duplicate Denote attribute.",));
                    }

                    index = Some(attr.parse_args::<Index>()?);
                }

                "rule" => {
                    if rule.is_some() {
                        return Err(error!(span, "Duplicate Rule attribute.",));
                    }

                    rule = Some(Rule::try_from(attr)?);
                }

                "trivia" => {
                    if trivia.span().is_some() {
                        return Err(error!(span, "Duplicate Trivia attribute.",));
                    }

                    trivia = match &attr.meta {
                        Meta::Path(..) => VariantTrivia::Empty(span),
                        _ => VariantTrivia::Rule(Rule::try_from(attr)?.zero_or_more()),
                    };
                }

                "recovery" => {
                    if recovery.is_some() {
                        return Err(error!(span, "Duplicate Recovery attribute.",));
                    }

                    recovery = match &attr.meta {
                        Meta::Path(..) => Some(Recovery::empty(span)),
                        _ => Some(attr.parse_args::<Recovery>()?),
                    };
                }

                "constructor" => {
                    if constructor.is_some() {
                        return Err(error!(span, "Duplicate Constructor attribute.",));
                    }

                    constructor = Some(Constructor::try_from(attr)?);
                }

                "parser" => {
                    if parser.is_some() {
                        return Err(error!(span, "Duplicate Parser attribute.",));
                    }

                    parser = Some((span, attr.parse_args::<Expr>()?));
                }

                "secondary" => {
                    if secondary.is_some() {
                        return Err(error!(span, "Duplicate Secondary attribute.",));
                    }

                    secondary = Some(span);
                }

                "scope" => {
                    if scope.is_some() {
                        return Err(error!(span, "Duplicate Scope attribute.",));
                    }

                    scope = Some(span);
                }

                "describe" => {
                    if description.is_set() {
                        return Err(error!(span, "Duplicate Describe attribute.",));
                    }

                    description = Description::try_from(attr)?;
                }

                "dump" => {
                    if dump.span().is_some() {
                        return Err(error!(span, "Duplicate Dump attribute.",));
                    }

                    dump = Dump::try_from(attr)?;
                }

                _ => (),
            }
        }

        if let Some(index) = &index {
            if rule.is_none() && !description.is_set() {
                return Err(error!(
                    index.span(),
                    "Denote attribute is not applicable to unparseable \
                    variants.\n\nTo make the variant parsable annotate this \
                    variant with the #[rule(...)] attribute.\n\nIf this is \
                    intending (e.g. if you want to make this Node variant \
                    describable)\nannotate this variant with the \
                    #[describe(...)] attribute.",
                ));
            }

            if root.is_some() {
                return Err(error!(
                    index.span(),
                    "Root denotation cannot be overridden.",
                ));
            }
        }

        if let Some(span) = root {
            if rule.is_none() {
                return Err(error!(
                    span,
                    "Root variant requires rule expression.\n\
                    Annotate this variant with the #[rule(...)] attribute.",
                ));
            }

            index = Some(Index::Generated(span, 0));
        }

        if let Some(span) = trivia.span() {
            if rule.is_none() {
                return Err(error!(
                    span,
                    "Trivia attribute is not applicable to unparseable \
                    variants.\nTo make the variant parsable annotate this \
                    variant with the #[rule(...)] attribute.",
                ));
            }

            if parser.is_some() {
                return Err(error!(
                    span,
                    "Trivia attribute is not applicable to variants with \
                    overridden parser.\nThe overridden Parser's function \
                    supposed to handle trivia expressions explicitly.",
                ));
            }
        };

        if let Some(recovery) = &recovery {
            if rule.is_none() {
                return Err(error!(
                    recovery.span(),
                    "Recovery attribute is not applicable to unparseable \
                    variants.\nTo make the variant parsable annotate this \
                    variant with the #[rule(...)] attribute.",
                ));
            }

            if parser.is_some() {
                return Err(error!(
                    recovery.span(),
                    "Recovery attribute is not applicable to variants with \
                    overridden parser.\nThe overridden Parser's function \
                    supposed to recover from syntax errors explicitly.",
                ));
            }
        };

        let inheritance = Inheritance::try_from(&variant)?;

        let constructor = match (rule.is_some(), parser.is_some(), constructor) {
            (true, false, None) => Some(Constructor::try_from(variant)?),

            (true, false, Some(constructor)) => Some(constructor),

            (true, true, Some(constructor)) => {
                return Err(error!(
                    constructor.span(),
                    "Overridden constructor conflicts with overridden Parser \
                    (#[parser(...)]) attribute.\nThe overridden Parser's \
                    function supposed to construct the Node explicitly.",
                ));
            }

            (false, _, Some(constructor)) => {
                return Err(error!(
                    constructor.span(),
                    "Overridden constructor attribute is not applicable to \
                    unparseable variants.\nTo make the variant parsable \
                    annotate this variant with the #[rule(...)] attribute.",
                ));
            }

            _ => None,
        };

        let parser = match parser {
            None => None,
            Some((span, parser)) => {
                if rule.is_none() {
                    return Err(error!(
                        span,
                        "Parser attribute is not applicable to unparseable \
                        variants.\nTo make the variant parsable annotate this \
                        variant with the #[rule(...)] attribute.",
                    ));
                }

                Some(parser)
            }
        };

        if let Some(secondary) = &secondary {
            if rule.is_none() {
                return Err(error!(
                    *secondary,
                    "Secondary attribute is not applicable to unparseable \
                    variants.\nTo make the variant parsable annotate this \
                    variant with #[rule(...)] attribute.",
                ));
            }

            if root.is_some() {
                return Err(error!(*secondary, "Root rule must always be primary.",));
            }
        }

        let scope = match rule.is_some() || index.is_some() {
            false => {
                if let Some(span) = scope {
                    return Err(error!(
                        span,
                        "Scope attribute is not applicable to unparseable \
                        variants without denotation.\nAnnotate this variant \
                        with the #[denote(...)] or #[rule(...)] attributes.",
                    ));
                }

                false
            }

            true => scope.is_some(),
        };

        let description = match rule.is_some() || index.is_some() {
            false => {
                if let Some(span) = description.span() {
                    return Err(error!(
                        span,
                        "Describe attribute is not applicable to unparseable \
                        variants without denotation.\nAnnotate this variant \
                        with the #[denote(...)] or #[rule(...)] attributes.",
                    ));
                }

                Description::Unset
            }

            true => description.complete(|| (ident.span(), ident.to_string().to_case(Case::Title))),
        };

        if let Some(span) = dump.span() {
            if rule.is_none() {
                return Err(error!(
                    span,
                    "Dump attribute is not applicable to unparseable \
                    variants.\nTo make the variant parsable annotate this \
                    variant with the #[rule(...)] attribute.",
                ));
            }

            if parser.is_some() {
                return Err(error!(
                    span,
                    "Dump attribute conflicts with overridden parser \
                    (#[parser(...)]).\nWhen the rule has overridden parser \
                    there is nothing to dump.",
                ));
            }

            if let Dump::Meta(_) = &dump {
                return Err(error!(
                    span,
                    "Metadata dump is not applicable to individual rules.",
                ));
            }

            if let Dump::Dry(_) = &dump {
                return Err(error!(
                    span,
                    "Dry dump is not applicable to individual rules.",
                ));
            }

            if let Dump::Trivia(_) = &dump {
                match &trivia {
                    VariantTrivia::Rule(..) => (),
                    _ => {
                        return Err(error!(
                            span,
                            "Trivia dump is not applicable here because the variant \
                            does not override the default trivia expression.",
                        ));
                    }
                }
            }
        }

        let rule = match rule {
            Some(rule) if root.is_some() => Some(rule.greedy()),
            Some(rule) => Some(rule),
            None => None,
        };

        Ok(Self {
            ident,
            root,
            index,
            rule,
            trivia,
            recovery,
            inheritance,
            constructor,
            parser,
            secondary,
            scope,
            description,
            dump,
        })
    }
}

impl NodeVariant {
    pub(super) fn compile_parser_fn(
        &self,
        input: &NodeInput,
        globals: &mut Globals,
        include_trivia: bool,
        include_globals: bool,
        output_comments: bool,
        allow_warnings: bool,
    ) -> Option<TokenStream> {
        let rule = self.rule.as_ref()?;
        let function_ident = self.parser_fn_ident();

        if let Some(parser) = &self.parser {
            return Some(
                input
                    .make_fn(
                        function_ident,
                        false,
                        vec![],
                        Some(input.this()),
                        parser.to_token_stream(),
                        allow_warnings,
                    )
                    .1,
            );
        }

        let context = expect_some!(self.index.as_ref(), "Parsable variant without index.",);
        let constructor = expect_some!(
            self.constructor.as_ref(),
            "Parsable variant without constructor.",
        );
        let variables = expect_some!(rule.variables.as_ref(), "Missing parsable rule variables.",);

        let span = rule.span;

        let recovery_var = match (&self.recovery, &input.recovery) {
            (Some(recovery), _) => globals.recovery(recovery.clone()),
            (None, Some(recovery)) => globals.recovery(recovery.clone()),
            _ => GlobalVar::UnlimitedRecovery,
        };

        let with_trivia = match &self.trivia {
            VariantTrivia::Inherited => input.trivia.is_some(),
            VariantTrivia::Empty(..) => false,
            VariantTrivia::Rule(..) => true,
        };

        let surround_trivia = self.root.is_some();

        let body = rule.compile(
            input,
            globals,
            context,
            &recovery_var,
            with_trivia,
            surround_trivia,
            output_comments,
        );

        let trivia_fn = match self.trivia.rule() {
            Some(trivia) if include_trivia => Some(input.compile_skip_fn(
                globals,
                trivia,
                context,
                false,
                output_comments,
                allow_warnings,
            )),
            _ => None,
        };

        let constructor = constructor.compile(input, variables, allow_warnings);

        let globals = match include_globals {
            false => None,
            true => Some(globals.compile(span, &input.token)),
        };

        Some(
            input
                .make_fn(
                    function_ident,
                    false,
                    vec![],
                    Some(input.this()),
                    quote_spanned!(span=>
                        #globals
                        #trivia_fn
                        #body
                        #constructor
                    ),
                    allow_warnings,
                )
                .1,
        )
    }

    pub(super) fn parser_fn_ident(&self) -> Ident {
        let ident = &self.ident;

        format_ident!("parse_{ident}", span = ident.span())
    }
}

pub(super) enum VariantTrivia {
    Inherited,
    Empty(Span),
    Rule(Rule),
}

impl VariantTrivia {
    #[inline(always)]
    pub(super) fn rule(&self) -> Option<&Rule> {
        match self {
            VariantTrivia::Rule(rule) => Some(rule),
            _ => None,
        }
    }

    #[inline(always)]
    pub(super) fn rule_mut(&mut self) -> Option<&mut Rule> {
        match self {
            VariantTrivia::Rule(rule) => Some(rule),
            _ => None,
        }
    }

    #[inline(always)]
    fn span(&self) -> Option<Span> {
        match self {
            Self::Inherited => None,
            Self::Empty(span) => Some(*span),
            Self::Rule(rule) => Some(rule.span),
        }
    }
}
