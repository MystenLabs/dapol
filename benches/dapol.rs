extern crate nalgebra as na;

use std::mem;
use std::path::PathBuf;

use criterion::{criterion_group, criterion_main};
use criterion::{BenchmarkId, Criterion, SamplingMode};
use gnuplot::Figure;
use jemalloc_ctl::{epoch, stats};
use rand::distributions::{Distribution, Uniform};

use dapol::accumulators::NdmSmt;
use dapol::{EntityId, Height, InclusionProof, MaxThreadCount};

mod data;
mod setup;

// CONSTANTS
// ================================================================================================

const TREE_HEIGHTS: [u8; 3] = [16, 32, 64];
const NUM_USERS: [u64; 35] = [
    10_000,
    20_000,
    30_000,
    40_000,
    50_000,
    60_000,
    70_000,
    80_000,
    90_000,
    100_000,
    200_000,
    300_000,
    400_000,
    500_000,
    600_000,
    700_000,
    800_000,
    900_000,
    1_000_000,
    2_000_000,
    3_000_000,
    4_000_000,
    5_000_000,
    6_000_000,
    7_000_000,
    8_000_000,
    9_000_000,
    10_000_000,
    30_000_000,
    50_000_000,
    70_000_000,
    90_000_000,
    100_000_000,
    125_000_000,
    250_000_000,
];

const HASH_SIZE_BYTES: usize = 32;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

// BENCHMARKS
// ================================================================================================

fn bench_build_tree(c: &mut Criterion) {
    let e = epoch::mib().unwrap();
    let alloc = stats::allocated::mib().unwrap();

    let mut group = c.benchmark_group("dapol");
    group.sample_size(20);
    // `SamplingMode::Flat` is used here as that is what Criterion recommends for long-running benches
    // https://bheisler.github.io/criterion.rs/book/user_guide/advanced_configuration.html#sampling-mode
    group.sampling_mode(SamplingMode::Flat);

    dapol::initialize_machine_parallelism();

    let thread_counts = {
        let mut tc: Vec<u8> = Vec::new();

        let max_thread_count: u8 = MaxThreadCount::default().get_value();

        let step = if max_thread_count < 8 {
            1
        } else {
            max_thread_count >> 2
        };

        for i in (step..max_thread_count).step_by(step as usize) {
            tc.push(i);
        }

        tc.push(max_thread_count);

        tc
    };

    for h in TREE_HEIGHTS.into_iter() {
        for t in thread_counts.iter() {
            for u in NUM_USERS.into_iter() {
                let mut ndm_smt = Option::<NdmSmt>::None;

                // Many of the statistics tracked by `jemalloc` are cached.
                // The epoch controls when they are refreshed.
                // We care about measuring ndm_smt so we refresh before it's construction
                e.advance().unwrap();
                let before = alloc.read().unwrap();

                let max_users_for_height = 2_u64.pow((h - 1) as u32);

                if u > max_users_for_height {
                    break;
                }

                let tup: (Height, MaxThreadCount, u64) =
                    (Height::from(h), MaxThreadCount::from(*t), u);

                // tree build compute time
                group.bench_with_input(
                    BenchmarkId::new(
                        "build_tree",
                        format!("{:?}/{:?}/NUM_USERS: {:?}", tup.0, tup.1, tup.2),
                    ),
                    &tup,
                    |bench, tup| {
                        bench.iter(|| {
                            ndm_smt = Some(crate::setup::build_ndm_smt(tup.clone()));
                        });
                    },
                );

                e.advance().unwrap();
                let after = alloc.read().unwrap();

                // mem used is the difference between the 2 measurements
                let diff = after - before;

                // tree build file size
                let tree_build_file_size = setup::serialize_tree(
                    &ndm_smt.as_ref().expect("Tree not found"),
                    PathBuf::from("./target"),
                );

                println!(
                    "\n Metrics {{ variable: \"TreeBuild\", mem_usage: {}, file_size: {} }} \n",
                    setup::bytes_as_string(diff),
                    tree_build_file_size
                );
            }
        }
    }

    group.finish()
}

fn bench_generate_proof(c: &mut Criterion) {
    let mut group = c.benchmark_group("dapol");
    group.sample_size(20);
    group.sampling_mode(SamplingMode::Auto);

    dapol::initialize_machine_parallelism();

    let thread_counts = {
        let mut tc: Vec<u8> = Vec::new();

        let max_thread_count: u8 = MaxThreadCount::default().get_value();

        let step = if max_thread_count < 8 {
            1
        } else {
            max_thread_count >> 2
        };

        for i in (step..max_thread_count).step_by(step as usize) {
            tc.push(i);
        }

        tc.push(max_thread_count);

        tc
    };

    let mut rng = rand::thread_rng();

    for h in TREE_HEIGHTS.into_iter() {
        for t in thread_counts.iter() {
            for u in NUM_USERS.into_iter() {
                let mut proof = Option::<InclusionProof>::None;

                let max_users_for_height = 2_u64.pow((h - 1) as u32);

                if u > max_users_for_height {
                    break;
                }

                let tup: (Height, MaxThreadCount, u64) =
                    (Height::from(h), MaxThreadCount::from(*t), u);

                let ndm_smt = Some(setup::build_ndm_smt(tup.clone())).expect("Tree not found");

                let entity_ids: Vec<&EntityId> = ndm_smt.entity_mapping().keys().collect();

                let i = Uniform::from(0..NUM_USERS.len() - 1);

                // proof generation compute time
                group.bench_with_input(
                    BenchmarkId::new(
                        "generate_proof",
                        format!("{:?}/{:?}/NUM_USERS: {:?}", tup.0, tup.1, tup.2),
                    ),
                    &ndm_smt,
                    |bench, ndm_smt| {
                        bench.iter(|| {
                            proof = Some(setup::generate_proof(
                                ndm_smt,
                                entity_ids[i.sample(&mut rng)],
                            ));
                        });
                    },
                );

                // proof file size
                let proof_file_size = setup::serialize_proof(
                    proof.as_ref().expect("Proof not found"),
                    &entity_ids[0],
                    PathBuf::from("./target"),
                );

                println!(
                    "\n Metrics {{ variable: \"ProofGeneration\", file_size: {} }} \n",
                    proof_file_size
                );
            }
        }
    }

    group.finish()
}

