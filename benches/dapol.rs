// use core::fmt::Debug;
// use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, SamplingMode};
// use dapol::{
//     utils::get_secret, Dapol, DapolNode, RangeProofPadding, RangeProofSplitting, RangeProvable,
//     RangeVerifiable,
// };
// use digest::Digest;
// use rand::{distributions::Uniform, thread_rng, Rng};
// use smtree::{
//     index::TreeIndex,
//     traits::{ProofExtractable, Rand, Serializable, TypeName},
// };
// use std::time::Duration;

// // CONSTANTS
// // ================================================================================================

// const TREE_HEIGHTS: [usize; 3] = [16, 24, 32];
// const NUM_USERS: [usize; 3] = [1024, 2048, 4096];

// // BENCHMARKS
// // ================================================================================================

// fn build_dapol(c: &mut Criterion) {
//     let mut group = c.benchmark_group("build");
//     group.sample_size(10);
//     group.sampling_mode(SamplingMode::Flat);
//     group.measurement_time(Duration::from_secs(20));

//     // bench tree height = 16
//     let tree_height = 16;
//     for &num_leaves in NUM_USERS.iter() {
//         let items = build_item_list(num_leaves, tree_height);
//         group.bench_function(BenchmarkId::new("height_16", num_leaves), |bench| {
//             bench.iter(|| {
//                 // we bench range proof padding only because building a tree does not depend on
//                 // the type of range proof we do
//                 build_dapol_tree::<blake3::Hasher, RangeProofPadding>(&items, tree_height)
//             });
//         });
//     }

//     // bench tree height = 32
//     let tree_height = 32;
//     for &num_leaves in NUM_USERS.iter() {
//         let items = build_item_list(num_leaves, tree_height);
//         group.bench_function(BenchmarkId::new("height_32", num_leaves), |bench| {
//             bench.iter(|| {
//                 // we bench range proof padding only because building a tree does not depend on
//                 // the type of range proof we do
//                 build_dapol_tree::<blake3::Hasher, RangeProofPadding>(&items, tree_height)
//             });
//         });
//     }

//     group.finish();
// }

// fn generate_proof(c: &mut Criterion) {
//     let mut group = c.benchmark_group("prove");
//     group.sample_size(10);

//     // this benchmark depends on the tree height and not the number of leaves,
//     // so we just pick the smallest number of leaves
//     let num_leaves = NUM_USERS[0];
//     for &tree_height in TREE_HEIGHTS.iter() {
//         let items = build_item_list(num_leaves, tree_height);
//         let mut rng = thread_rng();
//         let item_range = Uniform::new(0usize, num_leaves);

//         let dapol = build_dapol_tree::<blake3::Hasher, RangeProofSplitting>(&items, tree_height);
//         group.bench_function(BenchmarkId::new("splitting", tree_height), |bench| {
//             bench.iter(|| {
//                 // time proof generation
//                 let tree_index = &items[rng.sample(item_range)].0;
//                 dapol.generate_proof(tree_index).unwrap()
//             });
//         });

//         let dapol = build_dapol_tree::<blake3::Hasher, RangeProofPadding>(&items, tree_height);
//         group.bench_function(BenchmarkId::new("padding", tree_height), |bench| {
//             bench.iter(|| {
//                 // time proof generation
//                 let tree_index = &items[rng.sample(item_range)].0;
//                 dapol.generate_proof(tree_index).unwrap()
//             });
//         });
//     }

//     group.finish();
// }

// fn verify_proof(c: &mut Criterion) {
//     let mut group = c.benchmark_group("verify");
//     group.sample_size(10);

//     // this benchmark depends on the tree height and not the number of leaves,
//     // so we just pick the smallest number of leaves
//     let num_leaves = NUM_USERS[0];
//     for &tree_height in TREE_HEIGHTS.iter() {
//         let items = build_item_list(num_leaves, tree_height);
//         let mut rng = thread_rng();
//         let item_range = Uniform::new(0usize, num_leaves);

