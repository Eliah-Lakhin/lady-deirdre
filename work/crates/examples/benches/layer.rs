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

use std::{ops::Range, time::Duration};

use criterion::{BenchmarkId, Criterion};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};

use crate::{BenchData, FrameworkCase, SourceSample};

const SEED: u64 = 154656;
static LAYERS: [Range<usize>; 3] = [1500..2000, 60000..80000, 160000..200000];
static SHORT_EDITS: usize = 1000;
static LONG_EDITS: usize = 100;

pub struct BenchDataLayer {
    pub index: usize,
    pub load: BenchData,
    pub short_edits: BenchData,
    pub long_edits: BenchData,
    pub many_edits: BenchData,
}

impl BenchDataLayer {
    pub fn new() -> Vec<Self> {
        println!("Preparing test data...");

        let mut random = StdRng::seed_from_u64(SEED);
        let mut layers = Vec::new();

        let mut short_edits = Vec::with_capacity(50);

        for _ in 0..SHORT_EDITS {
            short_edits.push(SourceSample::gen_short(&mut random, 50));
        }

        println!("Short edits ready.");

        let mut long_edits = Vec::with_capacity(LONG_EDITS);

        for _ in 0..LONG_EDITS {
            long_edits.push(SourceSample::gen_long(&mut random, 10..100));
        }

        println!("Long edits ready.");

        let mut many_edits = Vec::with_capacity(short_edits.len() + long_edits.len());
        many_edits.append(
            &mut short_edits
                .iter()
                .map(|edit| (1, edit))
                .collect::<Vec<_>>()
                .clone(),
        );
        many_edits.append(
            &mut long_edits
                .iter()
                .map(|edit| (2, edit))
                .collect::<Vec<_>>()
                .clone(),
        );
        many_edits.shuffle(&mut random);

        println!("Sequential edits ready.");

        for (index, layer) in LAYERS.iter().enumerate() {
            let load_data = BenchData::new(SourceSample::gen_init(&mut random, layer.clone()));

            let mut short_edits_data = load_data.clone();

            for edit in &short_edits {
                short_edits_data.edit_short(&mut random, edit.clone());
                short_edits_data.reset();
            }

            let mut long_edits_data = load_data.clone();

            for edit in &long_edits {
                long_edits_data.edit_long(&mut random, edit.clone());
                long_edits_data.reset();
            }

            let mut many_edits_data = load_data.clone();

            for (kind, edit) in many_edits.clone() {
                match kind {
                    1 => many_edits_data.edit_short(&mut random, edit.clone()),
                    2 => many_edits_data.edit_long(&mut random, edit.clone()),
                    _ => unreachable!(),
                }
            }

            println!("Layer {} complete.", load_data.describe_init());

            layers.push(BenchDataLayer {
                index,
                load: load_data,
                short_edits: short_edits_data,
                long_edits: long_edits_data,
                many_edits: many_edits_data,
            });
        }

        println!("Verifying test data...");

        for layer in &layers {
            // layer.load.verify_independent();
            // println!("Layer {} load data OK.", layer.load.describe_init());

            // layer.short_edits.verify_independent();
            // println!("Layer {} short edits data OK.", layer.load.describe_init());

            // layer.long_edits.verify_independent();
            // println!("Layer {} long edits data OK.", layer.load.describe_init());

            layer.many_edits.verify_sequential();
            println!("Layer {} many edits data OK.", layer.load.describe_init(),);
        }

        println!("Test data ready.");

        layers
    }

    pub fn run(&self, criterion: &mut Criterion, frameworks: &[Box<dyn FrameworkCase>]) {
        let mut group = criterion.benchmark_group(self.load.describe_init());

        for framework in frameworks {
            let configuration = framework.configuration(self);

            group.sample_size(configuration.sample_size);

            if configuration.data_load {
                group.bench_with_input(
                    BenchmarkId::new("Data Load", framework.name()),
                    &self.load,
                    |bencher, sample| {
                        bencher.iter_custom(|iterations| {
                            let mut total = Duration::ZERO;

                            for _ in 0..iterations {
                                total += framework.bench_load(&sample.init.source);
                            }

                            total
                        })
                    },
                );
            }

            if configuration.short_edits {
                group.bench_with_input(
                    BenchmarkId::new(
                        format!("Short edits {}", self.short_edits.describe_average_edit()),
                        framework.name(),
                    ),
                    &self.short_edits,
                    |bencher, sample| {
                        let init = sample.init.source.as_str();
                        let mut random = StdRng::seed_from_u64(SEED);

                        bencher.iter_custom(|iterations| {
                            let mut total = Duration::ZERO;

                            for _ in 0..iterations {
                                let step = &sample.steps[random.gen_range(0..sample.steps.len())];

                                total += framework.bench_single_edit(
                                    init,
                                    step.span.clone(),
                                    step.source.as_str(),
                                );
                            }

                            total
                        })
                    },
                );
            }

            if configuration.long_edits {
                group.bench_with_input(
                    BenchmarkId::new(
                        format!("Long edits {}", self.long_edits.describe_average_edit()),
                        framework.name(),
                    ),
                    &self.long_edits,
                    |bencher, sample| {
                        let init = sample.init.source.as_str();
                        let mut random = StdRng::seed_from_u64(SEED);

                        bencher.iter_custom(|iterations| {
                            let mut total = Duration::ZERO;

                            for _ in 0..iterations {
                                let step =
                                    sample.steps[random.gen_range(0..sample.steps.len())].clone();

                                total += framework.bench_single_edit(
                                    init,
                                    step.span.clone(),
                                    step.source.as_str(),
                                );
                            }

                            total
                        })
                    },
                );
            }

            if configuration.many_edits {
                group.bench_with_input(
                    BenchmarkId::new(
                        format!("Many edits {}", self.many_edits.describe_total_edits(),),
                        framework.name(),
                    ),
                    &self.many_edits,
                    |bencher, sample| {
                        let init = sample.init.source.as_str();

                        bencher.iter_custom(|iterations| {
                            let mut total = Duration::ZERO;

                            for _ in 0..iterations {
                                total += framework.bench_sequential_edits(
                                    init,
                                    sample
                                        .steps
                                        .iter()
                                        .map(|sample| (sample.span.clone(), sample.source.as_str()))
                                        .collect::<Vec<_>>(),
                                );
                            }

                            total
                        })
                    },
                );
            }
        }

        group.finish();
    }
}
