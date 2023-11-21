mod setup;

use criterion::{criterion_group, criterion_main, SamplingMode};
use criterion::{BenchmarkId, Criterion};
use dapol::accumulators::{NdmSmt, NdmSmtSecrets};
use iai_callgrind::{black_box, library_benchmark, library_benchmark_group, main};

use std::path::PathBuf;
use std::str::FromStr;

use dapol::{
    Entity, EntityId, Height, MaxThreadCount, Secret,
};

use setup::{NUM_USERS, TREE_HEIGHTS};

// BENCHMARKS: CRITERION
// ================================================================================================

fn bench_build_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);

    dapol::initialize_machine_parallelism();

    let max_thread_count = MaxThreadCount::default().get_value();

    let mut thread_counts: Vec<u8> = Vec::new();

    if max_thread_count <= 8 {
        for i in 1..max_thread_count {
            thread_counts.push(i)
        }
    } else if max_thread_count > 8 && max_thread_count <= 32 {
        for i in 1..max_thread_count / 2 {
            thread_counts.push(i * 2)
        }
    } else if max_thread_count > 32 && max_thread_count <= 64 {
        for i in 1..max_thread_count / 4 {
            thread_counts.push(i * 4)
        }

    } else if max_thread_count > 64 && max_thread_count <= 128 {
        for i in 1..max_thread_count / 8 {
            thread_counts.push(i * 8);
        }

    } else if max_thread_count > 128 && max_thread_count <= 192 {
        for i in 1..max_thread_count / 16 {
            thread_counts.push(i * 16);
        }
    } else {
        for i in 1..max_thread_count / 32 {
            thread_counts.push(i * 32);
        }
    }
    
    thread_counts.push(max_thread_count);

    let dummy_secrets = NdmSmtSecrets {
        master_secret: Secret::from_str("master_secret").unwrap(),
        salt_b: dapol::Secret::from_str("salt_b").unwrap(),
        salt_s: Secret::from_str("salt_s").unwrap(),
    };

    let mut dummy_entities: Vec<Entity> = Vec::new();

    dummy_entities.push(Entity {
        liability: 893267,
        id: EntityId::from_str("john.doe@example.com").unwrap(),
    });

    dummy_entities.push(Entity {
        liability: 724851,
        id: EntityId::from_str("jane.smith@example.com").unwrap(),
    });

    let mut ndm_smt = NdmSmt::new(
        dummy_secrets,
        Height::from(16),
        MaxThreadCount::default(),
        dummy_entities,
    )
    .unwrap();

    // TREE_HEIGHT = 16
    // tree height = 16 maxes out at 32_768, so num users only goes up to 30_000
    for t in thread_counts.iter() {
        for u in NUM_USERS[0..=2].into_iter() {
            let tup: (Height, MaxThreadCount, u64) =
                (Height::from(16), MaxThreadCount::from(*t), *u);

            group.bench_function(
                BenchmarkId::new(
                    "build_tree",
                    format!("{:?}/{:?}/NUM_USERS: {:?}", &tup.0, &tup.1, &tup.2),
                ),
                |bench| {
                    bench.iter(|| {
                        ndm_smt = setup::build_ndm_smt(tup.clone());
                    })
                },
            );
            setup::serialize_tree(&ndm_smt, PathBuf::from("./target"));
        }
    }

    // TREE_HEIGHT = 32 and 64
    for h in [32u8, 64u8].into_iter() {
        for t in thread_counts.iter() {
            for u in NUM_USERS {
                let tup: (Height, MaxThreadCount, u64) =
                    (Height::from(h), MaxThreadCount::from(*t), u);
                group.bench_function(
                    BenchmarkId::new(
                        "build_tree",
                        format!("{:?}/{:?}/{:?}", &tup.0, &tup.1, &tup.2),
                    ),
                    |bench| {
                        bench.iter(|| {
                            setup::build_ndm_smt(tup.clone());
                        })
                    },
                );

                setup::serialize_tree(&ndm_smt, PathBuf::from("./target"));
            }
        }
    }

    group.finish();
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

// #[library_benchmark]
// fn bench_build_height16() {
//     black_box(setup::build_ndm_smt(
//         Height::from(16),
//         MaxThreadCount::default(),
//         NUM_USERS[2],
//     ));
// }

// #[library_benchmark]
// fn bench_build_height32() {
//     black_box(setup::build_ndm_smt(
//         Height::from(32),
//         MaxThreadCount::default(),
//         NUM_USERS[2],
//     ));
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

// library_benchmark_group!(
//     name = bench_dapol;
//     benchmarks = bench_build_height16, bench_build_height32, bench_build_height64, bench_build_max_threads4, bench_build_max_threads8, bench_build_max_threads16, bench_build_height32, bench_build_height64, bench_build_max_threads128, bench_build_max_threads255,
//     bench_generate_height16, bench_generate_height32, bench_generate_height64, bench_generate_max_threads4, bench_generate_max_threads8, bench_generate_max_threads16, bench_generate_height32, bench_generate_height64, bench_generate_max_threads128, bench_generate_max_threads255,
// );

// main!(library_benchmark_groups = bench_dapol);
