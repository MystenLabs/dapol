use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, SamplingMode};
use criterion::{BenchmarkId, Criterion};

use dapol::accumulators::NdmSmt;
use dapol::{Height, MaxThreadCount};

mod setup;

use setup::{NUM_USERS, TREE_HEIGHTS};

// ================================================================================================

// *COMPUTE TIME & SERIALIZED FILE SIZE*

// ================================================================================================

fn bench_build_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");
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

    for i in (step..max_thread_count).step_by(step as usize) {
        thread_counts.push(i);
    }

    thread_counts.push(max_thread_count);

    let mut ndm_smt = Option::<NdmSmt>::None;

    for h in TREE_HEIGHTS.into_iter() {
        for t in thread_counts.iter() {
            for u in NUM_USERS.into_iter() {
                let max_users_for_height = 2_u64.pow((h - 1) as u32);

                if u > max_users_for_height {
                    break;
                }

                let tup: (Height, MaxThreadCount, u64) =
                    (Height::from(h), MaxThreadCount::from(*t), u);

                group.bench_function(
                    BenchmarkId::new(
                        "build_tree",
                        format!("{:?}/{:?}/NUM_USERS: {:?}", &tup.0, &tup.1, &tup.2),
                    ),
                    |bench| {
                        bench.iter(|| {
                            ndm_smt = Some(setup::build_ndm_smt(tup.clone()));
                        });
                    },
                );

                setup::serialize_tree(
                    ndm_smt.as_ref().expect("Tree not found"),
                    PathBuf::from("./target"),
                )
            }
        }
    }

    group.finish()
}

// fn bench_generate_proof(c: &mut Criterion) {
//     let mut group = c.benchmark_group("generate");
//     group.sample_size(10);

//     let num_entities = NUM_USERS[2]; // 30_000: max. value for tree height 16
//     let thread_counts: [u8; 7] = [4, 8, 16, 32, 64, 128, 255];

//     for h in TREE_HEIGHTS {
//         let ndm_smt =
//             setup::build_ndm_smt(Height::from(h), MaxThreadCount::default(), num_entities);
//         let entity_id = EntityId::from_str("foo").unwrap();

//         group.bench_function(BenchmarkId::new("tree_height", h), |bench| {
//             bench.iter(|| {
//                 setup::generate_proof(&ndm_smt, &entity_id);
//             })
//         });
//     }

//     for t in thread_counts {
//         let ndm_smt = setup::build_ndm_smt(Height::from(16), MaxThreadCount::from(t), num_entities);
//         let entity_id = EntityId::from_str("foo").unwrap();

//         group.bench_function(BenchmarkId::new("max_thread_count", t), |bench| {
//             bench.iter(|| {
//                 setup::generate_proof(&ndm_smt, &entity_id);
//             })
//         });
//     }

//     for u in NUM_USERS {
//         let ndm_smt = setup::build_ndm_smt(Height::from(16), MaxThreadCount::default(), u);
//         let entity_id = EntityId::from_str("foo").unwrap();

//         group.bench_function(BenchmarkId::new("num_users", u), |bench| {
//             bench.iter(|| setup::generate_proof(&ndm_smt, &entity_id));
//         });
//     }

//     group.finish();
// }

// fn bench_verify_proof(c: &mut Criterion) {
//     let mut group = c.benchmark_group("verify");
//     group.sample_size(10);

//     let num_entities = NUM_USERS[2]; // 30_000: max. value for tree height 16
//     let thread_counts: [u8; 7] = [4, 8, 16, 32, 64, 128, 255];

//     for h in TREE_HEIGHTS {
//         let ndm_smt =
//             setup::build_ndm_smt(Height::from(h), MaxThreadCount::default(), num_entities);
//         let entity_id = EntityId::from_str("foo").unwrap();
//         let proof = setup::generate_proof(&ndm_smt, &entity_id);

//         group.bench_function(BenchmarkId::new("tree_height", h), |bench| {
//             bench.iter(|| {
//                 proof
//                     .verify(ndm_smt.root_hash())
//                     .expect("Unable to verify proof")
//             })
//         });
//     }

//     for t in thread_counts {
//         let ndm_smt = setup::build_ndm_smt(Height::from(16), MaxThreadCount::from(t), num_entities);
//         let entity_id = EntityId::from_str("foo").unwrap();
//         let proof = setup::generate_proof(&ndm_smt, &entity_id);

//         group.bench_function(BenchmarkId::new("max_thread_count", t), |bench| {
//             bench.iter(|| {
//                 proof
//                     .verify(ndm_smt.root_hash())
//                     .expect("Unable to verify proof")
//             })
//         });
//     }

//     for u in NUM_USERS {
//         let ndm_smt = setup::build_ndm_smt(Height::from(16), MaxThreadCount::default(), u);
//         let entity_id = EntityId::from_str("foo").unwrap();
//         let proof = setup::generate_proof(&ndm_smt, &entity_id);

//         group.bench_function(BenchmarkId::new("num_users", u), |bench| {
//             bench.iter(|| {
//                 proof
//                     .verify(ndm_smt.root_hash())
//                     .expect("Unable to verify proof")
//             })
//         });
//     }

// group.finish();
// }

// TODO: add bench_verify_proof benches

// ================================================================================================

// *MEMORY USAGE

// ================================================================================================

// TODO





// ================================================================================================

// ================================================================================================

criterion_group!(
    benches,
    bench_build_tree,
    // bench_generate_proof,
    // bench_verify_proof
);

criterion_main!(benches);
