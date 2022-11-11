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

use std::time::{Duration, Instant};

use criterion::black_box;
use lady_deirdre::lexis::SiteSpan;
use ropey::Rope;

use crate::FrameworkCase;

pub struct RopeyCase(pub &'static str);

impl FrameworkCase for RopeyCase {
    fn name(&self) -> &'static str {
        self.0
    }

    #[inline(never)]
    fn bench_load(&self, text: &str) -> Duration {
        let start = Instant::now();
        let result = Rope::from_str(text);
        let time = start.elapsed();

        black_box(result);

        time
    }

    #[inline(never)]
    fn bench_single_edit<'a>(&self, text: &'a str, span: SiteSpan, edit: &'a str) -> Duration {
        let mut result = Rope::from_str(text);

        let start = Instant::now();
        result.remove(span.clone());
        result.insert(span.start, edit);
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
        let mut result = Rope::from_str(text);

        let mut total = Duration::ZERO;

        for (span, edit) in edits {
            let start = Instant::now();
            result.remove(span.clone());
            result.insert(span.start, edit);
            let time = start.elapsed();

            total += time;
        }

        black_box(result);

        total
    }
}
