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

pub mod nom;
pub mod ropey;
pub mod treesitter;

use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

use criterion::black_box;
use lady_deirdre::{compiler::MutableUnit, lexis::SiteSpan, syntax::Node};

use crate::BenchDataLayer;

pub trait FrameworkCase {
    fn name(&self) -> &'static str;

    #[allow(unused)]
    fn configuration(&self, layer: &BenchDataLayer) -> FrameworkConfiguration {
        FrameworkConfiguration {
            sample_size: match layer.index == 0 {
                false => 10,
                true => 100,
            },

            ..FrameworkConfiguration::default()
        }
    }

    fn bench_load(&self, text: &str) -> Duration;

    fn bench_single_edit<'a>(&self, text: &'a str, span: SiteSpan, edit: &'a str) -> Duration;

    fn bench_sequential_edits<'a>(
        &self,
        text: &'a str,
        edits: Vec<(SiteSpan, &'a str)>,
    ) -> Duration;
}

pub struct FrameworkConfiguration {
    pub sample_size: usize,
    pub data_load: bool,
    pub short_edits: bool,
    pub long_edits: bool,
    pub many_edits: bool,
}

impl Default for FrameworkConfiguration {
    fn default() -> Self {
        Self {
            data_load: true,
            short_edits: true,
            long_edits: true,
            many_edits: true,
            sample_size: 100,
        }
    }
}

pub struct SelfCase<Syntax: Node> {
    name: &'static str,
    syntax: PhantomData<Syntax>,
}

impl<Syntax: Node> FrameworkCase for SelfCase<Syntax> {
    fn name(&self) -> &'static str {
        self.name
    }

    #[inline(never)]
    fn bench_load(&self, text: &str) -> Duration {
        let start = Instant::now();
        let result = MutableUnit::<Syntax>::from(text);
        let time = start.elapsed();

        black_box(result);

        time
    }

    #[inline(never)]
    fn bench_single_edit<'a>(&self, text: &'a str, span: SiteSpan, edit: &'a str) -> Duration {
        let mut result = MutableUnit::<Syntax>::from(text);

        let start = Instant::now();
        result.write(span, edit);
        let time = start.elapsed();

        black_box(result);

        time
    }

    #[inline(never)]
    fn bench_sequential_edits<'a>(
        &self,
        text: &'a str,
        edits: Vec<(SiteSpan, &'a str)>,
    ) -> Duration {
        let mut result = MutableUnit::<Syntax>::from(text);

        let mut total = Duration::ZERO;

        for (span, edit) in edits {
            let start = Instant::now();
            result.write(span, edit);
            let time = start.elapsed();

            total += time;
        }

        black_box(result);

        total
    }
}

impl<Syntax: Node> SelfCase<Syntax> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            syntax: Default::default(),
        }
    }
}
