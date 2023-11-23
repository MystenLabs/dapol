use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, SamplingMode};
use criterion::{BenchmarkId, Criterion};
use iai_callgrind::{black_box, library_benchmark, library_benchmark_group, main};

use dapol::accumulators::NdmSmt;
use dapol::{Height, MaxThreadCount};

mod setup;

use setup::{NUM_USERS, TREE_HEIGHTS};

// BENCHMARKS: CRITERION
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

                setup::serialize_tree(ndm_smt.as_ref().expect("Tree not found"), PathBuf::from("./target"))
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

// BENCHMARKS: IAI
// ================================================================================================

// fn setup_proof(
//     tree_height: Height,
//     max_thread_count: MaxThreadCount,
//     num_entities: u64,
// ) -> InclusionProof {
//     let ndm_smt = setup::build_ndm_smt(tree_height, max_thread_count, num_entities);
//     let entity_id = EntityId::from_str("foo").unwrap();
//     setup::generate_proof(&ndm_smt, &entity_id)
// }

#[library_benchmark]
fn bench_build_height16_threads_default_users10k() {
    dapol::initialize_machine_parallelism();

    let _ = black_box(setup::build_ndm_smt((
        Height::from(16),
        MaxThreadCount::default(),
        NUM_USERS[0],
    )));
}

#[library_benchmark]
fn bench_build_height16_threads_default_users20k() {
    dapol::initialize_machine_parallelism();

    let _ = black_box(setup::build_ndm_smt((
        Height::from(16),
        MaxThreadCount::default(),
        NUM_USERS[1],
    )));
}

#[library_benchmark]
fn bench_build_height16_threads_default_users30k() {
    dapol::initialize_machine_parallelism();

    let _ = black_box(setup::build_ndm_smt((
        Height::from(16),
        MaxThreadCount::default(),
        NUM_USERS[2],
    )));
}

// #[library_benchmark]
// fn bench_build_height32_threads_default_users10k() {
//     black_box(
//         setup::build_ndm_smt((Height::from(32), MaxThreadCount::default(), NUM_USERS[0])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height32_threads_default_users50k() {
//     black_box(
//         setup::build_ndm_smt((Height::from(32), MaxThreadCount::default(), NUM_USERS[4])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height32_threads_default_users100k() {
//     black_box(
//         setup::build_ndm_smt((Height::from(32), MaxThreadCount::default(), NUM_USERS[9])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height32_threads_default_users500k() {
//     black_box(
//         setup::build_ndm_smt((Height::from(32), MaxThreadCount::default(), NUM_USERS[13])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height32_threads_default_users1M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(32), MaxThreadCount::default(), NUM_USERS[18])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height32_threads_default_users5M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(32), MaxThreadCount::default(), NUM_USERS[22])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height32_threads_default_users10M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(32), MaxThreadCount::default(), NUM_USERS[27])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height32_threads_default_users50M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(32), MaxThreadCount::default(), NUM_USERS[29])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height32_threads_default_users100M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(32), MaxThreadCount::default(), NUM_USERS[32])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height32_threads_default_users250M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(32), MaxThreadCount::default(), NUM_USERS[34])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height64_threads_default_users10k() {
//     black_box(
//         setup::build_ndm_smt((Height::from(64), MaxThreadCount::default(), NUM_USERS[0])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height64_threads_default_users50k() {
//     black_box(
//         setup::build_ndm_smt((Height::from(64), MaxThreadCount::default(), NUM_USERS[4])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height64_threads_default_users100k() {
//     black_box(
//         setup::build_ndm_smt((Height::from(64), MaxThreadCount::default(), NUM_USERS[9])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height64_threads_default_users500k() {
//     black_box(
//         setup::build_ndm_smt((Height::from(64), MaxThreadCount::default(), NUM_USERS[13])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height64_threads_default_users1M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(64), MaxThreadCount::default(), NUM_USERS[18])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height64_threads_default_users5M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(64), MaxThreadCount::default(), NUM_USERS[22])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height64_threads_default_users10M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(64), MaxThreadCount::default(), NUM_USERS[27])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height64_threads_default_users50M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(64), MaxThreadCount::default(), NUM_USERS[29])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height64_threads_default_users100M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(64), MaxThreadCount::default(), NUM_USERS[32])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height64_threads_default_users250M() {
//     black_box(
//         setup::build_ndm_smt((Height::from(64), MaxThreadCount::default(), NUM_USERS[34])).unwrap(),
//     );
// }

