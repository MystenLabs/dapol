use std::path::PathBuf;

use criterion::{criterion_group, criterion_main};
use criterion::{BenchmarkId, Criterion, SamplingMode};
use jemalloc_ctl::{epoch, stats};

use dapol::accumulators::NdmSmt;
use dapol::{EntityId, Height, InclusionProof, MaxThreadCount};

mod setup;
use crate::setup::{Metrics, Variable, NUM_USERS, TREE_HEIGHTS};

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn bench_dapol(c: &mut Criterion) {
    let e = epoch::mib().unwrap();
    let alloc = stats::allocated::mib().unwrap();

    let mut group = c.benchmark_group("dapol");
    group.sample_size(10);

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

    let mut ndm_smt = Option::<NdmSmt>::None;

    e.advance().unwrap();

    for h in TREE_HEIGHTS.into_iter() {
        for t in thread_counts.iter() {
            for u in NUM_USERS.into_iter() {
                // Many of the statistics tracked by jemalloc are cached. The epoch controls when they are refreshed. We care about measuring ndm_smt so we refresh before it's construction
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
                            ndm_smt = Some(setup::build_ndm_smt(tup.clone()));
                        });
                    },
                );

                e.advance().unwrap();
                let after = alloc.read().unwrap();

                // mem used is the difference between the 2 measurements
                let diff = after - before;

                // tree build file size
                let tree_build_file_size = setup::serialize_tree(
                    ndm_smt.as_ref().expect("Tree not found"),
                    PathBuf::from("./target"),
                );

                let tree_build = Metrics {
                    variable: Variable::TreeBuild,
                    mem_usage: setup::bytes_as_string(diff),
                    file_size: tree_build_file_size,
                };

                println!("\n{:?}\n", tree_build);

                let mut proof = Option::<InclusionProof>::None;

                let entity_ids: Vec<&EntityId> =
                    ndm_smt.as_ref().unwrap().entity_mapping().keys().collect();

                e.advance().unwrap();
                let before = alloc.read().unwrap();

                // proof generation compute time
                group.bench_function(
                    BenchmarkId::new(
                        "generate_proof",
                        format!("{:?}/{:?}/NUM_USERS: {:?}", &tup.0, &tup.1, &tup.2),
                    ),
                    |bench| {
                        bench.iter(|| {
                            proof = Some(setup::generate_proof(
                                ndm_smt.as_ref().expect("Tree not found"),
                                entity_ids[0],
                            ));
                        });
                    },
                );

                e.advance().unwrap();
                let after = alloc.read().unwrap();

                // mem used is the difference between the 2 measurements
                let diff = after - before;

                // proof file size
                let proof_file_size = setup::serialize_proof(
                    proof.as_ref().expect("Proof not found"),
                    &entity_ids[0],
                    PathBuf::from("./target"),
                );

                let proof_generation = Metrics {
                    variable: Variable::ProofGeneration,
                    mem_usage: setup::bytes_as_string(diff),
                    file_size: proof_file_size.clone(),
                };

                println!("\n{:?}\n", proof_generation);

                e.advance().unwrap();
                let before = alloc.read().unwrap();

                // proof verification compute time
                group.bench_function(
                    BenchmarkId::new(
                        "verify_proof",
                        format!("{:?}/{:?}/NUM_USERS: {:?}", &tup.0, &tup.1, &tup.2),
                    ),
                    |bench| {
                        bench.iter(|| {
                            InclusionProof::verify(
                                proof.as_ref().expect("Proof not found"),
                                ndm_smt.as_ref().expect("Tree not found").root_hash(),
                            )
                            .expect("Unable to verify proof")
                        });
                    },
                );

                e.advance().unwrap();
                let after = alloc.read().unwrap();

                // mem used is the difference between the 2 measurements
                let diff = after - before;

                let proof_verification = Metrics {
                    variable: Variable::ProofVerification,
                    mem_usage: setup::bytes_as_string(diff),
                    file_size: proof_file_size.clone(),
                };

                println!("\n{:?}\n", proof_verification);
            }
        }
    }

    group.finish()
}

// ================================================================================================

fn bench_test_jemalloc_readings() {
    let e = epoch::mib().unwrap();
    let alloc = stats::allocated::mib().unwrap();
    let act = stats::active::mib().unwrap();
    let res = stats::resident::mib().unwrap();

    // 1 MB
    let buf: Vec<u8> = Vec::with_capacity(1024u32.pow(2) as usize);

    e.advance().unwrap();

    println!(
        "buf capacity: {:<6}",
        setup::bytes_as_string(buf.capacity())
    );

    let alloc = alloc.read().unwrap();
    let act = act.read().unwrap();
    let res = res.read().unwrap();

    println!(
        "Memory usage: {} allocated / {} active / {} resident",
        setup::bytes_as_string(alloc),
        setup::bytes_as_string(act),
        setup::bytes_as_string(res)
    );
}

// ================================================================================================

criterion_group!(benches, bench_dapol);

criterion_main!(benches, bench_test_jemalloc_readings);
