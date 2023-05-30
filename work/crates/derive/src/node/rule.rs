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
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{parse::ParseStream, spanned::Spanned, Attribute, Error, Result, Type};

use crate::{
    node::{
        automata::{NodeAutomata, NodeAutomataImpl, Scope, Terminal},
        index::Index,
        input::NodeInput,
        leftmost::Leftmost,
        recovery::Recovery,
        regex::{Operator, Regex, RegexImpl},
        token::TokenLit,
        variables::VariableMap,
    },
    utils::{
        expect_some,
        null,
        system_panic,
        AutomataContext,
        Map,
        PredictableCollection,
        Set,
        SetImpl,
        SpanFacade,
        State,
    },
};

pub(super) struct Rule {
    pub(super) span: Span,
    pub(super) regex: Regex,
    pub(super) leftmost: Option<Leftmost>,
    pub(super) automata: Option<NodeAutomata>,
    pub(super) variables: Option<VariableMap>,
}

impl TryFrom<Attribute> for Rule {
    type Error = Error;

    #[inline(always)]
    fn try_from(attr: Attribute) -> Result<Self> {
        let span = attr.span();

        attr.parse_args_with(|input: ParseStream| {
            let regex = input.parse::<Regex>()?;

            Ok(Self {
                span,
                regex,
                leftmost: None,
                automata: None,
                variables: None,
            })
        })
    }
}

impl Rule {
    #[inline]
    pub(super) fn zero_or_more(mut self) -> Self {
        self.regex = Regex::Unary(Operator::ZeroOrMore(None), Box::new(self.regex));

        self
    }

    #[inline]
    pub(super) fn encode(&mut self, scope: &mut Scope) -> Result<()> {
        self.leftmost = Some(Leftmost::from(&self.regex));

        let mut automata = self.regex.encode(scope)?;
        automata.merge_captures(scope)?;

        let variables = VariableMap::try_from(&automata)?;

        self.automata = Some(automata);
        self.variables = Some(variables);

        Ok(())
    }

    pub(super) fn compile(
        &self,
        input: &NodeInput,
        globals: &mut Globals,
        context: Index,
        recovery_var: &GlobalVar,
        with_trivia: bool,
        surround: bool,
    ) -> TokenStream {
        let automata = expect_some!(self.automata.as_ref(), "Missing automata.",);
        let variables = expect_some!(self.variables.as_ref(), "Missing variable map.",);

        let delimiter = automata.delimiter();

        let automata = {
            let mut scope = Scope::default();
            scope.copy(automata)
        };

        let span = self.span;
        let core = span.face_core();
        let unreachable = span.face_unreachable();

        let start = automata.start();

        let init_vars = variables.init();
        let init_first;
        let init_step;
        let finish_step;

        match with_trivia {
            false => {
                init_first = None;
                init_step = None;
                finish_step = None;
            }

            true => match surround {
                false => {
                    init_first = Some(quote_spanned!(span=> let mut first = true;));
                    init_step = Some(quote_spanned!(span=> match first {
                        true => first = false,
                        false => skip_trivia(session),
                    }));
                    finish_step = None;
                }

                true => {
                    init_first = None;
                    init_step = Some(quote_spanned!(span=> skip_trivia(session);));
                    finish_step = Some(quote_spanned!(span=> skip_trivia(session);));
                }
            },
        }

        let mut transitions = automata
            .transitions()
            .view()
            .keys()
            .map(|from| {
                let handler = self.compile_outgoing(
                    input,
                    globals,
                    context,
                    &automata,
                    &variables,
                    delimiter,
                    recovery_var,
                    *from,
                );

                (
                    *from,
                    quote_spanned!(span=> #from => {
                        #handler
                    }),
                )
            })
            .collect::<Vec<_>>();

        transitions.sort_by_key(|(key, _)| *key);

        let transitions = transitions.into_iter().map(|(_, stream)| stream);

