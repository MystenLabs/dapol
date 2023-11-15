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

// HELPER FUNCTIONS
// ================================================================================================

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

use bulletproofs::PedersenGens;
use curve25519_dalek_ng::scalar::Scalar;
use dapol::binary_tree::{
    BinaryTree, Coordinate, InputLeafNode, Mergeable, Node, PathSiblings, TreeBuilder,
    MAX_THREAD_COUNT,
};
use dapol::node_content::FullNodeContent;
use dapol::{AggregationFactor, Hasher, Height, InclusionProof};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use primitive_types::H256;
use rand::distributions::Uniform;
use rand::{thread_rng, Rng};
use serde::Serialize;

use core::fmt::Debug;
use std::time::Duration;

// CONSTANTS
// ================================================================================================

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
            let tree_height = Height::from(TREE_HEIGHTS[0]);
            let leaf_nodes = get_leaf_nodes(8, &tree_height);
            build_tree(tree_height, leaf_nodes, get_padding_node_content());
            ()
        })
    });

    // TREE_HEIGHT = 8
    group.bench_function(BenchmarkId::new("height_8", 128), |bench| {
        bench.iter(|| {
            let tree_height = Height::from(TREE_HEIGHTS[1]);
            let leaf_nodes = get_leaf_nodes(128, &tree_height);
            build_tree(tree_height, leaf_nodes, get_padding_node_content());
            ()
        })
    });

    // TREE_HEIGHT = 16
    for l in [4096, 8192, 16_384, 32_768].into_iter() {
        group.bench_function(BenchmarkId::new("height_16", l), |bench| {
            bench.iter(|| {
                let tree_height = Height::from(TREE_HEIGHTS[2]);
                let leaf_nodes = get_leaf_nodes(l, &tree_height);
                build_tree(tree_height, leaf_nodes, get_padding_node_content());
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
                build_tree(tree_height, leaf_nodes, get_padding_node_content());
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

    // NUM_USERS is not applicable for this benchmark
    let num_leaves = NUM_USERS[0];

    for h in TREE_HEIGHTS.into_iter() {
        let tree_height = Height::from(TREE_HEIGHTS[h as usize]);
        // TODO: these need to be FullNodeContents not LeafNodes
        let leaf_nodes = get_full_node_contents();
        let mut rng = rand::thread_rng();

        let node_range = Uniform::new(0usize, num_leaves);

        let tree = build_tree(tree_height, leaf_nodes, get_full_padding_node_content());

        group.bench_function(BenchmarkId::new("splitting", h), |bench| {
            bench.iter(|| {
                let leaf_node = leaf_nodes[rng.sample(node_range)];
                generate_proof(tree, leaf_node);
            });
        });
    }
}

// HELPER FUNCTIONS
// ================================================================================================

pub fn build_tree<C, F>(
    height: Height,
    leaf_nodes: Vec<Node<C>>,
    new_padding_node_content: F,
) -> BinaryTree<C>
where
    C: Clone + Debug + Mergeable + Serialize + Send + Sync,
    F: Fn(&Coordinate) -> C + Send + Sync + 'static,
{
    let builder = TreeBuilder::<C>::new()
        .with_height(height)
        .with_leaf_nodes(leaf_nodes);

    let tree = builder
        .build_using_multi_threaded_algorithm(new_padding_node_content)
        .expect("Unable to build tree");

    tree
}

fn generate_proof(tree: BinaryTree<FullNodeContent>, leaf_node: Node<FullNodeContent>) {
    let aggregation_factor = AggregationFactor::Divisor(2u8);
    let upper_bound_bit_length = 64u8;

    // leaf at (2,0)
    let liability = 27u64;
    let blinding_factor = Scalar::from_bytes_mod_order(*b"11112222333344445555666677778888");
    let commitment = PedersenGens::default().commit(Scalar::from(liability), blinding_factor);
    let mut hasher = Hasher::new();
    hasher.update("leaf".as_bytes());
    let hash = hasher.finalize();
    let leaf = Node {
        coord: Coordinate { x: 2u64, y: 0u8 },
        content: FullNodeContent::new(liability, blinding_factor, commitment, hash),
    };

    let path_siblings = PathSiblings::build_using_multi_threaded_algorithm(
        &tree,
        &leaf_node,
        get_full_padding_node_content(),
    )
    .expect("Unable to generate path siblings");

    InclusionProof::generate(
        leaf_node,
        path_siblings,
        aggregation_factor,
        upper_bound_bit_length,
    )
    .expect("Unable to generate proof");
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

pub fn get_full_node_contents() -> Vec<Node<FullNodeContent>> {
    // leaf at (2,0)
    let liability = 27u64;
    let blinding_factor = Scalar::from_bytes_mod_order(*b"11112222333344445555666677778888");
    let commitment = PedersenGens::default().commit(Scalar::from(liability), blinding_factor);
    let mut hasher = Hasher::new();
    hasher.update("leaf".as_bytes());
    let hash = hasher.finalize();
    let leaf = Node {
        coord: Coordinate { x: 2u64, y: 0u8 },
        content: FullNodeContent::new(liability, blinding_factor, commitment, hash),
    };

    // sibling at (3,0)
    let liability = 23u64;
    let blinding_factor = Scalar::from_bytes_mod_order(*b"22223333444455556666777788881111");
    let commitment = PedersenGens::default().commit(Scalar::from(liability), blinding_factor);
    let mut hasher = Hasher::new();
    hasher.update("sibling1".as_bytes());
    let hash = hasher.finalize();
    let sibling1 = Node {
        coord: Coordinate { x: 3u64, y: 0u8 },
        content: FullNodeContent::new(liability, blinding_factor, commitment, hash),
    };

    // we need to construct the root hash & commitment for verification testing
    let (parent_hash, parent_commitment) = build_parent(
        leaf.content.commitment,
        sibling1.content.commitment,
        leaf.content.hash,
        sibling1.content.hash,
    );

    // sibling at (0,1)
    let liability = 30u64;
    let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
    let commitment = PedersenGens::default().commit(Scalar::from(liability), blinding_factor);
    let mut hasher = Hasher::new();
    hasher.update("sibling2".as_bytes());
    let hash = hasher.finalize();
    let sibling2 = Node {
        coord: Coordinate { x: 0u64, y: 1u8 },
        content: FullNodeContent::new(liability, blinding_factor, commitment, hash),
    };

    // we need to construct the root hash & commitment for verification testing
    let (parent_hash, parent_commitment) = build_parent(
        sibling2.content.commitment,
        parent_commitment,
        sibling2.content.hash,
        parent_hash,
    );

    // sibling at (1,2)
    let liability = 144u64;
    let blinding_factor = Scalar::from_bytes_mod_order(*b"44445555666677778888111122223333");
    let commitment = PedersenGens::default().commit(Scalar::from(liability), blinding_factor);
    let mut hasher = Hasher::new();
    hasher.update("sibling3".as_bytes());
    let hash = hasher.finalize();
    let sibling3 = FullNodeContent::new(liability, blinding_factor, commitment, hash);

    [leaf, sibling1, sibling2, sibling3].to_vec()
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

criterion_group!(benches, bench_build_tree);
criterion_main!(benches);
