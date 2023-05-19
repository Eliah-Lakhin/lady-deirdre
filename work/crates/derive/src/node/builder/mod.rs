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

use std::collections::hash_map::Keys;

use proc_macro2::Ident;
use syn::{
    parse::ParseStream,
    spanned::Spanned,
    AttrStyle,
    Attribute,
    Data,
    DeriveInput,
    Error,
    Generics,
    Result,
    Type,
    Variant,
};

use crate::{
    node::{
        automata::{conflicts::CheckConflicts, scope::Scope, skip::IsSkipAutomata, NodeAutomata},
        builder::{kind::VariantKind, variant::NodeVariant},
        regex::{
            encode::Encode,
            inline::Inline,
            operand::RegexOperand,
            operator::RegexOperator,
            prefix::{Leftmost, RegexPrefix},
            skip::IsSkipRegex,
            Regex,
        },
    },
    utils::{debug_panic, Map, PredictableCollection, Set},
};

pub(in crate::node) mod constructor;
pub(in crate::node) mod index;
pub(in crate::node) mod kind;
pub(in crate::node) mod rule;
pub(in crate::node) mod variant;

pub(in crate::node) struct Builder {
    node_name: Ident,
    generics: Generics,
    token_type: Option<Type>,
    error_type: Option<Type>,
    scope: Scope,
    skip: Option<Regex>,
    inline_map: Map<Ident, Regex>,
    variant_map: Map<Ident, NodeVariant>,
    skip_leftmost: Leftmost,
    skip_automata: Option<NodeAutomata>,
    synchronization: Map<Ident, Ident>,
}

impl<'a> TryFrom<&'a DeriveInput> for Builder {
    type Error = Error;

    fn try_from(input: &'a DeriveInput) -> Result<Self> {
        let node_name = input.ident.clone();
        let generics = input.generics.clone();

        let mut builder = Self {
            node_name,
            generics,
            token_type: None,
            error_type: None,
            scope: Scope::default(),
            skip: None,
            inline_map: Map::empty(),
            variant_map: Map::empty(),
            skip_leftmost: Leftmost::default(),
            skip_automata: None,
            synchronization: Map::empty(),
        };

        let data = match &input.data {
            Data::Enum(data) => data,

            other => {
                let span = match other {
                    Data::Struct(data) => data.struct_token.span,
                    Data::Union(data) => data.union_token.span,
                    _ => debug_panic!("Unsupported Item format."),
                };

                return Err(Error::new(
                    span,
                    "Node must be derived on the enum type with variants representing \
                    syntax variants.",
                ));
            }
        };

        for attribute in &input.attrs {
            match attribute.style {
                AttrStyle::Inner(_) => continue,
                AttrStyle::Outer => (),
            }

            let name = match attribute.path.get_ident() {
                None => continue,
                Some(name) => name,
            };

            match name.to_string().as_str() {
                "token" => {
                    builder.set_token_type(attribute)?;
                }

                "error" => {
                    builder.set_error_type(attribute)?;
                }

                "skip" => {
                    builder.set_skip(attribute)?;
                }

                "define" => {
                    builder.add_inline(attribute)?;
                }

                _ => continue,
            }
        }

        for variant in &data.variants {
            builder.add_variant(variant)?;
        }

        builder.check_error_type()?;
        builder.check_token_type()?;
        builder.check_root()?;
        builder.check_references()?;
        builder.build_indices()?;
        builder.build_leftmost()?;
        builder.build_skip()?;
        builder.build_automata()?;
        builder.check_conflicts()?;
        builder.build_synchronizations()?;

        Ok(builder)
    }
}

impl<'a> IntoIterator for &'a Builder {
    type Item = &'a Ident;
    type IntoIter = Keys<'a, Ident, NodeVariant>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.variant_map.keys()
    }
}

impl Builder {
    #[inline(always)]
    pub(in crate::node) fn node_name(&self) -> &Ident {
        &self.node_name
    }

    #[inline(always)]
    pub(in crate::node) fn token_type(&self) -> &Type {
        self.token_type
            .as_ref()
            .expect("Internal error. Missing token type.")
    }

    #[inline(always)]
    pub(in crate::node) fn error_type(&self) -> &Type {
        self.error_type
            .as_ref()
            .expect("Internal error. Missing error type.")
    }

    #[inline(always)]
    pub(in crate::node) fn generics(&self) -> &Generics {
        &self.generics
    }

