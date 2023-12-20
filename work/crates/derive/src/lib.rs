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

//todo consider replacing HashMap with AHashMap

#![doc = include_str!("../readme.md")]
//TODO check warnings regularly
#![allow(warnings)]

#[macro_use]
extern crate quote;

#[macro_use]
extern crate syn;

extern crate core;
extern crate proc_macro;

use std::str::FromStr;

use proc_macro2::TokenStream;
use quote::ToTokens;

use crate::{feature::FeatureInput, node::NodeInput, token::TokenInput, utils::system_panic};

mod feature;
mod node;
mod token;
mod utils;

#[doc = include_str!("./token/readme.md")]
#[proc_macro_derive(
    Token,
    attributes(define, rule, priority, constructor, blank, describe, opt, dump)
)]
pub fn token(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as TokenInput);

    let declarative = input.dump.is_declarative();

    output_stream(declarative, input.into_token_stream())
}

#[doc = include_str!("./node/readme.md")]
#[proc_macro_derive(
    Node,
    attributes(
        token,
        error,
        define,
        trivia,
        recovery,
        rule,
        root,
        index,
        constructor,
        secondary,
        parser,
        default,
        node,
        parent,
        child,
        semantics,
        describe,
        scope,
        dump,
    )
)]
pub fn node(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as NodeInput);

    let declarative = input.dump.is_declarative();

    output_stream(declarative, input.into_token_stream())
}

// todo link documentation
#[proc_macro_derive(Feature, attributes(node, invalidate, scope, dump))]
pub fn feature(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as FeatureInput);

    let declarative = input.dump.is_declarative();

    output_stream(declarative, input.into_token_stream())
}

fn output_stream(declarative: bool, stream: TokenStream) -> proc_macro::TokenStream {
    match declarative {
        true => match TokenStream::from_str(&stream.to_string()) {
            Ok(stream) => stream.into(),
            Err(error) => system_panic!("Spans erase failure. {error}",),
        },
        false => stream.into(),
    }
}
