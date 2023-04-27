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

mod automata;
mod builder;
mod compiler;
mod regex;

use std::time::{Duration, Instant};

use syn::{
    parse::{Parse, ParseStream, Result},
    DeriveInput,
};

use crate::{
    node::{builder::Builder, compiler::Compiler},
    BENCHMARK,
};

pub struct Node {
    builder: Builder,
    build_time: Duration,
}

impl Parse for Node {
    #[inline(always)]
    fn parse(input: ParseStream) -> Result<Self> {
        let build_start = Instant::now();
        let builder = Builder::try_from(&input.parse::<DeriveInput>()?)?;

        Ok(Self {
            builder,
            build_time: build_start.elapsed(),
        })
    }
}

impl From<Node> for proc_macro::TokenStream {
    #[inline(always)]
    fn from(node: Node) -> Self {
        let name = node.builder.node_name().clone();
        let compile_start = Instant::now();
        let result = Compiler::compile(&node.builder).into();
        let compile_time = compile_start.elapsed();

        if BENCHMARK {
            println!(
                "Node {} compile time: {:?}",
                name,
                compile_time + node.build_time
            )
        }

        result
    }
}