    #[inline(always)]
    pub(in crate::node) fn get_inline(&self, name: &Ident) -> Option<&Regex> {
        self.inline_map.get(name)
    }

    #[inline(always)]
    pub(in crate::node) fn get_variant(&self, name: &Ident) -> Option<&NodeVariant> {
        self.variant_map.get(name)
    }

    #[inline(always)]
    pub(in crate::node) fn variant(&self, name: &Ident) -> &NodeVariant {
        self.variant_map
            .get(name)
            .as_ref()
            .expect("Internal error. Unknown variant.")
    }

    #[inline(always)]
    pub(in crate::node) fn variants_count(&self) -> usize {
        self.variant_map.len()
    }

    #[inline(always)]
    pub(in crate::node) fn skip_leftmost(&self) -> &Leftmost {
        &self.skip_leftmost
    }

    #[inline(always)]
    pub(in crate::node) fn skip_automata(&self) -> Option<&NodeAutomata> {
        self.skip_automata.as_ref()
    }

    #[inline(always)]
    pub(in crate::node) fn synchronization(&self) -> &Map<Ident, Ident> {
        &self.synchronization
    }

    #[inline(always)]
    pub(in crate::node) fn modify(
        &mut self,
        name: &Ident,
        mut function: impl FnMut(&mut Self, &mut NodeVariant) -> Result<()>,
    ) -> Result<()> {
        let (name, mut variant) = self
            .variant_map
            .remove_entry(name)
            .expect("Internal error. Unknown variant.");

        function(self, &mut variant)?;

        assert!(
            self.variant_map.insert(name, variant).is_none(),
            "Internal error. Duplicate variant."
        );

        Ok(())
    }

    #[inline(always)]
    pub(in crate::node) fn scope(&mut self) -> &mut Scope {
        &mut self.scope
    }

    fn set_token_type(&mut self, attribute: &Attribute) -> Result<()> {
        if self.token_type.is_some() {
            return Err(Error::new(attribute.span(), "Duplicate Token attribute."));
        }

        self.token_type = Some(attribute.parse_args::<Type>()?);

        Ok(())
    }

    fn set_error_type(&mut self, attribute: &Attribute) -> Result<()> {
        if self.error_type.is_some() {
            return Err(Error::new(attribute.span(), "Duplicate Error attribute."));
        }

        self.error_type = Some(attribute.parse_args::<Type>()?);

        Ok(())
    }

    fn set_skip(&mut self, attribute: &Attribute) -> Result<()> {
        if self.skip.is_some() {
            return Err(Error::new(attribute.span(), "Duplicate Skip attribute."));
        }

        let mut skip = attribute.parse_args::<Regex>()?;

        skip.inline(self)?;
        skip.is_skip()?;

        self.skip = Some(skip);

        Ok(())
    }

    fn add_inline(&mut self, attribute: &Attribute) -> Result<()> {
        let (name, mut regex) = attribute.parse_args_with(|input: ParseStream| {
            let name = input.parse::<Ident>()?;
            let _ = input.parse::<Token![=]>()?;

            let expression = input.parse::<Regex>()?;

            Ok((name, expression))
        })?;

        self.is_vacant(&name)?;

        regex.inline(self)?;

        assert!(
            self.inline_map.insert(name, regex).is_none(),
            "Internal error. Inline redefined.",
        );

        Ok(())
    }

    fn add_variant(&mut self, variant: &Variant) -> Result<()> {
        let mut variant = NodeVariant::try_from(variant)?;

        self.is_vacant(variant.name())?;

        variant.inline(self)?;

        assert!(
            self.variant_map
                .insert(variant.name().clone(), variant)
                .is_none(),
            "Internal error. Variant redefined.",
        );

        Ok(())
    }

    fn check_error_type(&self) -> Result<()> {
        if self.error_type.is_none() {
            return Err(Error::new(
                self.node_name.span(),
                "Error Type must be specified explicitly.\nUse #[error(<type name>)] \
                attribute on the derived type to specify Error type.\nFor example you can specify \
                default \"SyntaxError\" error type.",
            ));
        }

        Ok(())
    }

    fn check_token_type(&self) -> Result<()> {
        if self.token_type.is_none() {
            return Err(Error::new(
                self.node_name.span(),
                "Token Type must be specified explicitly.\nUse #[token(<type name>)] \
                attribute on the derived type to specify Token type.",
            ));
        }

        Ok(())
    }

