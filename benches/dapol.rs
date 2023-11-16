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

use bulletproofs::PedersenGens;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, SamplingMode};
use curve25519_dalek_ng::scalar::Scalar;
use primitive_types::H256;
use rand::distributions::Uniform;
use rand::Rng;
use serde::Serialize;

use core::fmt::Debug;
use std::time::Duration;

use dapol::binary_tree::{
    BinaryTree, Coordinate, InputLeafNode, Mergeable, Node, PathSiblings, TreeBuilder,
    MAX_THREAD_COUNT,
};
use dapol::node_content::FullNodeContent;
use dapol::{AggregationFactor, Hasher, Height, InclusionProof};

// CONSTANTS
// ================================================================================================

// TREE_HEIGHT = 4 and 8 are hard coded in benchmarks
pub const TREE_HEIGHTS: [u8; 3] = [16, 32, 64];
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

// STRUCTS
// ================================================================================================

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

// BENCHMARKS
// ================================================================================================

pub fn bench_build_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");

    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.measurement_time(Duration::from_secs(120));

    // TREE_HEIGHT = 4
    group.bench_function(BenchmarkId::new("height_4", 8), |bench| {
        bench.iter(|| {
            let tree_height = Height::from(4);
            let leaf_nodes = get_input_leaf_nodes(8, &tree_height);
            build_tree(tree_height, leaf_nodes, get_padding_node_content());
            ()
        })
    });

    // TREE_HEIGHT = 8
    group.bench_function(BenchmarkId::new("height_8", 128), |bench| {
        bench.iter(|| {
            let tree_height = Height::from(8);
            let leaf_nodes = get_input_leaf_nodes(128, &tree_height);
            build_tree(tree_height, leaf_nodes, get_padding_node_content());
            ()
        })
    });

    // TREE_HEIGHT = 16
    // NUM_USERS starts at 10_000, so these are hardcoded for benchmarking
    for l in [4096, 8192, 16_384, 32_768].into_iter() {
        group.bench_function(BenchmarkId::new("height_16", l), |bench| {
            bench.iter(|| {
                let tree_height = Height::from(TREE_HEIGHTS[0]);
                let leaf_nodes = get_input_leaf_nodes(l, &tree_height);
                build_tree(tree_height, leaf_nodes, get_padding_node_content());
                ()
            })
        });
    }

    // TREE_HEIGHT = 32
    for l in NUM_USERS[0..10].into_iter() {
        group.bench_function(BenchmarkId::new("height_32", l), |bench| {
            bench.iter(|| {
                let tree_height = Height::from(TREE_HEIGHTS[1]);
                let leaf_nodes = get_input_leaf_nodes(*l, &tree_height);
                build_tree(tree_height, leaf_nodes, get_padding_node_content());
                ()
            })
        });
    }

    // TREE_HEIGHT = 64
    for l in NUM_USERS[0..10].into_iter() {
        group.bench_function(BenchmarkId::new("height_64", l), |bench| {
            bench.iter(|| {
                let tree_height = Height::from(TREE_HEIGHTS[2]);
                let leaf_nodes = get_input_leaf_nodes(*l, &tree_height);

                build_tree(tree_height, leaf_nodes, get_padding_node_content());
                ()
            })
        });
    }

    group.finish();
}

fn bench_generate_proof(c: &mut Criterion) {
    let mut group = c.benchmark_group("prove");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(120));

    // changing NUM_USERS is not applicable for this benchmark
    let num_leaves = NUM_USERS[0]; // 16 users

    for h in TREE_HEIGHTS.into_iter() {
        let height = Height::from(h);
        let leaf_nodes = get_full_node_content(num_leaves, &height);
        let mut rng = rand::thread_rng();
        let node_range = Uniform::new(0usize, num_leaves);

        let tree = build_tree(height, leaf_nodes.clone(), get_full_padding_node_content());

        let leaf_node = leaf_nodes[rng.sample(node_range)].clone();

        group.bench_function(BenchmarkId::new("inclusion_proof", h), |bench| {
            bench.iter(|| {
                generate_proof(&tree, &leaf_node);
            });
        });
    }

    group.finish();
}

