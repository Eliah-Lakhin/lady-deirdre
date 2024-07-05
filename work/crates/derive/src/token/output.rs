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

use std::{
    collections::{BTreeMap, BTreeSet},
    mem::take,
    ops::RangeInclusive,
};

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{spanned::Spanned, LitByte, LitStr};

use crate::{
    token::{automata::Terminal, chars::Class, ucd::CharProperties, TokenInput},
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
    ident: &'a Ident,
    input: &'a TokenInput,
    buffering: bool,
    pending: BTreeSet<State>,
    handled: Set<State>,
    transitions: Vec<TokenStream>,
    from: State,
    ascii: BTreeMap<State, Set<u8>>,
    unicode: BTreeMap<State, Set<char>>,
    properties: Option<(CharProperties, State)>,
    other: Option<State>,
}

impl<'a> Output<'a> {
    pub(super) fn compile(input: &'a TokenInput, buffer: bool) -> Vec<TokenStream> {
        let mut output = Output {
            ident: &input.ident,
            input,
            buffering: buffer,
            pending: BTreeSet::new(),
            handled: Set::empty(),
            transitions: Vec::with_capacity(input.automata.transitions().len()),
            from: 0,
            ascii: BTreeMap::new(),
            unicode: BTreeMap::new(),
            properties: None,
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

                let handle = self.handle(to, false, false);

                quote!(#pattern => #handle)
            })
            .collect::<Vec<_>>();

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
                    statements.append(self.handle(to, true, false));
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
                let handle = self.handle(to, true, true);

                quote!(#pattern => #handle)
            })
            .collect::<Vec<_>>();

        if !unicode_cases.is_empty() {
            statements.push_branching(quote!(
                match ch {
                    #(
                        #unicode_cases
                    )*
                    _ => (),
                }
            ))
        }

        if let Some((properties, to)) = self.properties {
            let matcher = Self::properties_matcher(Span::call_site(), properties);
            let mut handle = self.handle(to, true, true);
            handle.surround = true;

            statements.push_branching(quote!(if #matcher #handle));
        }

        match self.other {
            None => statements.push(quote!(break)),
            Some(to) => {
                let handle = self.handle(to, true, false);
                statements.append(handle);
            }
        }

        statements
    }

    fn requires_char(&self) -> bool {
        if self.buffering {
            return true;
        }

        if !self.unicode.is_empty() {
            return true;
        }

        if self.properties.is_some() {
            return true;
        }

        false
    }

    fn handle(&mut self, to: State, unicode: bool, force_continue: bool) -> Statements {
        let transit = self
            .input
            .automata
            .transitions()
            .outgoing(&to)
            .filter(|view| !view.is_empty())
            .is_some();

        let mut statements = Statements::default();

        if transit && self.buffering {
            let string = self.input.ident.span().face_string();

            match unicode {
                false => statements.push(quote!(#string::push(&mut buffer, byte as char))),
                true => statements.push(quote!(#string::push(&mut buffer, ch))),
            }
        }

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
                    let ident = self.ident;

                    statements.push(quote_spanned!(span=>
                        token = {
                            #[allow(unused)]
                            #[inline(always)]
                            fn __construct(fragment: &str) -> #ident {
                                #constructor
                            }

                            __construct(#string::as_str(&buffer))
                        }
                    ))
                }
            }
        }

        match transit {
            false => statements.push(quote!(break)),

            true => {
                if self.from != to {
                    let _ = self.pending.insert(to);

                    statements.push(quote!(state = #to))
                }

                if force_continue {
                    statements.push(quote!(continue));
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

            Class::Props(props) => {
                self.properties = Some((*props, to));
                self.insert_ascii_class(Class::Props(*props), to);
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
        self.properties = None;
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

    fn properties_matcher(span: Span, properties: CharProperties) -> TokenStream {
        let core = span.face_core();

        let mut setters = Vec::new();
        let mut single = None;

        if properties.alpha {
            setters.push(quote_spanned!(span=> props.alpha = true;));
            single = Some(quote_spanned!(span=> #core::lexis::Char::is_alpha(ch)));
        }

        if properties.lower {
            setters.push(quote_spanned!(span=> props.lower = true;));
            single = Some(quote_spanned!(span=> #core::lexis::Char::is_lower(ch)));
        }

        if properties.num {
            setters.push(quote_spanned!(span=> props.num = true;));
            single = Some(quote_spanned!(span=> #core::lexis::Char::is_num(ch)));
        }

        if properties.space {
            setters.push(quote_spanned!(span=> props.space = true;));
            single = Some(quote_spanned!(span=> #core::lexis::Char::is_space(ch)));
        }

        if properties.upper {
            setters.push(quote_spanned!(span=> props.upper = true;));
            single = Some(quote_spanned!(span=> #core::lexis::Char::is_upper(ch)));
        }

        if properties.xid_continue {
            setters.push(quote_spanned!(span=> props.xid_continue = true;));
            single = Some(quote_spanned!(span=> #core::lexis::Char::is_xid_continue(ch)));
        }

        if properties.xid_start {
            setters.push(quote_spanned!(span=> props.xid_start = true;));
            single = Some(quote_spanned!(span=> #core::lexis::Char::is_xid_start(ch)));
        }

        if setters.len() == 1 {
            return expect_some!(single, "Missing single matcher.",);
        }

        quote_spanned! (span=> #core::lexis::Char::has_properties(&const {
            let mut props = #core::lexis::CharProperties::new();

            #( #setters )*

            props
        }))
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
            fn scan(session: &mut impl #core::lexis::LexisSession) -> Self {
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

    fn compile_lookback(&self) -> TokenStream {
        let span = self.ident.span();
        let core = span.face_core();

        let lookback = match &self.lookback {
            Some(expr) => expr.to_token_stream(),
            None => quote_spanned!(span => 1),
        };

        quote_spanned!(span=>
            const LOOKBACK: #core::lexis::Length = #lookback;
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

        let lookback = self.compile_lookback();
        let parse = self.compile_parse_fn();
        let eoi = self.compile_eoi_fn();
        let mismatch = self.compile_mismatch_fn();
        let rule = self.compile_rule_fn();
        let name = self.compile_name_fn();
        let description = self.compile_description_fn();

        quote_spanned!(span=>
            impl #impl_generics #core::lexis::Token for #ident #ty_generics
            #where_clause
            {
                #lookback
                #parse
                #eoi
                #mismatch
                #rule
                #name
                #description
            }
        )
        .to_tokens(tokens)
    }
}