    fn check_root(&self) -> Result<()> {
        let mut found = false;

        for variant in self.variant_map.values() {
            match variant.kind() {
                VariantKind::Root(..) if !found => found = true,

                VariantKind::Root(span) if found => {
                    return Err(Error::new(
                        *span,
                        "Duplicate Root rule.\nThe syntax must specify only one Root rule.",
                    ));
                }

                _ => (),
            }
        }

        if !found {
            return Err(Error::new(
                self.node_name.span(),
                "Node syntax must specify a Root rule.\nAnnotate one of the enum variants \
                with #[root] attribute.",
            ));
        }

        Ok(())
    }

    fn check_references(&self) -> Result<()> {
        let mut visited = Set::empty();

        let mut pending = self
            .variant_map
            .iter()
            .filter(|(_, variant)| match variant.kind() {
                VariantKind::Root(..) | VariantKind::Comment(..) => true,
                _ => false,
            })
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();

        while let Some(next) = pending.pop() {
            if visited.contains(&next) {
                continue;
            }

            let variant = self.variant(&next);

            let references = variant.check_references(self)?;

            for reference in references {
                pending.push(reference);
            }

            let _ = visited.insert(next);
        }

        for (_, variant) in &self.variant_map {
            let span = match variant.kind() {
                VariantKind::Unspecified(..) | VariantKind::Root(..) | VariantKind::Comment(..) => {
                    continue;
                }

                VariantKind::Sentence(span) => span,
            };

            let explicit_index = variant.index().filter(|index| index.explicit).is_some();

            if !explicit_index && !visited.contains(variant.name()) {
                return Err(Error::new(
                    *span,
                    "Variant's rule is not referred by any other rule.\nEvery \
                    parsable variant without explicit index except the root rule or a \
                    comment rule must be referred directly or indirectly from the root.\n\
                    If this is intended (e.g. if you want to descend into this rule \
                    manually) mark this variant with #[index(<number>)] \
                    index override attribute.\nLater on one will be able to descend \
                    into this rule using that index number.",
                ));
            }
        }

        Ok(())
    }

    fn build_indices(&mut self) -> Result<()> {
        let mut index_map = Map::empty();
        let mut in_use = Set::empty();

        for (_, variant) in &self.variant_map {
            if let VariantKind::Unspecified(..) = variant.kind() {
                continue;
            }

            if let Some(index) = variant.index() {
                if let Some(previous) = index_map.insert(index.index, variant.name()) {
                    return Err(Error::new(
                        index.span(),
                        format!(
                            "Rule \"{previous}\" has the same index.\nRule indices \
                            must be unique."
                        ),
                    ));
                }

                let _ = in_use.insert(index.index);
            }
        }

        let mut next = 1;

        for (_, variant) in &mut self.variant_map {
            if let VariantKind::Unspecified(..) = variant.kind() {
                continue;
            }

            while in_use.contains(&next) {
                next += 1;
            }

            if variant.set_index(next) {
                let _ = in_use.insert(next);
                next += 1;
            }
        }

        Ok(())
    }

    fn build_leftmost(&mut self) -> Result<()> {
        for variant in &self.variant_map.keys().cloned().collect::<Vec<_>>() {
            self.modify(variant, |builder, variant| variant.build_leftmost(builder))?
        }

        Ok(())
    }

    fn build_skip(&mut self) -> Result<()> {
        let skip = self
            .variant_map
            .values()
            .fold(self.skip.clone(), |accumulator, variant| {
                match variant.kind() {
                    VariantKind::Comment(..) => (),

                    _ => return accumulator,
                }

                let comment = Regex::Operand(RegexOperand::Rule {
                    name: variant.name().clone(),
                    capture: None,
                });

                match accumulator {
                    None => Some(comment),

                    Some(accumulator) => Some(Regex::Binary {
                        operator: RegexOperator::Union,
                        left: Box::new(accumulator),
                        right: Box::new(comment),
                    }),
                }
            });

        let skip = match skip {
            None => return Ok(()),
            Some(skip) => Regex::Unary {
                operator: RegexOperator::ZeroOrMore { separator: None },
                inner: Box::new(skip),
            },
        };

        self.skip_leftmost = skip.leftmost();

        for node in self.skip_leftmost.nodes().clone() {
            self.skip_leftmost
                .append(self.variant(&node).leftmost().clone());
        }

        let skip_automata = skip.encode(&mut self.scope)?;

        skip_automata.is_skip()?;

        self.skip_automata = Some(skip_automata);

        for variant in &self.variant_map.keys().cloned().collect::<Vec<_>>() {
            self.modify(variant, |_, variant| {
                variant.inject_skip(&skip);

                Ok(())
            })
            .expect("Internal error. Skip injection failure");
        }

        Ok(())
    }