        quote_spanned!(span=>
            let mut state = #start;
            let mut site = #core::lexis::SiteRef::nil();
            #init_first
            #init_vars

            loop {
                #init_step

                match state {
                    #(
                        #transitions
                    )*
                    other => #unreachable("Unknown state {other}."),
                }
            }

            #finish_step
        )
    }

    fn compile_outgoing(
        &self,
        input: &NodeInput,
        globals: &mut Globals,
        context: Index,
        automata: &NodeAutomata,
        variables: &VariableMap,
        delimiter: Option<&TokenLit>,
        recovery_var: &GlobalVar,
        from: State,
    ) -> TokenStream {
        let outgoing = expect_some!(
            automata.transitions().outgoing(&from),
            "Empty state transitions.",
        );

        let mut covered = Set::with_capacity(input.alphabet.len());
        let mut uncovered = input.alphabet.clone();
        uncovered.insert(TokenLit::Other(self.span));

        let mut expected_tokens = Set::with_capacity(input.alphabet.len());
        let mut expected_rules = Set::with_capacity(input.variants.len());
        let mut by_token = Map::with_capacity(input.alphabet.len());

        for (through, to) in outgoing {
            match through {
                Terminal::Null => null!(),

                Terminal::Token(capture, lit) => {
                    let _ = covered.insert(lit.clone());

                    if !uncovered.remove(lit) {
                        system_panic!("Duplicate uncovered token.",);
                    }

                    let previous = by_token.insert(
                        lit.clone(),
                        Action {
                            transition: *to,
                            capture: capture.clone(),
                            descend: None,
                            insert: None,
                        },
                    );

                    if previous.is_some() {
                        system_panic!("Duplicate by_token entry.")
                    }

                    if let TokenLit::Ident(ident) = lit {
                        let _ = expected_tokens.insert(ident.clone());
                    }
                }

                Terminal::Node(capture, ident) => {
                    let variant = expect_some!(input.variants.get(ident), "Unresolved reference.",);
                    let rule =
                        expect_some!(variant.rule.as_ref(), "Reference to unparseable variant .",);
                    let leftmost =
                        expect_some!(rule.leftmost.as_ref(), "Missing leftmost of rule.",);
                    let matches = expect_some!(leftmost.matches(), "Unresolved leftmost matches.",);

                    for lit in matches {
                        let _ = covered.insert(lit.clone());

                        if !uncovered.remove(lit) {
                            system_panic!("Duplicate uncovered token.");
                        }

                        let previous = by_token.insert(
                            lit.clone(),
                            Action {
                                transition: *to,
                                capture: capture.clone(),
                                descend: Some(ident.clone()),
                                insert: None,
                            },
                        );

                        if previous.is_some() {
                            system_panic!("Duplicate by_token entry.")
                        }
                    }

                    let index =
                        expect_some!(variant.index.as_ref(), "Missing parsable variant index.",);

                    expected_rules.insert(index);
                }
            }
        }

        let mut insert_map = Map::with_capacity(uncovered.len());

        'outer: for (insert, to) in outgoing {
            if let Terminal::Token(_, TokenLit::Other(..)) = insert {
                insert_map.clear();
                break 'outer;
            }

            if let Some(outgoing) = automata.transitions().outgoing(&to) {
                for (through, to) in outgoing {
                    match through {
                        Terminal::Null => null!(),

                        Terminal::Token(capture, lit) => {
                            if by_token.contains_key(lit) {
                                insert_map.clear();
                                break 'outer;
                            }

                            let previous = insert_map.insert(
                                lit.clone(),
                                Action {
                                    transition: *to,
                                    capture: capture.clone(),
                                    descend: None,
                                    insert: Some(insert.clone()),
                                },
                            );

                            if previous.is_some() {
                                insert_map.clear();
                                break 'outer;
                            }
                        }

                        Terminal::Node(capture, ident) => {
                            let variant =
                                expect_some!(input.variants.get(ident), "Unresolved reference.",);
                            let rule = expect_some!(
                                variant.rule.as_ref(),
                                "Reference to unparseable variant .",
                            );
                            let leftmost =
                                expect_some!(rule.leftmost.as_ref(), "Missing leftmost of rule.",);
                            let matches =
                                expect_some!(leftmost.matches(), "Unresolved leftmost matches.",);

                            for lit in matches {
                                if by_token.contains_key(lit) {
                                    insert_map.clear();
                                    break 'outer;
                                }

                                let previous = insert_map.insert(
                                    lit.clone(),
                                    Action {
                                        transition: *to,
                                        capture: capture.clone(),
                                        descend: Some(ident.clone()),
                                        insert: Some(insert.clone()),
                                    },
                                );

                                if previous.is_some() {
                                    insert_map.clear();
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }
        }

        for (lit, action) in insert_map {
            if !uncovered.remove(&lit) {
                system_panic!("Duplicate uncovered token.");
            }

            let previous = by_token.insert(lit, action);

            if previous.is_some() {
                system_panic!("Duplicate by_token entry.")
            }
        }

        let mut stream = TokenStream::new();

        let is_final = automata.finish().contains(&from);

        let span = self.span;
        let core = span.face_core();
        let option = span.face_option();

        let missing;
        let expected_tokens_var;
        let expected_rules_var;

        match is_final {
            true => {
                missing = quote_spanned!(span=> break,);
                expected_tokens_var = GlobalVar::EmptyTokenSet.compile(span);
                expected_rules_var = GlobalVar::EmptyRuleSet.compile(span);
            }

            false => {
                expected_tokens_var = globals
                    .inclusive_tokens(
                        expected_tokens
                            .into_iter()
                            .map(|ident| TokenLit::Ident(ident)),
                    )
                    .compile(span);
                expected_rules_var = globals
                    .rules(expected_rules.into_iter().cloned())
                    .compile(span);

                missing = quote_spanned!(span=> {
                    site = #core::lexis::TokenCursor::site_ref(session, 0);
                    #core::syntax::SyntaxSession::error(
                        session,
                        #core::syntax::SyntaxError {
                            span: site..site,
                            context: #context,
                            expected_tokens: &#expected_tokens_var,
                            expected_rules: &#expected_rules_var,
                        },
                    );

                    break;
                })
            }
        };

        let mut by_action = BTreeMap::<Action, Set<TokenLit>>::new();

        for (lit, action) in by_token {
            by_action
                .entry(action)
                .and_modify(|set| {
                    let _ = set.insert(lit.clone());
                })
                .or_insert_with(|| Set::new([lit]));
        }

        quote_spanned!(span=>
            let token = match #core::lexis::TokenCursor::token(session, 0) {
                #option::Some(token) => token,
                #option::None => #missing
            };
        )
        .to_tokens(&mut stream);

        for (action, set) in by_action {
            let to = action.transition;

            let has_outgoing = automata.transitions().outgoing(&to).is_some();
            let is_final = automata.finish().contains(&to);
            let is_looping = from == to;

            let mut body = TokenStream::new();

            if let Some(insert) = &action.insert {
                match insert {
                    Terminal::Null => null!(),

                    Terminal::Token(capture, lit) => {
                        if let Some(variable) = capture {
                            variables.get(variable).write_nil().to_tokens(&mut body);
                        }

                        let var = globals
                            .inclusive_tokens([lit.clone()].into_iter())
                            .compile(span);

                        quote_spanned!(span=>
                            site = #core::lexis::TokenCursor::site_ref(session, 0);
                            #core::syntax::SyntaxSession::error(
                                session,
                                #core::syntax::SyntaxError {
                                    span: site..site,
                                    context: #context,
                                    expected_tokens: &#var,
                                    expected_rules: &#core::syntax::EMPTY_RULE_SET,
                                });
                        )
                        .to_tokens(&mut body)
                    }

                    Terminal::Node(capture, ident) => {
                        if let Some(variable) = capture {
                            variables.get(variable).write_nil().to_tokens(&mut body);
                        }

                        let variant =
                            expect_some!(input.variants.get(ident), "Unresolved reference.",);

                        let index = expect_some!(
                            variant.index.as_ref(),
                            "Missing parsable variant index.",
                        );

                        let var = globals.rules([*index].into_iter()).compile(span);

                        quote_spanned!(span=>
                            site = #core::lexis::TokenCursor::site_ref(session, 0);
                            #core::syntax::SyntaxSession::error(
                                session,
                                #core::syntax::SyntaxError {
                                    span: site..site,
                                    context: #context,
                                    expected_tokens: &#core::lexis::EMPTY_TOKEN_SET,
                                    expected_rules: &#var,
                                });
                        )
                        .to_tokens(&mut body)
                    }
                }
            }

            match action.descend {
                None => {
                    if let Some(variable) = action.capture {
                        variables
                            .get(&variable)
                            .write(quote_spanned!(span=>
                                #core::lexis::TokenCursor::token_ref(session, 0)))
                            .to_tokens(&mut body);
                    }

                    quote_spanned!(span=>
                        #core::lexis::TokenCursor::advance(session);
                    )
                    .to_tokens(&mut body)
                }

                Some(ident) => {
                    let variant =
                        expect_some!(input.variants.get(&ident), "Unresolved reference.",);

                    let descend = match variant.secondary.is_some() {
                        false => {
                            let index = expect_some!(
                                variant.index.as_ref(),
                                "Missing parsable variant index.",
                            );

                            quote_spanned!(span=> #core::syntax::SyntaxSession::descend(session, #index))
                        }

                        true => match &variant.parser {
                            None => {
                                let ident = variant.generated_parser_ident();

                                quote_spanned!(span=> {
                                    let node = #ident(session);

                                    #core::syntax::SyntaxSession::node(session, node)
                                })
                            }

                            Some(ident) => {
                                let this = input.this();

                                quote_spanned!(span=> {
                                    let node = #this::#ident(session);

                                    #core::syntax::SyntaxSession::node(session, node)
                                })
                            }
                        },
                    };

                    match action.capture {
                        None => quote_spanned!(span=> #descend;).to_tokens(&mut body),

                        Some(variable) => {
                            variables.get(&variable).write(descend).to_tokens(&mut body);
                        }
                    }
                }
            }

            if has_outgoing && !is_looping {
                quote_spanned!(span=> state = #to;).to_tokens(&mut body);
            }

            match !has_outgoing && is_final {
                true => quote_spanned!(span=> break;).to_tokens(&mut body),
                false => quote_spanned!(span=> continue;).to_tokens(&mut body),
            }

            match set.single() {
                Some(TokenLit::Ident(ident)) => {
                    let enum_variant = TokenLit::Ident(ident).as_enum_variant(&input.token);

                    quote_spanned!(span=> if token == #enum_variant {
                        #body
                    })
                    .to_tokens(&mut stream)
                }

                _ => {
                    let pattern = Self::make_pattern(input, globals, set).compile(span);

                    quote_spanned!(span=> if #core::lexis::TokenSet::contains(&#pattern, token as u8) {
                        #body
                    })
                    .to_tokens(&mut stream)
                }
            }
        }

        match is_final {
            true => {
                quote_spanned!(span=> break;).to_tokens(&mut stream);
            }

            false if !uncovered.is_empty() => {
                let recovery = recovery_var.compile(span);

                quote_spanned!(span=>
                    site = #core::lexis::TokenCursor::site_ref(session, 0);
                )
                .to_tokens(&mut stream);

                let delimiter_halt;

                match delimiter {
                    Some(delimiter) if !covered.contains(delimiter) => {
                        let _ = covered.insert(delimiter.clone());

                        delimiter_halt = Some(delimiter)
                    }

                    _ => delimiter_halt = None,
                }

                let expectations = Self::make_pattern(input, globals, covered).compile(span);

                quote_spanned!(span=>
                    let mut recovered = #core::syntax::Recovery::recover(
                        &#recovery,
                        session,
                        &#expectations,
                    );
                )
                .to_tokens(&mut stream);

                quote_spanned!(span=>
                    let end_site = #core::lexis::TokenCursor::site_ref(session, 0);

                    #core::syntax::SyntaxSession::error(
                        session,
                        #core::syntax::SyntaxError {
                            span: site..end_site,
                            context: #context,
                            expected_tokens: &#expected_tokens_var,
                            expected_rules: &#expected_rules_var,
                        }
                    );
                )
                .to_tokens(&mut stream);

                if let Some(delimiter) = delimiter_halt {
                    let delimiter = expect_some!(
                        delimiter.as_enum_variant(&input.token),
                        "Non-ident delimiter.",
                    );

                    quote_spanned!(span=>
                        if recovered {
                            if #core::lexis::TokenCursor::token(session, 0) == #option::Some(#delimiter) {
                                #core::lexis::TokenCursor::advance(session);
                                recovered = false;
                            }
                        }
                    )
                    .to_tokens(&mut stream);
                }

                quote_spanned!(span=>
                    if !recovered {
                        break
                    }
                )
                .to_tokens(&mut stream);
            }

            _ => (),
        }

        stream
    }

    fn make_pattern(input: &NodeInput, globals: &mut Globals, mut set: Set<TokenLit>) -> GlobalVar {
        let mut exclusive = false;

        set.retain(|lit| match lit {
            TokenLit::Ident(..) => true,
            TokenLit::Other(..) => {
                exclusive = true;
                false
            }
        });

        match exclusive {
            false => globals.inclusive_tokens(set.into_iter()),

            true => {
                let mut excluded = input.alphabet.clone();

                for lit in set {
                    let _ = excluded.remove(&lit);
                }

                globals.exclusive_tokens(excluded.into_iter())
            }
        }
    }
}

