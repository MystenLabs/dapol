//! Benchmarks using Criterion.
//!
//! Criterion has a minimum of 10 samples that it takes for a benchmark, which
//! can be an obstacle if each bench takes multiple hours to run. So there is
//! a) an env var `MAX_ENTITIES_FOR_CRITERION_BENCHES` to change how many
//! benches are run using Criterion,
//! b) a different framework that is used to benchmark the runs that take really
//! long.

use std::path::Path;

use criterion::measurement::Measurement;
use criterion::{criterion_group, criterion_main};
use criterion::{BenchmarkId, Criterion, SamplingMode};
use once_cell::sync::Lazy;
use statistical::*;

use dapol::accumulators::{NdmSmt, NdmSmtConfigBuilder};
use dapol::{Accumulator, initialize_machine_parallelism};

mod inputs;
use inputs::{max_thread_counts, num_entities_less_than_eq, tree_heights};

mod memory_usage_estimation;
use memory_usage_estimation::estimated_total_memory_usage_mb;

mod utils;
use utils::{abs_diff, bytes_as_string, system_total_memory_mb};

/// Determines how many runs are done for number of entities.
/// The higher this value the more runs that are done.
///
/// Some of the tree builds can take a few hours, and Criterion does a minimum
/// of 10 samples per bench. So this value gives us to decide how much of the
/// num_entities
static MAX_ENTITIES_FOR_CRITERION_BENCHES: Lazy<u64> = Lazy::new(|| {
    std::env::var("MAX_ENTITIES_FOR_CRITERION")
        .unwrap_or("100000".to_string())
        .parse()
        .unwrap()
});

/// This is required to get jemalloc_ctl to work properly.
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

// -------------------------------------------------------------------------------------------------
// Benchmarks

pub fn bench_build_tree<T: Measurement>(c: &mut Criterion<T>) {
    let epoch = jemalloc_ctl::epoch::mib().unwrap();
    let allocated = jemalloc_ctl::stats::allocated::mib().unwrap();

    initialize_machine_parallelism();

    let mut group = c.benchmark_group("build_tree");
    // `SamplingMode::Flat` is used here as that is what Criterion recommends for long-running benches
    // https://bheisler.github.io/criterion.rs/book/user_guide/advanced_configuration.html#sampling-mode
    group.sampling_mode(SamplingMode::Flat);

    for h in tree_heights().iter() {
        for t in max_thread_counts().iter() {
            for n in num_entities_less_than_eq(*MAX_ENTITIES_FOR_CRITERION_BENCHES).iter() {
                println!("=============================================================\n");

                // =============================================================
                // Input validation.

                {
                    // We attempt to guess the amount of memory that the tree
                    // build will require, and if that is greater than the
                    // amount of memory available on the machine then we skip
                    // the input tuple.

                    let total_mem = system_total_memory_mb();
                    let expected_mem = estimated_total_memory_usage_mb(h, n);

                    if total_mem < expected_mem {
                        println!(
                            "Skipping input height_{}/num_entities_{} since estimated memory \
                                  usage {} is greater than the system max {}",
                            h.as_u32(),
                            n,
                            expected_mem,
                            total_mem
                        );

                        continue;
                    }
                }

                // Do not try build the tree if the number of entities exceeds
                // the maximum number allowed. If this check is not done then
                // we would get an error on tree build.
                if n > &h.max_bottom_layer_nodes() {
                    println!(
                        "Skipping input height_{}/num_entities_{} since number of entities is \
                              greater than max allowed",
                        h.as_u32(),
                        n
                    );

                    continue;
                }

                // =============================================================
                // Tree build.

                let mut memory_readings = vec![];
                let mut ndm_smt = Option::<NdmSmt>::None;

                group.bench_with_input(
                    BenchmarkId::new(
                        "build_tree",
                        format!(
                            "height_{}/max_thread_count_{}/num_entities_{}",
                            h.as_u32(),
                            t.get_value(),
                            n
                        ),
                    ),
                    &(h, t, n),
                    |bench, tup| {
                        bench.iter(|| {
                            // this is necessary for the memory readings to work
                            ndm_smt = None;

                            epoch.advance().unwrap();
                            let before = allocated.read().unwrap();

                            ndm_smt = Some(
                                NdmSmtConfigBuilder::default()
                                    .height(tup.0.clone())
                                    .max_thread_count(tup.1.clone())
                                    .num_random_entities(*tup.2)
                                    .build()
                                    .parse()
                                    .expect("Unable to parse NdmSmtConfig"),
                            );

                            epoch.advance().unwrap();
                            memory_readings
                                .push(abs_diff(allocated.read().unwrap(), before) as f64);
                        });
                    },
                );

                memory_readings = memory_readings
                    .into_iter()
                    .map(|m| m / 1024u64.pow(2) as f64)
                    .collect();

                let mean = mean(&memory_readings);
                println!(
                    "\nMemory usage (MB): {:.2} +/- {:.4} ({:.2})\n",
                    mean,
                    standard_deviation(&memory_readings, Some(mean)),
                    median(&memory_readings)
                );

                // =============================================================
                // Tree serialization.

                let src_dir = env!("CARGO_MANIFEST_DIR");
                let target_dir = Path::new(&src_dir).join("target");
                let dir = target_dir.join("serialized_trees");
                let path = Accumulator::parse_accumulator_serialization_path(dir).unwrap();
                let acc = Accumulator::NdmSmt(ndm_smt.expect("Tree should have been built"));

                group.bench_with_input(
                    BenchmarkId::new(
                        "serialize_tree",
                        format!(
                            "height_{}/max_thread_count_{}/num_entities_{}",
                            h.as_u32(),
                            t.get_value(),
                            n
                        ),
                    ),
                    &(h, t, n),
                    |bench, tup| {
                        bench.iter(|| acc.serialize(path.clone()));
                    },
                );

                let file_size = std::fs::metadata(path)
                    .expect("Unable to get serialized tree metadata for {path}")
                    .len();

                println!(
                    "\nSerialized tree file size: {}\n",
                    bytes_as_string(file_size as usize)
                );
            }
        }
    }
}