    fn build_automata(&mut self) -> Result<()> {
        for variant in &self.variant_map.keys().cloned().collect::<Vec<_>>() {
            self.modify(variant, |builder, variant| variant.build_automata(builder))?
        }

        Ok(())
    }

    fn check_conflicts(&self) -> Result<()> {
        for variant in self.variant_map.values() {
            let allow_skips = match variant.kind() {
                VariantKind::Unspecified(..) => continue,
                VariantKind::Root(..) | VariantKind::Sentence(..) => false,
                VariantKind::Comment(..) => true,
            };

            variant.automata().check_conflicts(self, allow_skips)?;
        }

        Ok(())
    }

    fn build_synchronizations(&mut self) -> Result<()> {
        enum Suffix<'a> {
            Leftmost(&'a Ident),
            Rightmost(&'a Ident),
        }

        let mut set = Map::empty();

        for variant in self.variant_map.values() {
            match variant.kind() {
                VariantKind::Sentence(..) if variant.is_global_synchronization() => (),

                _ => continue,
            }

            let variant_synchronization = variant.synchronization();

            let open = variant_synchronization
                .open()
                .expect("Internal error. Missing synchronization's Open token.");

            let close = variant_synchronization
                .close()
                .expect("Internal error. Missing synchronization's Close token.");

            if let Some(candidate) = self.synchronization.get(open) {
                if candidate == close {
                    continue;
                }
            }

            if let Some(conflict) = set.insert(
                open,
                Suffix::Leftmost(variant_synchronization.variant_name()),
            ) {
                return match &conflict {
                    Suffix::Leftmost(conflict) => Err(Error::new(
                        variant_synchronization.span(),
                        format!(
                            "Synchronization conflict.\nRule's leftmost token \"${}\" \
                            conflicts with \"{}\" rule's leftmost token.\n.The set of all \
                            leftmost and rightmost tokens across all synchronization rules \
                            must be unique.",
                            open, conflict
                        ),
                    )),

                    Suffix::Rightmost(conflict) => Err(Error::new(
                        variant_synchronization.span(),
                        format!(
                            "Synchronization conflict.\nRule's leftmost token \"${}\" \
                            conflicts with \"{}\" rule's rightmost token.\n.The set of all \
                            leftmost and rightmost tokens across all synchronization rules \
                            must be unique.",
                            open, conflict
                        ),
                    )),
                };
            }

            if let Some(conflict) = set.insert(
                close,
                Suffix::Rightmost(variant_synchronization.variant_name()),
            ) {
                return match &conflict {
                    Suffix::Leftmost(conflict) => Err(Error::new(
                        variant_synchronization.span(),
                        format!(
                            "Synchronization conflict.\nRule's rightmost token \"${}\" \
                            conflicts with \"{}\" rule's leftmost token.\n.The set of all \
                            leftmost and rightmost tokens across all synchronization rules \
                            must be unique.",
                            open, conflict
                        ),
                    )),

                    Suffix::Rightmost(conflict) => Err(Error::new(
                        variant_synchronization.span(),
                        format!(
                            "Synchronization conflict.\nRule's rightmost token \"${}\" \
                            conflicts with \"{}\" rule's rightmost token.\n.The set of all \
                            leftmost and rightmost tokens across all synchronization rules \
                            must be unique.",
                            open, conflict
                        ),
                    )),
                };
            }

            let _ = self.synchronization.insert(open.clone(), close.clone());
        }

        Ok(())
    }

    fn is_vacant(&self, name: &Ident) -> Result<()> {
        if self.inline_map.contains_key(name) {
            return Err(Error::new(
                name.span(),
                "An inline expression with this name already defined.",
            ));
        }

        if self.variant_map.contains_key(name) {
            return Err(Error::new(
                name.span(),
                "An enum variant with this name already defined.",
            ));
        }

        Ok(())
    }
}
