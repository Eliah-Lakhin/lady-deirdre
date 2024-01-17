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

use std::{
    collections::{BTreeMap, BTreeSet},
    mem::take,
    ops::RangeInclusive,
};

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{LitByte, LitStr};

use crate::{
    token::{automata::Terminal, chars::Class, TokenInput},
    utils::{
        expect_some,
        null,
        system_panic,
        Dump,
        Facade,
        PredictableCollection,
        Set,
        SetImpl,
        State,
    },
};

pub(super) struct Output<'a> {
    input: &'a TokenInput,
    buffering: bool,
    pending: BTreeSet<State>,
    handled: Set<State>,
    transitions: Vec<TokenStream>,
    from: State,
    ascii: BTreeMap<State, Set<u8>>,
    unicode: BTreeMap<State, Set<char>>,
    upper: Option<State>,
    lower: Option<State>,
    num: Option<State>,
    space: Option<State>,
    alphabetic: Option<State>,
    alphanumeric: Option<State>,
    other: Option<State>,
}

impl<'a> Output<'a> {
    pub(super) fn compile(input: &'a TokenInput, buffer: bool) -> Vec<TokenStream> {
        let mut output = Output {
            input,
            buffering: buffer,
            pending: BTreeSet::new(),
            handled: Set::empty(),
            transitions: Vec::with_capacity(input.automata.transitions().len()),
            from: 0,
            ascii: BTreeMap::new(),
            unicode: BTreeMap::new(),
            upper: None,
            lower: None,
            num: None,
            space: None,
            alphabetic: None,
            alphanumeric: None,
            other: None,
        };

        let _ = output.pending.insert(input.automata.start());

        while output.pop() {}

        output.transitions
    }

    fn pop(&mut self) -> bool {
        self.from = match self.pending.pop_first() {
            Some(state) => state,
            None => return false,
        };

        if self.handled.contains(&self.from) {
            return true;
        }

        let outgoing = expect_some!(
            self.input.automata.transitions().outgoing(&self.from),
            "Missing outgoing.",
        );

        if outgoing.is_empty() {
            system_panic!("Empty view.");
        }

        self.reset();

        for (through, to) in outgoing {
            match through {
                Terminal::Null => null!(),
                Terminal::Product(index) => system_panic!("Unfiltered product {index}.",),
                Terminal::Class(class) => self.register_class(class, *to),
            };
        }

        let base = take(&mut self.ascii)
            .into_iter()
            .map(|(to, set)| {
                let pattern = match self.input.dump {
                    Dump::Output(..) => {
                        Self::pattern(set.into_iter().map(|byte| byte as char).collect())
                    }

                    _ => Self::pattern(set),
                };

                let handle = self.handle(to, false);

                quote!(#pattern => #handle)
            })
            .collect::<Vec<_>>();

        self.merge_classes();

        let fallback = self.fallback();

        let from = self.from;

        match base.is_empty() {
            true => self.transitions.push(quote!(#from => #fallback)),

            false => match self.input.dump {
                Dump::Output(..) => self.transitions.push(quote!(#from => match byte as char {
                    #(
                    #base
                    )*

                    _ => #fallback
                })),

                _ => self.transitions.push(quote!(#from => match byte {
                    #(
                    #base
                    )*

                    _ => #fallback
                })),
            },
        }

        if !self.handled.insert(self.from) {
            system_panic!("Duplicate compiled state.");
        }