/// We only loop through `tree_heights` & `num_entities` because we want proof
/// generation to have maximum threads.
pub fn bench_generate_proof<T: Measurement>(c: &mut Criterion<T>) {
    let mut group = c.benchmark_group("generate_proof");
    group.sample_size(20);

    for h in tree_heights().iter() {
        for n in num_entities_less_than_eq(*MAX_ENTITIES_FOR_CRITERION_BENCHES).iter() {
            // TODO continue if the memory heuristic check fails

            if n > &h.max_bottom_layer_nodes() {
                println!("Skipping input height_{}/num_entities_{} since number of entities is greater than max allowed", h.as_u32(), n);
                continue;
            }

            let ndm_smt = NdmSmtConfigBuilder::default()
                .height(h.clone())
                .num_random_entities(*n)
                .build()
                .parse()
                .expect("Unable to parse NdmSmtConfig");

            let entity_id = ndm_smt
                .entity_mapping()
                .keys()
                .next()
                .expect("Tree should have at least 1 entity");

            group.bench_function(
                BenchmarkId::new(
                    "build_tree",
                    format!("height_{}/num_entities_{}", h.as_u32(), n),
                ),
                |bench| {
                    bench.iter(|| {
                        let _proof = ndm_smt
                            .generate_inclusion_proof(entity_id)
                            .expect("Proof should have been generated successfully");
                    });
                },
            );
        }
    }
}

/// We only loop through `tree_heights` & `num_entities` because proof
/// verification does not depend on number of threads.
pub fn bench_verify_proof<T: Measurement>(c: &mut Criterion<T>) {
    let mut group = c.benchmark_group("generate_proof");
    group.sample_size(20);

    for h in tree_heights().iter() {
        for n in num_entities_less_than_eq(*MAX_ENTITIES_FOR_CRITERION_BENCHES).iter() {
            // TODO continue if the memory heuristic check fails

            if n > &h.max_bottom_layer_nodes() {
                println!("Skipping input height_{}/num_entities_{} since number of entities is greater than max allowed", h.as_u32(), n);
                continue;
            }

            let ndm_smt = NdmSmtConfigBuilder::default()
                .height(h.clone())
                .num_random_entities(*n)
                .build()
                .parse()
                .expect("Unable to parse NdmSmtConfig");

            let root_hash = ndm_smt.root_hash();

            let entity_id = ndm_smt
                .entity_mapping()
                .keys()
                .next()
                .expect("Tree should have at least 1 entity");

            let proof = ndm_smt
                .generate_inclusion_proof(entity_id)
                .expect("Proof should have been generated successfully");

            group.bench_function(
                BenchmarkId::new(
                    "build_tree",
                    format!("height_{}/num_entities_{}", h.as_u32(), n),
                ),
                |bench| {
                    bench.iter(|| proof.verify(root_hash));
                },
            );
        }
    }
}

// -------------------------------------------------------------------------------------------------
// TODO move to another file

// pub fn bench_test_jemalloc_readings() {
//     use jemalloc_ctl::{epoch, stats};

//     let e = epoch::mib().unwrap();
//     let alloc = stats::allocated::mib().unwrap();

//     e.advance().unwrap();
//     let before = alloc.read().unwrap();

//     // 1 MB
//     let buf: Vec<u8> = Vec::with_capacity(1024u32.pow(2) as usize);

//     e.advance().unwrap();
//     let after = alloc.read().unwrap();

//     let diff = after - before;

//     println!(
//         "buf capacity: {:<6}",
//         setup::bytes_as_string(buf.capacity())
//     );

//     println!("Memory usage: {} allocated", setup::bytes_as_string(diff),);
// }

// -------------------------------------------------------------------------------------------------
// Macros.

use std::time::Duration;

criterion_group! {
    name = wall_clock_time;
    config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(60));
    targets = bench_build_tree, bench_generate_proof, bench_verify_proof,
}

// Does not work, see memory_measurement.rs
// mod memory_measurement;
// criterion_group! {
//     name = memory_usage;
//     config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(60)).with_measurement(memory_measurement::Memory);
//     targets = bench_build_tree, bench_generate_proof, bench_verify_proof,
// }

criterion_main!(wall_clock_time);