fn bench_verify_proof(c: &mut Criterion) {
    let mut group = c.benchmark_group("dapol");
    group.sample_size(20);
    group.sampling_mode(SamplingMode::Auto);

    dapol::initialize_machine_parallelism();

    let thread_counts = {
        let mut tc: Vec<u8> = Vec::new();

        let max_thread_count: u8 = MaxThreadCount::default().get_value();

        let step = if max_thread_count < 8 {
            1
        } else {
            max_thread_count >> 2
        };

        for i in (step..max_thread_count).step_by(step as usize) {
            tc.push(i);
        }

        tc.push(max_thread_count);

        tc
    };

    let mut rng = rand::thread_rng();

    for h in TREE_HEIGHTS.into_iter() {
        for t in thread_counts.iter() {
            for u in NUM_USERS.into_iter() {
                let max_users_for_height = 2_u64.pow((h - 1) as u32);

                if u > max_users_for_height {
                    break;
                }

                let tup: (Height, MaxThreadCount, u64) =
                    (Height::from(h), MaxThreadCount::from(*t), u);

                let ndm_smt = Some(setup::build_ndm_smt(tup.clone())).expect("Tree not found");

                let i = Uniform::from(0..NUM_USERS.len() - 1);

                let entity_ids: Vec<&EntityId> = ndm_smt.entity_mapping().keys().collect();

                let proof = Some(setup::generate_proof(
                    &ndm_smt,
                    entity_ids[i.sample(&mut rng)],
                ))
                .expect("Proof not found");

                // proof file size
                let proof_file_size =
                    setup::serialize_proof(&proof, &entity_ids[0], PathBuf::from("./target"));

                // proof verification compute time
                group.bench_with_input(
                    BenchmarkId::new(
                        "verify_proof",
                        format!("{:?}/{:?}/NUM_USERS: {:?}", tup.0, tup.1, tup.2),
                    ),
                    &proof,
                    |bench, proof| {
                        bench.iter(|| {
                            InclusionProof::verify(proof, ndm_smt.root_hash())
                                .expect("Unable to verify proof")
                        });
                    },
                );

                println!(
                    "\n Metrics {{ variable: \"ProofVerification\", file_size: {} }} \n",
                    proof_file_size
                );
            }
        }
    }

    group.finish()
}

fn bench_test_jemalloc_readings() {
    let e = epoch::mib().unwrap();
    let alloc = stats::allocated::mib().unwrap();

    e.advance().unwrap();
    let before = alloc.read().unwrap();

    // 1 MB
    let buf: Vec<u8> = Vec::with_capacity(1024u32.pow(2) as usize);

    e.advance().unwrap();
    let after = alloc.read().unwrap();

    let diff = after - before;

    println!(
        "buf capacity: {:<6}",
        setup::bytes_as_string(buf.capacity())
    );

    println!("Memory usage: {} allocated", setup::bytes_as_string(diff),);
}

// HEURISTICS
// ================================================================================================

// Heuristic function to estimate memory usage for a Merkle Tree
fn estimate_memory_usage(height: u8, num_users: u64) -> usize {
    // Calculate the number of hash values in the Merkle Tree
    let num_hash_values = 2u32.pow(height as u32);

    // Calculate the total memory usage
    let memory_usage_bytes =
        num_users as usize * HASH_SIZE_BYTES + num_hash_values as usize * mem::size_of::<u8>();

    memory_usage_bytes
}

fn plot_heuristic_function() {
    let mut values: Vec<usize> = Vec::with_capacity(TREE_HEIGHTS.len() * NUM_USERS.len());

    for h in TREE_HEIGHTS.into_iter() {
        for u in NUM_USERS.into_iter() {
            let estimate = estimate_memory_usage(h, u);
            values.push(estimate);
        }
    }

    let mut fg = Figure::new();

    fg.set_title("Estimated memory usage").axes3d().surface(
        values.iter(),
        TREE_HEIGHTS.len(),
        NUM_USERS.len(),
        None,
        &[],
    );

    fg.show().unwrap();
}

// MACROS
// ================================================================================================

criterion_group!(
    benches,
    bench_build_tree,
    bench_generate_proof,
    bench_verify_proof
);

criterion_main!(
    benches,
    bench_test_jemalloc_readings,
    plot_heuristic_function,
    data::plot_data_points
);