fn bench_verify_proof(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify");
    group.sample_size(10);

    // changing NUM_USERS is not applicable for this benchmark
    let num_leaves = NUM_USERS[0];

    for h in TREE_HEIGHTS.into_iter() {
        let height = Height::from(h);
        let leaf_nodes = get_full_node_content(num_leaves, &height);
        let mut rng = rand::thread_rng();
        let node_range = Uniform::new(0usize, num_leaves);

        let tree = build_tree(height, leaf_nodes.clone(), get_full_padding_node_content());
        let leaf_node = leaf_nodes[rng.sample(node_range)].clone();

        group.bench_function(BenchmarkId::new("verify_proof", h), |bench| {
            bench.iter_batched(
                || generate_proof(&tree, &leaf_node),
                |(proof)| {
                    // verify_proof
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// HELPER FUNCTIONS
// ================================================================================================

pub fn build_tree<C, F>(
    height: Height,
    leaf_nodes: Vec<Node<C>>,
    new_padding_node_content: F,
) -> BinaryTree<C>
where
    C: Clone + Debug + Mergeable + Serialize + Send + Sync + 'static,
    F: Fn(&Coordinate) -> C + Send + Sync + 'static,
{
    let mut input_leaf_nodes: Vec<InputLeafNode<C>> = Vec::new();

    leaf_nodes.into_iter().for_each(|n| {
        input_leaf_nodes.push(InputLeafNode {
            content: n.content.clone(),
            x_coord: n.coord.x,
        })
    });

    let builder = TreeBuilder::<C>::new()
        .with_height(height)
        .with_leaf_nodes(input_leaf_nodes);

    let tree = builder
        .build_using_multi_threaded_algorithm(new_padding_node_content)
        .expect("Unable to build tree");

    tree
}

fn generate_proof(
    tree: &BinaryTree<FullNodeContent>,
    leaf_node: &Node<FullNodeContent>,
) -> InclusionProof {
    let aggregation_factor = AggregationFactor::Divisor(2u8);
    let upper_bound_bit_length = 64u8;

    let path_siblings = PathSiblings::build_using_multi_threaded_algorithm(
        tree,
        &leaf_node,
        get_full_padding_node_content(),
    )
    .expect("Unable to generate path siblings");

    let proof = InclusionProof::generate(
        leaf_node.clone(),
        path_siblings,
        aggregation_factor,
        upper_bound_bit_length,
    )
    .expect("Unable to generate proof");

    proof
}

pub fn get_input_leaf_nodes(num_leaves: usize, height: &Height) -> Vec<Node<BenchTestContent>> {
    let max_bottom_layer_nodes = 2usize.pow(height.as_u32() - 1);

    assert!(
        num_leaves <= max_bottom_layer_nodes,
        "Number of leaves exceeds maximum bottom layer nodes"
    );

    let mut leaf_nodes: Vec<Node<BenchTestContent>> = Vec::new();

    for i in 0..num_leaves {
        leaf_nodes.push(
            InputLeafNode::<BenchTestContent> {
                x_coord: i as u64,
                content: BenchTestContent {
                    hash: H256::random(),
                    value: i as u32,
                },
            }
            .into_node(),
        );
    }

    leaf_nodes
}

pub fn get_full_node_content(num_leaves: usize, height: &Height) -> Vec<Node<FullNodeContent>> {
    let max_bottom_layer_nodes = 2usize.pow(height.as_u32() - 1);

    assert!(
        num_leaves <= max_bottom_layer_nodes,
        "Number of leaves exceeds maximum bottom layer nodes"
    );

    let mut rng = rand::thread_rng();
    let node_range = Uniform::new(4, height.as_y_coord());
    let liability_range = Uniform::new(1, u32::MAX);

    let blinding_factors = [
        b"90998600161833439099840024221618",
        b"34334644060024221618334357559098",
        b"16183433909906002422161834335755",
        b"46442422181134335755929806001618",
        b"90980600242216183433575590980600",
        b"18113433575590998600161834339299",
        b"57551618343357559099860018113433",
        b"57559099860016183433909986002422",
        b"34335755909986001618343390990400",
        b"06001618343390998000242216183433",
    ];

    let mut leaf_nodes: Vec<Node<FullNodeContent>> = Vec::new();

    for i in 0..num_leaves {
        let liability = rng.sample(liability_range) as u64;
        let blinding_factor =
            Scalar::from_bytes_mod_order(*blinding_factors[i % blinding_factors.len()]);
        let commitment = PedersenGens::default().commit(Scalar::from(liability), blinding_factor);
        let leaf = Node {
            coord: Coordinate {
                x: i as u64,
                y: rng.sample(node_range),
            },
            content: FullNodeContent::new(liability, blinding_factor, commitment, H256::random()),
        };

        leaf_nodes.push(leaf)
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

pub fn get_full_padding_node_content() -> impl Fn(&Coordinate) -> FullNodeContent {
    |_coord: &Coordinate| -> FullNodeContent {
        let liability = 27u64;
        let blinding_factor = Scalar::from_bytes_mod_order(*b"11112222333344445555666677778888");
        let commitment = PedersenGens::default().commit(Scalar::from(liability), blinding_factor);
        let mut hasher = Hasher::new();
        hasher.update("leaf".as_bytes());
        let hash = hasher.finalize();

        FullNodeContent::new(liability, blinding_factor, commitment, hash)
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

criterion_group!(benches, /* bench_build_tree, */ bench_generate_proof);
criterion_main!(benches);
