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

#![doc = include_str!("../readme.md")]
//TODO check warnings regularly
#![allow(warnings)]

#[macro_use]
extern crate quote;

#[macro_use]
extern crate syn;

extern crate core;
extern crate proc_macro;

mod node;
mod token;
mod utils;

const BENCHMARK: bool = false;

#[doc = include_str!("./token/readme.md")]
#[proc_macro_derive(Token, attributes(define, rule, precedence, constructor, mismatch))]
pub fn token(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // panic!(
    //     "{}",
    //     proc_macro::TokenStream::from(parse_macro_input!(input as token::Token))
    // );

    parse_macro_input!(input as token::Token).into()

    // (quote! {}).into()
}

#[doc = include_str!("./node/readme.md")]
#[proc_macro_derive(
    Node,
    attributes(
        token,
        error,
        skip,
        define,
        rule,
        root,
        comment,
        synchronization,
        index,
        constructor,
        secondary,
        default,
    )
)]
pub fn node(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // panic!(
    //     "{}",
    //     proc_macro::TokenStream::from(parse_macro_input!(input as node::Node))
    // );

    parse_macro_input!(input as node::Node).into()

    // (quote! {}).into()
}