#[derive(Default)]
pub(super) struct Globals {
    recoveries: BTreeMap<Recovery, String>,
    rules: BTreeMap<BTreeSet<Index>, String>,
    tokens: BTreeMap<TokenSet, String>,
}

impl Globals {
    pub(super) fn compile(&self, span: Span, token_type: &Type) -> TokenStream {
        #[inline(always)]
        fn compare_keys(a: &str, b: &str) -> Ordering {
            let ordering = a.len().cmp(&b.len());

            if ordering.is_eq() {
                return a.cmp(&b);
            }

            ordering
        }

        let mut stream = TokenStream::new();

        let core = span.face_core();

        let mut recoveries = self.recoveries.iter().collect::<Vec<_>>();
        recoveries.sort_by(|(_, a), (_, b)| compare_keys(a, b));

        let mut rules = self.rules.iter().collect::<Vec<_>>();
        rules.sort_by(|(_, a), (_, b)| compare_keys(a, b));

        let mut tokens = self.tokens.iter().collect::<Vec<_>>();
        tokens.sort_by(|(_, a), (_, b)| compare_keys(a, b));

        for (recovery, ident) in recoveries {
            let recovery = recovery.compile(token_type);
            let ident = Ident::new(ident, span);

            quote_spanned!(span=> static #ident: #core::syntax::Recovery = #recovery;)
                .to_tokens(&mut stream);
        }

        for (rules, ident) in rules {
            let ident = Ident::new(ident, span);

            quote_spanned!(span=>
                static #ident: #core::syntax::RuleSet =
                    #core::syntax::RuleSet::new(&[#(#rules),*]);
            )
            .to_tokens(&mut stream);
        }

        for (tokens, ident) in tokens {
            let ident = Ident::new(ident, span);

            match tokens {
                TokenSet::Exclusive(lits) => {
                    let set = lits.into_iter().map(|lit| {
                        expect_some!(lit.as_enum_variant(token_type), "Non-ident token.",)
                    });

                    quote_spanned!(span=>
                        static #ident: #core::lexis::TokenSet
                            = #core::lexis::TokenSet::exclusive(&[#(#set as u8),*]);
                    )
                    .to_tokens(&mut stream);
                }

                TokenSet::Inclusive(lits) => {
                    let set = lits.into_iter().map(|lit| {
                        expect_some!(lit.as_enum_variant(token_type), "Non-ident token.",)
                    });

                    quote_spanned!(span=>
                        static #ident: #core::lexis::TokenSet
                            = #core::lexis::TokenSet::inclusive(&[#(#set as u8),*]);
                    )
                    .to_tokens(&mut stream);
                }
            }
        }

        stream
    }

    pub(super) fn recovery(&mut self, recovery: Recovery) -> GlobalVar {
        if recovery.is_empty() {
            return GlobalVar::UnlimitedRecovery;
        }

        if let Some(ident) = self.recoveries.get(&recovery) {
            return GlobalVar::Static(ident.clone());
        }

        let ident = format!("RECOVERY_{}", self.recoveries.len() + 1);

        let _ = self.recoveries.insert(recovery, ident.clone());

        GlobalVar::Static(ident.clone())
    }

    pub(super) fn rules(&mut self, set: impl Iterator<Item = Index>) -> GlobalVar {
        let set = set.collect::<BTreeSet<_>>();

        if set.is_empty() {
            return GlobalVar::EmptyRuleSet;
        }

        if let Some(ident) = self.rules.get(&set) {
            return GlobalVar::Static(ident.clone());
        }

        let ident = format!("RULES_{}", self.rules.len() + 1);

        let _ = self.rules.insert(set, ident.clone());

        GlobalVar::Static(ident.clone())
    }

    pub(super) fn inclusive_tokens(&mut self, set: impl Iterator<Item = TokenLit>) -> GlobalVar {
        let set = set.collect::<BTreeSet<_>>();

        if set.is_empty() {
            return GlobalVar::EmptyTokenSet;
        }

        self.tokens(TokenSet::Inclusive(set))
    }

    pub(super) fn exclusive_tokens(&mut self, set: impl Iterator<Item = TokenLit>) -> GlobalVar {
        let set = set.collect::<BTreeSet<_>>();

        if set.is_empty() {
            return GlobalVar::FullTokenSet;
        }

        self.tokens(TokenSet::Exclusive(set))
    }

    fn tokens(&mut self, set: TokenSet) -> GlobalVar {
        if set.is_empty() {
            return GlobalVar::EmptyTokenSet;
        }

        if let Some(ident) = self.tokens.get(&set) {
            return GlobalVar::Static(ident.clone());
        }

        let ident = format!("TOKENS_{}", self.tokens.len() + 1);

        let _ = self.tokens.insert(set, ident.clone());

        GlobalVar::Static(ident.clone())
    }
}

pub(super) enum GlobalVar {
    Static(String),
    EmptyTokenSet,
    FullTokenSet,
    EmptyRuleSet,
    UnlimitedRecovery,
}

impl GlobalVar {
    #[inline]
    fn compile(&self, span: Span) -> TokenStream {
        match self {
            Self::Static(string) => Ident::new(string, span).to_token_stream(),

            Self::EmptyTokenSet => {
                let core = span.face_core();

                quote_spanned!(span=> #core::lexis::EMPTY_TOKEN_SET)
            }

            Self::FullTokenSet => {
                let core = span.face_core();

                quote_spanned!(span=> #core::lexis::FULL_TOKEN_SET)
            }

            Self::EmptyRuleSet => {
                let core = span.face_core();

                quote_spanned!(span=> #core::syntax::EMPTY_RULE_SET)
            }

            Self::UnlimitedRecovery => {
                let core = span.face_core();

                quote_spanned!(span=> #core::syntax::UNLIMITED_RECOVERY)
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TokenSet {
    Inclusive(BTreeSet<TokenLit>),
    Exclusive(BTreeSet<TokenLit>),
}

impl TokenSet {
    #[inline(always)]
    fn is_empty(&self) -> bool {
        match self {
            Self::Inclusive(set) => set.is_empty(),
            Self::Exclusive(set) => set.is_empty(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Action {
    transition: State,
    insert: Option<Terminal>,
    descend: Option<Ident>,
    capture: Option<Ident>,
}