// #[library_benchmark]
// fn bench_build_height64() {
//     black_box(setup::build_ndm_smt(
//         Height::from(64),
//         MaxThreadCount::default(),
//         NUM_USERS[2],
//     ));
// }

// #[library_benchmark]
// fn bench_build_max_threads4() {
//     black_box(setup::build_ndm_smt(
//         Height::from(16),
//         MaxThreadCount::from(4),
//         NUM_USERS[2],
//     ));
// }

// #[library_benchmark]
// fn bench_build_max_threads8() {
//     black_box(setup::build_ndm_smt(
//         Height::from(16),
//         MaxThreadCount::from(8),
//         NUM_USERS[2],
//     ));
// }

// #[library_benchmark]
// fn bench_build_max_threads16() {
//     black_box(setup::build_ndm_smt(
//         Height::from(16),
//         MaxThreadCount::from(16),
//         NUM_USERS[2],
//     ));
// }

// #[library_benchmark]
// fn bench_build_max_threads32() {
//     black_box(setup::build_ndm_smt(
//         Height::from(16),
//         MaxThreadCount::from(32),
//         NUM_USERS[2],
//     ));
// }

// #[library_benchmark]
// fn bench_build_max_threads64() {
//     black_box(setup::build_ndm_smt(
//         Height::from(16),
//         MaxThreadCount::from(64),
//         NUM_USERS[2],
//     ));
// }

// #[library_benchmark]
// fn bench_build_max_threads128() {
//     black_box(setup::build_ndm_smt(
//         Height::from(16),
//         MaxThreadCount::from(128),
//         NUM_USERS[2],
//     ));
// }

// #[library_benchmark]
// fn bench_build_max_threads255() {
//     black_box(setup::build_ndm_smt(
//         Height::from(16),
//         MaxThreadCount::from(255),
//         NUM_USERS[2],
//     ));
// }

// #[library_benchmark]
// fn bench_generate_height16() -> InclusionProof {
//     let ndm_smt = setup::build_ndm_smt(Height::from(16), MaxThreadCount::default(), NUM_USERS[2]);
//     let entity_id = EntityId::from_str("foo").unwrap();
//     black_box(setup::generate_proof(&ndm_smt, &entity_id))
// }

// #[library_benchmark]
// fn bench_generate_height32() -> InclusionProof {
//     let ndm_smt = setup::build_ndm_smt(Height::from(32), MaxThreadCount::default(), NUM_USERS[2]);
//     let entity_id = EntityId::from_str("foo").unwrap();
//     black_box(setup::generate_proof(&ndm_smt, &entity_id))
// }

// #[library_benchmark]
// fn bench_generate_height64() -> InclusionProof {
//     let ndm_smt = setup::build_ndm_smt(Height::from(64), MaxThreadCount::default(), NUM_USERS[2]);
//     let entity_id = EntityId::from_str("foo").unwrap();
//     black_box(setup::generate_proof(&ndm_smt, &entity_id))
// }

// #[library_benchmark]
// fn bench_generate_max_threads4() -> InclusionProof {
//     let tree_height = Height::from(16);
//     let num_entities = NUM_USERS[2];
//     let ndm_smt = setup::build_ndm_smt(tree_height, MaxThreadCount::from(4), num_entities);
//     let entity_id = EntityId::from_str("foo").unwrap();
//     black_box(setup::generate_proof(&ndm_smt, &entity_id))
// }

