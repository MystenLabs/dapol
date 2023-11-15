// use crate::setup::{self, NUM_LEAVES, TREE_HEIGHTS};

// use criterion::{criterion_group, BenchmarkId, Criterion, SamplingMode};
// use dapol::Height;
// use std::time::Duration;

// pub fn bench_build_tree(c: &mut Criterion) {
//     let mut group = c.benchmark_group("build");

//     group.sample_size(10);
//     group.sampling_mode(SamplingMode::Flat);
//     group.measurement_time(Duration::from_secs(20));

//     // TREE_HEIGHT = 4
//     let num_leaves = NUM_LEAVES.0; // 16 (i.e., 2^4)
//     group.bench_function(BenchmarkId::new("height_4", num_leaves), |bench| {
//         bench.iter(|| {
//             let tree_height = Height::from(TREE_HEIGHTS[0]);
//             let leaf_nodes = setup::get_leaf_nodes(num_leaves, &tree_height);
//             setup::build_tree(tree_height, leaf_nodes);
//             ()
//         })
//     });

//     // TREE_HEIGHT = 8
//     let num_leaves = NUM_LEAVES.1; // 256 (i.e., 2^8)
//     group.bench_function(BenchmarkId::new("height_8", num_leaves), |bench| {
//         bench.iter(|| {
//             let tree_height = Height::from(TREE_HEIGHTS[1]);
//             let leaf_nodes = setup::get_leaf_nodes(num_leaves, &tree_height);
//             setup::build_tree(tree_height, leaf_nodes);
//             ()
//         })
//     });

//     // TREE_HEIGHT = 16
//     let num_leaves = NUM_LEAVES.2; // [1024, 2048, 4096] (i.e., 2^10, 2^11, 2^12)
//     for l in num_leaves.into_iter() {
//         group.bench_function(BenchmarkId::new("height_16", l), |bench| {
//             bench.iter(|| {
//                 let tree_height = Height::from(TREE_HEIGHTS[2]);
//                 let leaf_nodes = setup::get_leaf_nodes(l, &tree_height);
//                 setup::build_tree(tree_height, leaf_nodes);
//                 ()
//             })
//         });
//     }

//     // TREE_HEIGHT = 32
//     let num_leaves = NUM_LEAVES.3; // [4096, 8192, 16384] (i.e., 2^12, 2^13, 2^14)
//     for l in num_leaves.into_iter() {
//         group.bench_function(BenchmarkId::new("height_32", l), |bench| {
//             bench.iter(|| {
//                 let tree_height = Height::from(TREE_HEIGHTS[3]);
//                 let leaf_nodes = setup::get_leaf_nodes(l, &tree_height);
//                 setup::build_tree(tree_height, leaf_nodes);
//                 ()
//             })
//         });
//     }

//     // TREE_HEIGHT = 64
//     let num_leaves = NUM_LEAVES.4; // [16384, 32768, 65536] (i.e., 2^14, 2^15, 2^16)
//     for l in num_leaves.into_iter() {
//         group.bench_function(BenchmarkId::new("height_64", l), |bench| {
//             bench.iter(|| {
//                 let tree_height = Height::from(TREE_HEIGHTS[4]);
//                 let leaf_nodes = setup::get_leaf_nodes(l, &tree_height);
//                 setup::build_tree(tree_height, leaf_nodes);
//                 ()
//             })
//         });
//     }

//     group.finish();
// }
