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
    fs::{read_to_string, write},
    ops::Range,
    time::Duration,
};

use criterion::{BenchmarkId, Criterion};
use dirs::cache_dir;
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::{BenchData, FrameworkCase, SourceSample};

const LAYERS: [Range<usize>; 3] = [1500..2000, 60000..80000, 160000..200000];
const SHORT_EDITS: usize = 1000;
const LONG_EDITS: usize = 100;

#[derive(Serialize, Deserialize)]
pub struct BenchDataLayer {
    pub seed: u64,
    pub index: usize,
    pub load: BenchData,
    pub short_edits: BenchData,
    pub long_edits: BenchData,
    pub many_edits: BenchData,
}

impl BenchDataLayer {
    pub fn cached(seed: u64) -> Vec<Self> {
        println!("Loading data for seed {seed} from cache...");

        let mut path = match cache_dir() {
            Some(path) => path,
            None => {
                println!(
                    "Cache directory is not available.\nThe test data will be \
                    regenerated."
                );
                return Self::generate(seed);
            }
        };

        path.push(format!(".ld-bench-data-{seed}.json"));

        if path.is_dir() {
            println!("{path:?} is a directory.\nThe test data will be regenerated.",);
            return Self::generate(seed);
        }

        let mut save = false;
        let deserialized;

        loop {
            if !path.exists() {
                println!(
                    "{path:?} file does not exist.\nThe test data will be \
                regenerated and saved to this file.",
                );

                save = true;
                deserialized = Self::generate(seed);
                break;
            }

            match read_to_string(&path) {
                Ok(string) => {
                    deserialized = match serde_json::from_str::<Vec<Self>>(string.as_str()) {
                        Ok(data) => data,

                        Err(error) => {
                            println!(
                                "{path:?} deserialization error: {error}.\nThe \
                            test data will be regenerated and saved to \
                            this file.",
                            );

                            save = true;
                            Self::generate(seed)
                        }
                    };
                    break;
                }
                Err(error) => {
                    println!(
                        "{path:?} read error: {error}.\nThe test data will be \
                    regenerated and saved to this file.",
                    );

                    save = true;
                    deserialized = Self::generate(seed);
                    break;
                }
            };
        }

        match save {
            true => {
                let serialized = match serde_json::to_string(&deserialized) {
                    Ok(data) => {
                        println!("The test data serialized successfully.");
                        data
                    }

                    Err(error) => {
                        println!("Test data serialization failure: {error}.");
                        return deserialized;
                    }
                };

                match write(&path, serialized) {
                    Ok(()) => {
                        println!("The test data successfully saved to file: {path:?}.");
                    }

                    Err(error) => {
                        println!("File {path:?} save error: {error}.");
                    }
                }
            }

            false => {
                println!(
                    "{path:?} cached data for the seed {seed} loaded and \
                    deserialized successfully.",
                );
            }
        }

        deserialized
    }

    pub fn generate(seed: u64) -> Vec<Self> {
        println!("Preparing test data for seed {seed}...");

        let mut random = StdRng::seed_from_u64(seed);
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
                seed,
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
                        let mut random = StdRng::seed_from_u64(self.seed);

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
                        let mut random = StdRng::seed_from_u64(self.seed);

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
