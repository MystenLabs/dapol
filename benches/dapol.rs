use std::path::PathBuf;

use criterion::{criterion_group, criterion_main};
use criterion::{BenchmarkId, Criterion, SamplingMode};
use jemalloc_ctl::{epoch, stats};

use dapol::accumulators::NdmSmt;
use dapol::{EntityId, Height, InclusionProof, MaxThreadCount};

mod setup;
use setup::{Metric, Variable, NUM_USERS, TREE_HEIGHTS};

use crate::setup::{MemoryUsage, VarType};

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn bench_dapol(c: &mut Criterion) {
    let e = epoch::mib().unwrap();
    let alloc = stats::allocated::mib().unwrap();
    let act = stats::active::mib().unwrap();
    let res = stats::resident::mib().unwrap();

    let mut group = c.benchmark_group("dapol");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);

    dapol::initialize_machine_parallelism();

    let mut thread_counts: Vec<u8> = Vec::new();

    let max_thread_count: u8 = MaxThreadCount::default().get_value();

    let step = if max_thread_count < 8 {
        1
    } else {
        max_thread_count >> 2
    };

    e.advance().unwrap();

    for i in (step..max_thread_count).step_by(step as usize) {
        e.advance().unwrap();
        thread_counts.push(i);
    }

    thread_counts.push(max_thread_count);

    let mut ndm_smt = Option::<NdmSmt>::None;

    e.advance().unwrap();

    for h in TREE_HEIGHTS.into_iter() {
        e.advance().unwrap();

        for t in thread_counts.iter() {
            e.advance().unwrap();

            for u in NUM_USERS.into_iter() {
                e.advance().unwrap();

                let max_users_for_height = 2_u64.pow((h - 1) as u32);

                if u > max_users_for_height {
                    break;
                }

                let tup: (Height, MaxThreadCount, u64) =
                    (Height::from(h), MaxThreadCount::from(*t), u);

                // tree build compute time
                group.bench_function(
                    BenchmarkId::new(
                        "build_tree",
                        format!("{:?}/{:?}/NUM_USERS: {:?}", &tup.0, &tup.1, &tup.2),
                    ),
                    |bench| {
                        bench.iter(|| {
                            e.advance().unwrap();
                            ndm_smt = Some(setup::build_ndm_smt(tup.clone()));
                        });
                    },
                );

                // tree build memory usage
                let alloc = alloc.read().unwrap();
                let act = act.read().unwrap();
                let res = res.read().unwrap();

                let mem_usage = MemoryUsage {
                    allocated: setup::bytes_as_string(alloc),
                    active: setup::bytes_as_string(act),
                    resident: setup::bytes_as_string(res),
                };

                // tree build file size
                let tree_build_file_size = setup::serialize_tree(
                    ndm_smt.as_ref().expect("Tree not found"),
                    PathBuf::from("./target"),
                );

                let tree_build =
                    Variable::from(Metric(VarType::TreeBuild, mem_usage, tree_build_file_size));

                println!("\nTree build metrics: {:?} \n", tree_build);

                let alloc = stats::allocated::mib().unwrap();
                let act = stats::active::mib().unwrap();
                let res = stats::resident::mib().unwrap();

                let mut proof = Option::<InclusionProof>::None;

                let entity_keys = ndm_smt.as_ref().unwrap().entity_mapping().keys();
                let mut entity_ids: Vec<&EntityId> = Vec::new();

                e.advance().unwrap();

                entity_keys.for_each(|entity| {
                    e.advance().unwrap();
                    entity_ids.push(entity);
                });

                // proof generation compute time
                group.bench_function(
                    BenchmarkId::new(
                        "generate_proof",
                        format!("{:?}/{:?}/NUM_USERS: {:?}", &tup.0, &tup.1, &tup.2),
                    ),
                    |bench| {
                        bench.iter(|| {
                            e.advance().unwrap();

                            proof = Some(setup::generate_proof(
                                ndm_smt.as_ref().expect("Tree not found"),
                                entity_ids[0],
                            ));
                        });
                    },
                );

                // proof generation memory usage
                let alloc = alloc.read().unwrap();
                let act = act.read().unwrap();
                let res = res.read().unwrap();

                let mem_usage = MemoryUsage {
                    allocated: setup::bytes_as_string(alloc),
                    active: setup::bytes_as_string(act),
                    resident: setup::bytes_as_string(res),
                };

                // proof file size
                let proof_file_size = setup::serialize_proof(
                    proof.as_ref().expect("Proof not found"),
                    &entity_ids[0],
                    PathBuf::from("./target"),
                );

                let proof_generation = Variable::from(Metric(
                    VarType::ProofGeneration,
                    mem_usage,
                    proof_file_size.clone(),
                ));

                println!("\n Proof generation metrics: {:?} \n", proof_generation);

                let alloc = stats::allocated::mib().unwrap();
                let act = stats::active::mib().unwrap();
                let res = stats::resident::mib().unwrap();

                // proof verification compute time
                group.bench_function(
                    BenchmarkId::new(
                        "verify_proof",
                        format!("{:?}/{:?}/NUM_USERS: {:?}", &tup.0, &tup.1, &tup.2),
                    ),
                    |bench| {
                        bench.iter(|| {
                            e.advance().unwrap();

                            InclusionProof::verify(
                                proof.as_ref().expect("Proof not found"),
                                ndm_smt.as_ref().expect("Tree not found").root_hash(),
                            )
                            .expect("Unable to verify proof")
                        });
                    },
                );

                // proof verification memory usage
                let alloc = alloc.read().unwrap();
                let act = act.read().unwrap();
                let res = res.read().unwrap();

                let mem_usage = MemoryUsage {
                    allocated: setup::bytes_as_string(alloc),
                    active: setup::bytes_as_string(act),
                    resident: setup::bytes_as_string(res),
                };

                let proof_verification = Variable::from(Metric(
                    VarType::ProofVerification,
                    mem_usage,
                    proof_file_size.clone(),
                ));

                println!("\n Proof verification metrics: {:?} \n", proof_verification);
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
