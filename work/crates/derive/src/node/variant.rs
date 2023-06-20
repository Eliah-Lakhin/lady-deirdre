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

use std::mem::take;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use syn::{spanned::Spanned, AttrStyle, Error, LitStr, Meta, Result, Variant};

use crate::{
    node::{
        constructor::Constructor,
        globals::{GlobalVar, Globals},
        index::Index,
        input::NodeInput,
        recovery::Recovery,
        rule::Rule,
    },
    utils::{error, expect_some, Dump, Facade},
};

pub(super) struct NodeVariant {
    pub(super) ident: Ident,
    pub(super) root: Option<Span>,
    pub(super) index: Option<Index>,
    pub(super) rule: Option<Rule>,
    pub(super) trivia: VariantTrivia,
    pub(super) recovery: Option<Recovery>,
    pub(super) constructor: Option<Constructor>,
    pub(super) parser: Option<Ident>,
    pub(super) secondary: Option<Span>,
    pub(super) description: Option<LitStr>,
    pub(super) dump: Dump,
}

impl TryFrom<Variant> for NodeVariant {
    type Error = Error;

    fn try_from(mut variant: Variant) -> Result<Self> {
        let ident = variant.ident.clone();

        let mut root = None;
        let mut index = None;
        let mut rule = None;
        let mut trivia = VariantTrivia::Inherited;
        let mut recovery = None;
        let mut constructor = None;
        let mut parser = None;
        let mut secondary = None;
        let mut description = None;
        let mut dump = Dump::None;

        for attr in take(&mut variant.attrs) {
            match attr.style {
                AttrStyle::Inner(_) => continue,
                AttrStyle::Outer => (),
            }

            let name = match attr.meta.path().get_ident() {
                Some(ident) => ident,
                None => continue,
            };

            let span = attr.span();

            match name.to_string().as_str() {
                "root" => {
                    if root.is_some() {
                        return Err(error!(span, "Duplicate Root attribute.",));
                    }

                    root = Some(span);
                }

                "index" => {
                    if index.is_some() {
                        return Err(error!(span, "Duplicate Index attribute.",));
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

                    parser = Some((span, attr.parse_args::<Ident>()?));
                }

                "secondary" => {
                    if secondary.is_some() {
                        return Err(error!(span, "Duplicate Secondary attribute.",));
                    }

                    secondary = Some(span);
                }

                "describe" => {
                    if description.is_some() {
                        return Err(error!(span, "Duplicate Describe attribute.",));
                    }

                    description = Some((span, attr.parse_args::<LitStr>()?));
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
            if rule.is_none() && description.is_none() {
                return Err(error!(
                    index.span(),
                    "Index attribute is not applicable to unparseable \
                    variants.\n\nTo make the variant parsable annotate this \
                    variant with #[rule(...)] attribute.\n\nIf this is \
                    intending (e.g. if you want to make this Node variant \
                    describable)\nalso annotate this variant with \
                    #[describe(...)] attribute.",
                ));
            }

            if root.is_some() {
                return Err(error!(index.span(), "Root index cannot be overridden.",));
            }
        }

        if let Some(span) = root {
            if rule.is_none() {
                return Err(error!(
                    span,
                    "Root variant requires rule expression.\n\
                    Annotate this variant with #[rule(...)] attribute.",
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
                    variant with #[rule(...)] attribute.",
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
                    variant with #[rule(...)] attribute.",
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
                    annotate this variant with #[rule(...)] attribute.",
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
                        variant with #[rule(...)] attribute.",
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

        let description = match rule.is_some() || index.is_some() {
            false => {
                if let Some((span, _)) = description {
                    return Err(error!(
                        span,
                        "Describe attribute is not applicable to unparseable \
                        variants without index.\nAnnotate this variant with \
                        #[index(...)] or #[rule(...)] attributes.",
                    ));
                }

                None
            }

            true => description.map(|(_, string)| string).or_else(|| {
                Some(LitStr::new(
                    ident.to_string().to_case(Case::Title).as_str(),
                    ident.span(),
                ))
            }),
        };

        if let Some(span) = dump.span() {
            if rule.is_none() {
                return Err(error!(
                    span,
                    "Dump attribute is not applicable to unparseable \
                    variants.\nTo make the variant parsable annotate this \
                    variant with #[rule(...)] attribute.",
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
                            "Trivia dump is not applicable here, because the rule \
                            does not override default trivia expression.",
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
            constructor,
            parser,
            secondary,
            description,
            dump,
        })
    }
}

impl NodeVariant {
    pub(super) fn compile_parser_function(
        &self,
        input: &NodeInput,
        globals: &mut Globals,
        include_trivia: bool,
        include_globals: bool,
        allow_warnings: bool,
    ) -> Option<TokenStream> {
        let rule = self.rule.as_ref()?;
        let context = expect_some!(self.index.as_ref(), "Parsable variant without index.",);
        let constructor = expect_some!(
            self.constructor.as_ref(),
            "Parsable variant without constructor.",
        );
        let variables = expect_some!(rule.variables.as_ref(), "Missing parsable rule variables.",);
        let function_ident = self.generated_parser_ident();

        let span = rule.span;
        let core = span.face_core();

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
        );

        let trivia_function = match self.trivia.rule() {
            Some(trivia) if include_trivia => {
                Some(input.compile_skip_function(globals, trivia, context, false, allow_warnings))
            }
            _ => None,
        };

        let constructor = constructor.compile(input, variables);

        let (impl_generics, _, where_clause) = input.generics.func.split_for_impl();

        let code = &input.generics.code;

        let this = input.this();

        let allowed_warnings = match allow_warnings {
            true => Some(NodeInput::base_warnings(span)),
            false => None,
        };

        let globals = match include_globals {
            false => None,
            true => Some(globals.compile(span, &input.token)),
        };

        Some(quote_spanned!(span=>
            #allowed_warnings
            fn #function_ident #impl_generics (
                session: &mut impl #core::syntax::SyntaxSession<#code, Node = #this>,
            ) -> #this #where_clause {
                #globals
                #trivia_function
                #body
                #constructor
            }
        ))
    }

    pub(super) fn generated_parser_ident(&self) -> Ident {
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