//         let dapol = build_dapol_tree::<blake3::Hasher, RangeProofSplitting>(&items, tree_height);
//         group.bench_function(BenchmarkId::new("splitting", tree_height), |bench| {
//             bench.iter_batched(
//                 || {
//                     // generate a proof
//                     let item_idx = rng.sample(item_range);
//                     let tree_index = &items[item_idx].0;
//                     (item_idx, dapol.generate_proof(tree_index).unwrap())
//                 },
//                 |(item_idx, proof)| {
//                     // time proof verification
//                     proof.verify(&dapol.root(), &items[item_idx].1.get_proof_node())
//                 },
//                 BatchSize::SmallInput,
//             );
//         });

//         let dapol = build_dapol_tree::<blake3::Hasher, RangeProofPadding>(&items, tree_height);
//         group.bench_function(BenchmarkId::new("padding", tree_height), |bench| {
//             bench.iter_batched(
//                 || {
//                     // generate a proof
//                     let item_idx = rng.sample(item_range);
//                     let tree_index = &items[item_idx].0;
//                     (item_idx, dapol.generate_proof(tree_index).unwrap())
//                 },
//                 |(item_idx, proof)| {
//                     // time proof verification
//                     proof.verify(&dapol.root(), &items[item_idx].1.get_proof_node())
//                 },
//                 BatchSize::SmallInput,
//             );
//         });
//     }

//     group.finish();
// }

// criterion_group!(dapol_group, build_dapol, generate_proof, verify_proof);
// criterion_main!(dapol_group);

// // HELPER FUNCTIONS
// // ================================================================================================

// fn build_dapol_tree<D, R>(items: &[(TreeIndex, DapolNode<D>)], tree_height: usize) -> Dapol<D, R>
// where
//     D: Digest + Default + Clone + TypeName + Debug,
//     R: Clone + Serializable + RangeProvable + RangeVerifiable + TypeName,
// {
//     let secret = get_secret();
//     let mut dapol = Dapol::<D, R>::new_blank(tree_height, tree_height);
//     dapol.build(&items, &secret);
//     dapol
// }

// fn build_item_list(
//     num_leaves: usize,
//     tree_height: usize,
// ) -> Vec<(TreeIndex, DapolNode<blake3::Hasher>)> {
//     let mut result = Vec::new();
//     let mut value = DapolNode::<blake3::Hasher>::default();
//     let stride = 2usize.pow(tree_height as u32) / num_leaves;
//     for i in 0..num_leaves {
//         let idx = TreeIndex::from_u64(tree_height, (i * stride) as u64);
//         value.randomize();
//         result.push((idx, value.clone()));
//     }

//     result.sort_by_key(|(index, _)| *index);
//     result
// }

// mod setup;

// use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
// use dapol::Height;
// use setup::{get_leaf_nodes, NUM_USERS, TREE_HEIGHTS};
// use std::time::Duration;

// fn bench_build_tree(c: &mut Criterion) {
//     let mut group = c.benchmark_group("build");

//     group.sample_size(10);
//     group.sampling_mode(SamplingMode::Flat);
//     group.measurement_time(Duration::from_secs(20));

//     // TREE_HEIGHT = 4
//     let num_leaves = NUM_USERS.0; // 16 (i.e., 2^4)
//     group.bench_function(BenchmarkId::new("height_4", num_leaves), |bench| {
//         bench.iter(|| {
//             let tree_height = Height::from(TREE_HEIGHTS[0]);
//             let leaf_nodes = get_leaf_nodes(num_leaves, &tree_height);
//             setup::build_tree(tree_height, leaf_nodes);
//             ()
//         })
//     });

//     // TREE_HEIGHT = 8
//     let num_leaves = NUM_USERS.1; // 256 (i.e., 2^8)
//     group.bench_function(BenchmarkId::new("height_8", num_leaves), |bench| {
//         bench.iter(|| {
//             let tree_height = Height::from(TREE_HEIGHTS[1]);
//             let leaf_nodes = get_leaf_nodes(num_leaves, &tree_height);
//             setup::build_tree(tree_height, leaf_nodes);
//             ()
//         })
//     });