        true
    }

    fn fallback(&mut self) -> Statements {
        let core = self.input.ident.span().face_core();

        let mut statements = Statements::default();

        if !self.requires_char() {
            match self.other {
                None => statements.push(quote!(break)),

                Some(to) => {
                    statements.push(quote!(unsafe {
                        #core::lexis::LexisSession::consume(session)
                    }));
                    statements.append(self.handle(to, true));
                }
            }

            return statements;
        }

        statements.push(quote!(let ch = unsafe {
            #core::lexis::LexisSession::read(session)
        }));

        let unicode_cases = take(&mut self.unicode)
            .into_iter()
            .map(|(to, set)| {
                let pattern = Self::pattern(set);
                let handle = self.handle(to, true);

                quote!(#pattern => #handle)
            })
            .collect::<Vec<_>>();

        if !unicode_cases.is_empty() {
            statements.push_branching(quote!(
                match ch {#(
                    #unicode_cases,
                )*}
            ))
        }

        if let Some(to) = self.upper {
            let mut handle = self.handle(to, true);
            handle.surround = true;

            statements.push_branching(quote!(if char::is_uppercase(ch) #handle));
        }

        if let Some(to) = self.lower {
            let mut handle = self.handle(to, true);
            handle.surround = true;

            statements.push_branching(quote!(if char::is_lowercase(ch) #handle));
        }

        if let Some(to) = self.alphabetic {
            let mut handle = self.handle(to, true);
            handle.surround = true;

            statements.push_branching(quote!(if char::is_alphabetic(ch) #handle));
        }

        if let Some(to) = self.num {
            let mut handle = self.handle(to, true);
            handle.surround = true;

            statements.push_branching(quote!(if char::is_numeric(ch) #handle));
        }

        if let Some(to) = self.alphanumeric {
            let mut handle = self.handle(to, true);
            handle.surround = true;

            statements.push_branching(quote!(if char::is_alphanumeric(ch) #handle));
        }

        if let Some(to) = self.space {
            let mut handle = self.handle(to, true);
            handle.surround = true;

            statements.push_branching(quote!(if char::is_whitespace(ch) #handle));
        }

        match self.other {
            None => statements.push(quote!(break)),
            Some(to) => {
                let handle = self.handle(to, true);
                statements.append(handle);
            }
        }

        statements
    }

    fn merge_classes(&mut self) {
        if self.upper.is_some() && self.upper == self.lower {
            self.alphabetic = self.upper;
            self.upper = None;
            self.lower = None;
        }

        if self.alphabetic.is_some() && self.alphabetic == self.num {
            self.alphanumeric = self.alphabetic;
            self.alphabetic = None;
            self.num = None;
        }

        if self.other.is_some() && self.other == self.space && self.other == self.alphanumeric {
            self.space = None;
            self.alphanumeric = None;
        }
    }

    fn requires_char(&self) -> bool {
        if self.buffering {
            return true;
        }

        if !self.unicode.is_empty() {
            return true;
        }

        if self.upper.is_some() {
            return true;
        }

        if self.lower.is_some() {
            return true;
        }

        if self.num.is_some() {
            return true;
        }

        if self.space.is_some() {
            return true;
        }

        if self.alphabetic.is_some() {
            return true;
        }

        if self.alphanumeric.is_some() {
            return true;
        }

        false
    }

    fn handle(&mut self, to: State, unicode: bool) -> Statements {
        let mut statements = Statements::default();

        if let Some(index) = self.input.products.get(&to) {
            let core = self.input.ident.span().face_core();

            let variant =
                expect_some!(self.input.variants.get(*index), "Missing product variant.",);

            let ident = &variant.ident;

            statements.push(quote!(unsafe {
                #core::lexis::LexisSession::submit(session)
            }));

            match &variant.constructor {
                None => {
                    statements.push(quote!(token = Self::#ident));
                }

                Some(constructor) => {
                    let span = constructor.span();
                    let string = span.face_string();

                    statements.push(quote_spanned!(span=>
                        token = Self::#constructor(#string::as_str(&buffer))
                    ))
                }
            }
        }

        let transit = self
            .input
            .automata
            .transitions()
            .outgoing(&to)
            .filter(|view| !view.is_empty())
            .is_some();

        match transit {
            false => statements.push(quote!(break)),

            true => {
                if self.buffering {
                    let string = self.input.ident.span().face_string();

                    match unicode {
                        false => statements.push(quote!(#string::push(&mut buffer, byte as char))),
                        true => statements.push(quote!(#string::push(&mut buffer, ch))),
                    }
                }

                if self.from != to {
                    let _ = self.pending.insert(to);

                    statements.push(quote!(state = #to))
                }
            }
        };

        statements
    }

    fn register_class(&mut self, class: &Class, to: State) {
        match class {
            Class::Char(ch) => match ch.is_ascii() {
                true => self.insert_ascii((*ch) as u8, to),
                false => self.insert_unicode(*ch, to),
            },

            Class::Upper => {
                self.upper = Some(to);
                self.insert_ascii_class(Class::Upper, to);
            }

            Class::Lower => {
                self.lower = Some(to);
                self.insert_ascii_class(Class::Lower, to);
            }

            Class::Num => {
                self.num = Some(to);
                self.insert_ascii_class(Class::Num, to);
            }

            Class::Space => {
                self.space = Some(to);
                self.insert_ascii_class(Class::Space, to);
            }

            Class::Other => {
                self.other = Some(to);
            }
        }
    }

    fn insert_ascii_class(&mut self, class: Class, to: State) {
        for byte in 0u8..=0x7F {
            let ch = byte as char;

            if !class.includes(&ch) {
                continue;
            }

            if self.input.alphabet.contains(&ch) {
                continue;
            }

            self.insert_ascii(byte, to)
        }
    }

    fn insert_ascii(&mut self, byte: u8, to: State) {
        self.ascii
            .entry(to)
            .and_modify(|bytes| {
                if !bytes.insert(byte) {
                    let from = self.from;
                    system_panic!("Duplicate transition {from} -> {byte} -> {to}.",);
                }
            })
            .or_insert_with(|| Set::new([byte]));
    }

    fn insert_unicode(&mut self, ch: char, to: State) {
        self.unicode
            .entry(to)
            .and_modify(|chars| {
                if !chars.insert(ch) {
                    let from = self.from;
                    system_panic!("Duplicate transition {from} -> {ch:?} -> {to}.",);
                }
            })
            .or_insert_with(|| Set::new([ch]));
    }

    fn reset(&mut self) {
        self.ascii.clear();
        self.unicode.clear();
        self.upper = None;
        self.lower = None;
        self.num = None;
        self.space = None;
        self.alphabetic = None;
        self.alphanumeric = None;
        self.other = None;
    }

    fn pattern<T: Copy + Ord + Continuous>(set: Set<T>) -> TokenStream {
        enum Group<T: Continuous> {
            Single(T),
            Range(RangeInclusive<T>),
        }

        impl<T: Continuous> ToTokens for Group<T> {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                match self {
                    Self::Single(single) => single.represent().to_tokens(tokens),

                    Self::Range(range) => {
                        let start = range.start().represent();
                        let end = range.end().represent();

                        quote!(#start..=#end).to_tokens(tokens);
                    }
                }
            }
        }

        let mut vector = set.into_iter().collect::<Vec<_>>();
        vector.sort();

        let groups = vector.iter().fold(None, |acc, next| match acc {
            None => Some(vec![Group::Single(*next)]),

            Some(mut grouped) => {
                let group = expect_some!(grouped.pop(), "Missing subgroup.",);

                match group {
                    Group::Single(single) => {
                        if single.continuous_to(next) {
                            grouped.push(Group::Range(single..=*next))
                        } else {
                            grouped.push(Group::Single(single));
                            grouped.push(Group::Single(*next));
                        }
                    }

                    Group::Range(range) => {
                        if range.end().continuous_to(next) {
                            grouped.push(Group::Range(*range.start()..=*next))
                        } else {
                            grouped.push(Group::Range(range));
                            grouped.push(Group::Single(*next));
                        }
                    }
                }

                Some(grouped)
            }
        });

        let groups = expect_some!(groups, "Empty pattern.",);

        quote!(#( #groups )|*)
    }
}

trait Continuous {
    fn continuous_to(&self, next: &Self) -> bool;

    fn represent(&self) -> TokenStream;
}

impl Continuous for u8 {
    #[inline(always)]
    fn continuous_to(&self, next: &Self) -> bool {
        *self + 1 == *next
    }

    fn represent(&self) -> TokenStream {
        LitByte::new(*self, Span::call_site()).to_token_stream()
    }
}

impl Continuous for char {
    #[inline(always)]
    fn continuous_to(&self, next: &Self) -> bool {
        *self as u32 + 1 == *next as u32
    }

    #[inline(always)]
    fn represent(&self) -> TokenStream {
        self.to_token_stream()
    }
}

#[derive(Default)]
struct Statements {
    list: Vec<(bool, TokenStream)>,
    surround: bool,
}

impl ToTokens for Statements {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if self.list.is_empty() {
            match self.surround {
                true => quote!({}).to_tokens(tokens),
                false => quote!((),).to_tokens(tokens),
            }

            return;
        }

        if self.list.len() == 1 {
            let (branching, first) = expect_some!(self.list.first(), "Missing first statement.",);

            match (branching, self.surround) {
                (false, true) => quote!({ #first; }).to_tokens(tokens),
                (false, false) => quote!(#first,).to_tokens(tokens),
                (true, true) => quote!({ #first }).to_tokens(tokens),
                (true, false) => quote!(#first,).to_tokens(tokens),
            }

            return;
        }

        let list = self
            .list
            .iter()
            .map(|(branching, stream)| match *branching {
                true => quote!(#stream),
                false => quote!(#stream;),
            });

        quote!({#(
            #list
        )*})
        .to_tokens(tokens);
    }
}

impl Statements {
    #[inline(always)]
    fn push(&mut self, statement: TokenStream) {
        self.list.push((false, statement))
    }

    #[inline(always)]
    fn push_branching(&mut self, statement: TokenStream) {
        self.list.push((true, statement))
    }

    #[inline(always)]
    fn append(&mut self, mut other: Self) {
        self.list.append(&mut other.list)
    }
}

impl TokenInput {
    fn compile_parse_fn(&self) -> TokenStream {
        let span = self.ident.span();
        let core = span.face_core();
        let panic = span.face_panic();

        let mismatch = &self.mismatch;
        let start = self.automata.start();

        let buffer = match self
            .variants
            .iter()
            .any(|variant| variant.constructor.is_some())
        {
            false => None,
            true => {
                let string = span.face_string();

                Some(quote_spanned!(span =>
                    #[allow(unused_mut)]
                    let mut buffer = #string::new();
                ))
            }
        };

        let transitions = Output::compile(self, buffer.is_some());

        quote_spanned!(span=>
            fn parse(session: &mut impl #core::lexis::LexisSession) -> Self {
                #[allow(unused_mut)]
                let mut state = #start;

                #[allow(unused_mut)]
                let mut token = Self::#mismatch;

                #buffer

                loop {
                    let byte = #core::lexis::LexisSession::advance(session);

                    if byte == 0xFF {
                        break;
                    }

                    match state {
                        #(
                        #transitions
                        )*

                        #[cfg(not(debug_assertions))]
                        _ => (),

                        #[cfg(debug_assertions)]
                        state => #panic("Invalid state {state}."),
                    }
                }

                token
            }
        )
    }

    fn compile_eoi_fn(&self) -> TokenStream {
        let eoi = &self.eoi;
        let span = eoi.span();

        quote_spanned!(span=>
            #[inline(always)]
            fn eoi() -> Self {
                Self::#eoi
            }
        )
    }

    fn compile_mismatch_fn(&self) -> TokenStream {
        let mismatch = &self.mismatch;
        let span = mismatch.span();

        quote_spanned!(span=>
            #[inline(always)]
            fn mismatch() -> Self {
                Self::#mismatch
            }
        )
    }

    fn compile_rule_fn(&self) -> TokenStream {
        let span = self.ident.span();
        let core = span.face_core();

        quote_spanned!(span=>
            #[inline(always)]
            fn rule(self) -> #core::lexis::TokenRule {
                self as u8
            }
        )
    }

    fn compile_blank_fn(&self) -> TokenStream {
        let ident = &self.ident;
        let span = ident.span();
        let core = span.face_core();

        let this = match self.generics.params.is_empty() {
            true => ident.to_token_stream(),

            false => {
                let (_, ty_generics, _) = self.generics.split_for_impl();

                quote_spanned!(span=> #ident::#ty_generics)
            }
        };

        let variants = self
            .variants
            .iter()
            .filter_map(|variant| {
                let span = match variant.blank {
                    Some(span) => span,
                    None => return None,
                };

                let ident = &variant.ident;

                Some(quote_spanned!(span=> #this::#ident as u8))
            })
            .collect::<Vec<_>>();

        let body = match variants.is_empty() {
            true => quote_spanned!(span=> &#core::lexis::EMPTY_TOKEN_SET),

            false => quote_spanned!(span=>
                static BLANKS: #core::lexis::TokenSet
                    = #core::lexis::TokenSet::inclusive(&[
                        #(#variants,)*
                    ]);

                &BLANKS
            ),
        };

        quote_spanned!(span=>
            #[inline(always)]
            fn blanks() -> &'static #core::lexis::TokenSet {
                #body
            }
        )
    }

    fn compile_name_fn(&self) -> TokenStream {
        let span = self.ident.span();
        let core = span.face_core();
        let option = span.face_option();

        let names = self.variants.iter().map(|variant| {
            let ident = &variant.ident;
            let span = ident.span();
            let option = span.face_option();
            let name = LitStr::new(ident.to_string().as_str(), span);

            quote_spanned!(span=>
                if Self::#ident as u8 == rule {
                    return #option::Some(#name);
                }
            )
        });

        quote_spanned!(span=>
            fn rule_name(rule: #core::lexis::TokenRule) -> #option<&'static str> {
                #(#names)*

                None
            }
        )
    }

    fn compile_description_fn(&self) -> TokenStream {
        let span = self.ident.span();
        let core = span.face_core();
        let option = span.face_option();

        let descriptions = self.variants.iter().map(|variant| {
            let ident = &variant.ident;
            let span = ident.span();
            let option = span.face_option();

            let short = variant.description.short();
            let verbose = variant.description.verbose();

            match short == verbose {
                true => quote_spanned!(span=>
                    if Self::#ident as u8 == rule {
                        return #option::Some(#short);
                    }
                ),

                false => quote_spanned!(span=>
                    if Self::#ident as u8 == rule {
                        return match verbose {
                            false => #option::Some(#short),
                            true => #option::Some(#verbose),
                        }
                    }
                ),
            }
        });

        quote_spanned!(span=>
            #[allow(unused_variables)]
            fn rule_description(rule: #core::lexis::TokenRule, verbose: bool) -> #option<&'static str> {
                #(#descriptions)*

                None
            }
        )
    }
}

impl ToTokens for TokenInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Dump::Dry(..) = self.dump {
            return;
        }

        let ident = &self.ident;
        let span = ident.span();
        let core = span.face_core();

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let parse = self.compile_parse_fn();
        let eoi = self.compile_eoi_fn();
        let mismatch = self.compile_mismatch_fn();
        let rule = self.compile_rule_fn();
        let blanks = self.compile_blank_fn();
        let name = self.compile_name_fn();
        let description = self.compile_description_fn();

        quote_spanned!(span=>
            impl #impl_generics #core::lexis::Token for #ident #ty_generics
            #where_clause
            {
                #parse
                #eoi
                #mismatch
                #rule
                #name
                #description
                #blanks
            }
        )
        .to_tokens(tokens)
    }
}