// #[library_benchmark]
// fn bench_generate_max_threads8() -> InclusionProof {
//     let tree_height = Height::from(16);
//     let num_entities = NUM_USERS[2];
//     let ndm_smt = setup::build_ndm_smt(tree_height, MaxThreadCount::from(8), num_entities);
//     let entity_id = EntityId::from_str("foo").unwrap();
//     black_box(setup::generate_proof(&ndm_smt, &entity_id))
// }

// #[library_benchmark]
// fn bench_generate_max_threads16() -> InclusionProof {
//     let tree_height = Height::from(16);
//     let num_entities = NUM_USERS[2];
//     let ndm_smt = setup::build_ndm_smt(tree_height, MaxThreadCount::from(16), num_entities);
//     let entity_id = EntityId::from_str("foo").unwrap();
//     black_box(setup::generate_proof(&ndm_smt, &entity_id))
// }

// #[library_benchmark]
// fn bench_generate_max_threads32() -> InclusionProof {
//     let tree_height = Height::from(16);
//     let num_entities = NUM_USERS[2];
//     let ndm_smt = setup::build_ndm_smt(tree_height, MaxThreadCount::from(32), num_entities);
//     let entity_id = EntityId::from_str("foo").unwrap();
//     black_box(setup::generate_proof(&ndm_smt, &entity_id))
// }

// #[library_benchmark]
// fn bench_generate_max_threads64() -> InclusionProof {
//     let tree_height = Height::from(16);
//     let num_entities = NUM_USERS[2];
//     let ndm_smt = setup::build_ndm_smt(tree_height, MaxThreadCount::from(64), num_entities);
//     let entity_id = EntityId::from_str("foo").unwrap();
//     black_box(setup::generate_proof(&ndm_smt, &entity_id))
// }

// #[library_benchmark]
// fn bench_generate_max_threads128() -> InclusionProof {
//     let tree_height = Height::from(16);
//     let num_entities = NUM_USERS[2];
//     let ndm_smt = setup::build_ndm_smt(tree_height, MaxThreadCount::from(128), num_entities);
//     let entity_id = EntityId::from_str("foo").unwrap();
//     black_box(setup::generate_proof(&ndm_smt, &entity_id))
// }

// #[library_benchmark]
// fn bench_generate_max_threads255() -> InclusionProof {
//     let tree_height = Height::from(16);
//     let num_entities = NUM_USERS[2];
//     let ndm_smt = setup::build_ndm_smt(tree_height, MaxThreadCount::from(255), num_entities);
//     let entity_id = EntityId::from_str("foo").unwrap();
//     black_box(setup::generate_proof(&ndm_smt, &entity_id))
// }

// TODO: add bench_verify_proof benches

criterion_group!(
    benches,
    bench_build_tree,
    // bench_generate_proof,
    // bench_verify_proof
);

criterion_main!(benches);

library_benchmark_group!(
    name = bench_dapol;
    benchmarks =
    bench_build_height16_threads_default_users10k,
    bench_build_height16_threads_default_users20k,
    bench_build_height16_threads_default_users30k,

    // bench_build_height32_threads_default_users10k,
    // bench_build_height32_threads_default_users50k,
    // bench_build_height32_threads_default_users100k,
    // bench_build_height32_threads_default_users500k,
    // bench_build_height32_threads_default_users1M,
    // bench_build_height32_threads_default_users5M,
    // bench_build_height32_threads_default_users10M,
    // bench_build_height32_threads_default_users50M,
    // bench_build_height32_threads_default_users100M,
    // bench_build_height32_threads_default_users250M,

    // bench_build_height64_threads_default_users10k,
    // bench_build_height64_threads_default_users50k,
    // bench_build_height64_threads_default_users100k,
    // bench_build_height64_threads_default_users500k,
    // bench_build_height64_threads_default_users1M,
    // bench_build_height64_threads_default_users5M,
    // bench_build_height64_threads_default_users10M,
    // bench_build_height64_threads_default_users50M,
    // bench_build_height64_threads_default_users100M,
    // bench_build_height64_threads_default_users250M,


);

// main!(library_benchmark_groups = bench_dapol);