//     // TREE_HEIGHT = 16
//     let num_leaves = NUM_USERS.2; // [1024, 2048, 4096] (i.e., 2^10, 2^11, 2^12)
//     for l in num_leaves.into_iter() {
//         group.bench_function(BenchmarkId::new("height_16", l), |bench| {
//             bench.iter(|| {
//                 let tree_height = Height::from(TREE_HEIGHTS[2]);
//                 let leaf_nodes = get_leaf_nodes(l, &tree_height);
//                 setup::build_tree(tree_height, leaf_nodes);
//                 ()
//             })
//         });
//     }

//     // TREE_HEIGHT = 32
//     let num_leaves = NUM_USERS.3; // [4096, 8192, 16384] (i.e., 2^12, 2^13, 2^14)
//     for l in num_leaves.into_iter() {
//         group.bench_function(BenchmarkId::new("height_32", l), |bench| {
//             bench.iter(|| {
//                 let tree_height = Height::from(TREE_HEIGHTS[3]);
//                 let leaf_nodes = get_leaf_nodes(l, &tree_height);
//                 setup::build_tree(tree_height, leaf_nodes);
//                 ()
//             })
//         });
//     }

//     // TREE_HEIGHT = 64
//     let num_leaves = NUM_USERS.4; // [16384, 32768, 65536] (i.e., 2^14, 2^15, 2^16)
//     for l in num_leaves.into_iter() {
//         group.bench_function(BenchmarkId::new("height_64", l), |bench| {
//             bench.iter(|| {
//                 let tree_height = Height::from(TREE_HEIGHTS[4]);
//                 let leaf_nodes = get_leaf_nodes(l, &tree_height);
//                 setup::build_tree(tree_height, leaf_nodes);
//                 ()
//             })
//         });
//     }

//     group.finish();
// }

use dapol::binary_tree::{
    BinaryTree, Coordinate, InputLeafNode, Mergeable, TreeBuilder, MAX_THREAD_COUNT,
};
use dapol::{Hasher, Height};

use primitive_types::H256;
use serde::Serialize;

use core::fmt::Debug;
use std::time::Duration;

pub const TREE_HEIGHTS: [u8; 5] = [4, 8, 16, 32, 64];
pub const NUM_USERS: [usize; 23] = [
    10_000,
    20_000,
    40_000,
    60_000,
    80_000,
    100_000,
    200_000,
    400_000,
    600_000,
    800_000,
    1_000_000,
    2_000_000,
    4_000_000,
    6_000_000,
    8_000_000,
    10_000_000,
    20_000_000,
    40_000_000,
    60_000_000,
    80_000_000,
    100_000_000,
    125_000_000,
    250_000_000,
];

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BenchTestContent {
    pub value: u32,
    pub hash: H256,
}

impl Mergeable for BenchTestContent {
    fn merge(left_sibling: &Self, right_sibling: &Self) -> Self {
        // C(parent) = C(L) + C(R)
        let parent_value = left_sibling.value + right_sibling.value;

        // H(parent) = Hash(C(L) | C(R) | H(L) | H(R))
        let parent_hash = {
            let mut hasher = Hasher::new();
            hasher.update(&left_sibling.value.to_le_bytes());
            hasher.update(&right_sibling.value.to_le_bytes());
            hasher.update(left_sibling.hash.as_bytes());
            hasher.update(right_sibling.hash.as_bytes());
            hasher.finalize()
        };

        BenchTestContent {
            value: parent_value,
            hash: parent_hash,
        }
    }
}

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};

pub fn bench_build_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");

    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.measurement_time(Duration::from_secs(120));

    // TREE_HEIGHT = 8
    group.bench_function(BenchmarkId::new("height_4", 8), |bench| {
        bench.iter(|| {
            let tree_height = Height::from(TREE_HEIGHTS[0]);
            let leaf_nodes = get_leaf_nodes(8, &tree_height);
            build_tree(tree_height, leaf_nodes);
            ()
        })
    });

    // TREE_HEIGHT = 8
    group.bench_function(BenchmarkId::new("height_8", 128), |bench| {
        bench.iter(|| {
            let tree_height = Height::from(TREE_HEIGHTS[1]);
            let leaf_nodes = get_leaf_nodes(128, &tree_height);
            build_tree(tree_height, leaf_nodes);
            ()
        })
    });

    // TREE_HEIGHT = 16
    for l in [4096, 8192, 16_384, 32_768].into_iter() {
        group.bench_function(BenchmarkId::new("height_16", l), |bench| {
            bench.iter(|| {
                let tree_height = Height::from(TREE_HEIGHTS[2]);
                let leaf_nodes = get_leaf_nodes(l, &tree_height);
                build_tree(tree_height, leaf_nodes);
                ()
            })
        });
    }

    // TREE_HEIGHT = 32
    for l in NUM_USERS.into_iter() {
        group.bench_function(BenchmarkId::new("height_32", l), |bench| {
            bench.iter(|| {
                let tree_height = Height::from(TREE_HEIGHTS[3]);
                let leaf_nodes = get_leaf_nodes(l, &tree_height);
                build_tree(tree_height, leaf_nodes);
                ()
            })
        });
    }

    // TREE_HEIGHT = 64
    for l in NUM_USERS.into_iter() {
        group.bench_function(BenchmarkId::new("height_64", l), |bench| {
            bench.iter(|| {
                let tree_height = Height::from(TREE_HEIGHTS[4]);
                let leaf_nodes = get_leaf_nodes(l, &tree_height);
                build_tree(tree_height, leaf_nodes);
                ()
            })
        });
    }

    group.finish();
}

pub fn build_tree(
    height: Height,
    leaf_nodes: Vec<InputLeafNode<BenchTestContent>>,
) -> BinaryTree<BenchTestContent> {
    let builder = TreeBuilder::<BenchTestContent>::new()
        .with_height(height)
        .with_leaf_nodes(leaf_nodes);

    let tree = builder
        .build_using_multi_threaded_algorithm(get_padding_node_content())
        .expect("Unable to build tree");

    tree
}

pub fn get_leaf_nodes(num_leaves: usize, height: &Height) -> Vec<InputLeafNode<BenchTestContent>> {
    let max_bottom_layer_nodes = 2usize.pow(height.as_u32() - 1);

    assert!(
        num_leaves <= max_bottom_layer_nodes,
        "Number of leaves exceeds maximum bottom layer nodes"
    );

    let mut leaf_nodes: Vec<InputLeafNode<BenchTestContent>> = Vec::new();

    for i in 0..num_leaves {
        leaf_nodes.push(InputLeafNode::<BenchTestContent> {
            x_coord: i as u64,
            content: BenchTestContent {
                hash: H256::random(),
                value: i as u32,
            },
        });
    }

    leaf_nodes
}

pub fn get_padding_node_content() -> impl Fn(&Coordinate) -> BenchTestContent {
    |_coord: &Coordinate| -> BenchTestContent {
        BenchTestContent {
            value: 0,
            hash: H256::default(),
        }
    }
}

pub fn get_threads(num_cores: u8) -> Vec<u8> {
    let mut range: Vec<u8> = Vec::new();
    match num_cores {
        _ if num_cores <= 8 => {
            for i in 1..(num_cores) {
                range.push(i);
                range.push(MAX_THREAD_COUNT() / i)
            }
        }

        _ if num_cores > 8 && num_cores <= 32 => {
            for i in 1..(num_cores / 5) {
                range.push(MAX_THREAD_COUNT() / i)
            }
        }

        _ if num_cores > 32 && num_cores <= 64 => {
            for i in 1..(num_cores / 10) {
                range.push(MAX_THREAD_COUNT() / i);
            }
        }

        _ if num_cores > 64 && num_cores <= 128 => {
            for i in 1..(num_cores / 20) {
                range.push(MAX_THREAD_COUNT() / i);
            }
        }

        _ if num_cores > 128 => {
            for i in 1..(num_cores / 40) {
                range.push(MAX_THREAD_COUNT() / i);
            }
        }

        _ => panic!("Thread count overflow"),
    }

    range
}

criterion_group!(benches, bench_build_tree);
criterion_main!(benches);
