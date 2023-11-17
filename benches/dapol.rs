use bulletproofs::PedersenGens;
use criterion::{criterion_group, criterion_main};
use criterion::{BatchSize, BenchmarkId, Criterion, SamplingMode};
use curve25519_dalek_ng::ristretto::RistrettoPoint;
use curve25519_dalek_ng::scalar::Scalar;
use primitive_types::H256;
use rand::distributions::Uniform;
use rand::Rng;
use serde::Serialize;

use iai_callgrind::{black_box, library_benchmark, library_benchmark_group, main};

use core::fmt::Debug;

use dapol::binary_tree::{
    BinaryTree, Coordinate, InputLeafNode, Mergeable, Node, PathSiblings, TreeBuilder,
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
    // group.measurement_time(Duration::from_secs(120));

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

    // TREE_HEIGHT = 16 (max. NUM_USERS is 32_768)
    for l in NUM_USERS[0..2].into_iter() {
        group.bench_function(BenchmarkId::new("height_16", l), |bench| {
            bench.iter(|| {
                let tree_height = Height::from(TREE_HEIGHTS[0]);
                let leaf_nodes = get_input_leaf_nodes(*l, &tree_height);
                build_tree(tree_height, leaf_nodes, get_padding_node_content());
                ()
            })
        });
    }

    // TREE_HEIGHT = 32
    for l in NUM_USERS[0..16].into_iter() {
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
    for l in NUM_USERS[0..16].into_iter() {
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

    for h in TREE_HEIGHTS.into_iter() {
        let height = Height::from(h);
        let leaf_nodes = get_full_node_contents();

        let tree = build_tree(height, leaf_nodes.1, get_full_padding_node_content());
        let leaf_node = leaf_nodes.0;

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

    for h in TREE_HEIGHTS.into_iter() {
        let height = Height::from(h);
        let leaf_nodes = get_full_node_contents();

        let tree = build_tree(height, leaf_nodes.1, get_full_padding_node_content());
        let leaf_node = leaf_nodes.0;

        let root_hash = leaf_nodes.3;

        group.bench_function(BenchmarkId::new("verify_proof", h), |bench| {
            bench.iter_batched(
                || generate_proof(&tree, &leaf_node),
                |proof| proof.verify(root_hash),
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn setup_generate(
    tree_height: Height,
) -> (BinaryTree<FullNodeContent>, Node<FullNodeContent>, H256) {
    let leaf_nodes = get_full_node_contents();
    let tree = build_tree(tree_height, leaf_nodes.1, get_full_padding_node_content());

    (tree, leaf_nodes.0, leaf_nodes.3)
}

fn setup_verify(tree_height: Height) -> (InclusionProof, H256) {
    let leaf_nodes = get_full_node_contents();
    let tree = build_tree(tree_height, leaf_nodes.1, get_full_padding_node_content());

    (generate_proof(&tree, &leaf_nodes.0), leaf_nodes.3)
}

#[library_benchmark]
fn bench_build_height4() -> () {
    let tree_height = Height::from(4);
    let leaf_nodes = get_input_leaf_nodes(8, &tree_height);
    build_tree(tree_height, leaf_nodes, get_padding_node_content());
}

#[library_benchmark]
fn bench_build_height8() -> () {
    let tree_height = Height::from(8);
    let leaf_nodes = get_input_leaf_nodes(128, &tree_height);
    build_tree(tree_height, leaf_nodes, get_padding_node_content());
}

#[library_benchmark]
fn bench_build_height16() -> () {
    for l in NUM_USERS[0..2].into_iter() {
        let tree_height = Height::from(TREE_HEIGHTS[0]);
        let leaf_nodes = get_input_leaf_nodes(*l, &tree_height);
        build_tree(tree_height, leaf_nodes, get_padding_node_content());
    }
}

#[library_benchmark]
fn bench_build_height32() -> () {
    for l in NUM_USERS[0..16].into_iter() {
        let tree_height = Height::from(TREE_HEIGHTS[1]);
        let leaf_nodes = get_input_leaf_nodes(*l, &tree_height);
        build_tree(tree_height, leaf_nodes, get_padding_node_content());
    }
}

#[library_benchmark]
fn bench_build_height64() -> () {
    for l in NUM_USERS[0..16].into_iter() {
        let tree_height = Height::from(TREE_HEIGHTS[2]);
        let leaf_nodes = get_input_leaf_nodes(*l, &tree_height);
        build_tree(tree_height, leaf_nodes, get_padding_node_content());
    }
}

#[library_benchmark]
fn bench_generate_height4() -> InclusionProof {
    generate_proof(
        black_box(&setup_generate(Height::from(4)).0),
        black_box(&setup_generate(Height::from(4)).1),
    )
}

#[library_benchmark]
fn bench_generate_height8() -> InclusionProof {
    generate_proof(
        black_box(&setup_generate(Height::from(8)).0),
        black_box(&setup_generate(Height::from(8)).1),
    )
}

#[library_benchmark]
fn bench_generate_height16() -> InclusionProof {
    generate_proof(
        black_box(&setup_generate(Height::from(16)).0),
        black_box(&setup_generate(Height::from(16)).1),
    )
}

#[library_benchmark]
fn bench_generate_height32() -> InclusionProof {
    generate_proof(
        black_box(&setup_generate(Height::from(32)).0),
        black_box(&setup_generate(Height::from(32)).1),
    )
}

#[library_benchmark]
fn bench_generate_height64() -> InclusionProof {
    generate_proof(
        black_box(&setup_generate(Height::from(64)).0),
        black_box(&setup_generate(Height::from(64)).1),
    )
}

#[library_benchmark]
fn bench_verify_height4() -> () {
    let proof = black_box(setup_verify(Height::from(4)).0);
    let root_hash = black_box(setup_verify(Height::from(4)).1);

    dapol::InclusionProof::verify(&proof, root_hash).expect("Unable to verify proof")

    // proof.verify(root_hash).expect("Unable to verify proof")
}

#[library_benchmark]
fn bench_verify_height8() -> () {
    let proof = black_box(setup_verify(Height::from(8)).0);
    let root_hash = black_box(setup_verify(Height::from(8)).1);

    dapol::InclusionProof::verify(&proof, root_hash).expect("Unable to verify proof")

    // proof.verify(root_hash).expect("Unable to verify proof")
}

#[library_benchmark]
fn bench_verify_height16() -> () {
    let proof = black_box(setup_verify(Height::from(16)).0);
    let root_hash = black_box(setup_verify(Height::from(16)).1);

    dapol::InclusionProof::verify(&proof, root_hash).expect("Unable to verify proof")

    // proof.verify(root_hash).expect("Unable to verify proof")
}

#[library_benchmark]
fn bench_verify_height32() -> () {
    let proof = black_box(setup_verify(Height::from(32)).0);
    let root_hash = black_box(setup_verify(Height::from(32)).1);

    dapol::InclusionProof::verify(&proof, root_hash).expect("Unable to verify proof")

    // proof.verify(root_hash).expect("Unable to verify proof")
}

#[library_benchmark]
fn bench_verify_height64() -> () {
    let proof = black_box(setup_verify(Height::from(64)).0);
    let root_hash = black_box(setup_verify(Height::from(64)).1);

    dapol::InclusionProof::verify(&proof, root_hash).expect("Unable to verify proof")

    // proof.verify(root_hash).expect("Unable to verify proof")
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
        leaf_node,
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

pub fn get_full_node_contents(// height: &Height,
) -> (
    Node<FullNodeContent>,
    Vec<Node<FullNodeContent>>,
    RistrettoPoint,
    H256,
) {
    let mut rng = rand::thread_rng();
    let liability_range = Uniform::new(1, u64::MAX);

    let liabilities = [
        rng.sample(liability_range),
        rng.sample(liability_range),
        rng.sample(liability_range),
        rng.sample(liability_range),
    ];

    let blinding_factors = [
        Scalar::from_bytes_mod_order(*b"90998600161833439099840024221618"),
        Scalar::from_bytes_mod_order(*b"34334644060024221618334357559098"),
        Scalar::from_bytes_mod_order(*b"16183433909906002422161834335755"),
        Scalar::from_bytes_mod_order(*b"46442422181134335755929806001618"),
    ];

    let commitments = [
        PedersenGens::default().commit(Scalar::from(liabilities[0]), blinding_factors[0]),
        PedersenGens::default().commit(Scalar::from(liabilities[1]), blinding_factors[1]),
        PedersenGens::default().commit(Scalar::from(liabilities[2]), blinding_factors[2]),
        PedersenGens::default().commit(Scalar::from(liabilities[3]), blinding_factors[3]),
    ];

    let bytes = [
        b"leafleafleafleafleafleafleafleaf",
        b"sibling1sibling1sibling1sibling1",
        b"sibling2sibling2sibling2sibling2",
        b"sibling3sibling3sibling3sibling3",
    ];

    let mut hasher = Hasher::new();
    hasher.update(bytes[0]);
    let hash = hasher.finalize();

    let leaf = Node {
        coord: Coordinate { x: 2u64, y: 0u8 },
        content: FullNodeContent::new(liabilities[0], blinding_factors[0], commitments[0], hash),
    };

    let mut hasher = Hasher::new();
    hasher.update(bytes[1]);
    let hash = hasher.finalize();

    let sibling1 = Node {
        coord: Coordinate { x: 3u64, y: 0u8 },
        content: FullNodeContent::new(liabilities[1], blinding_factors[1], commitments[1], hash),
    };

    let (parent_commitment, parent_hash) = get_parent_node(
        leaf.content.commitment,
        sibling1.content.commitment,
        leaf.content.hash,
        sibling1.content.hash,
    );

    let mut hasher = Hasher::new();
    hasher.update(bytes[2]);
    let hash = hasher.finalize();

    let sibling2 = Node {
        coord: Coordinate { x: 0u64, y: 1u8 },
        content: FullNodeContent::new(liabilities[2], blinding_factors[2], commitments[2], hash),
    };

    let (parent_commitment, parent_hash) = get_parent_node(
        sibling2.content.commitment,
        parent_commitment,
        sibling2.content.hash,
        parent_hash,
    );

    let mut hasher = Hasher::new();
    hasher.update(bytes[3]);
    let hash = hasher.finalize();

    let sibling3 = Node {
        coord: Coordinate { x: 1u64, y: 2u8 },
        content: FullNodeContent::new(liabilities[3], blinding_factors[3], commitments[3], hash),
    };

    let (root_commitment, root_hash) = get_parent_node(
        sibling3.content.commitment,
        parent_commitment,
        sibling3.content.hash,
        parent_hash,
    );

    let nodes = vec![leaf.clone(), sibling1, sibling2, sibling3];

    (leaf, nodes, root_commitment, root_hash)
}

fn get_parent_node(
    left_commitment: RistrettoPoint,
    right_commitment: RistrettoPoint,
    left_hash: H256,
    right_hash: H256,
) -> (RistrettoPoint, H256) {
    let parent_commitment: RistrettoPoint = left_commitment + right_commitment;

    let mut hasher = Hasher::new();

    let parent_hash: H256 = {
        hasher.update(left_commitment.compress().as_bytes());
        hasher.update(right_commitment.compress().as_bytes());
        hasher.update(left_hash.as_bytes());
        hasher.update(right_hash.as_bytes());
        hasher.finalize()
    };

    (parent_commitment, parent_hash)
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

criterion_group!(
    benches,
    bench_build_tree,
    bench_generate_proof,
    bench_verify_proof
);

// criterion_main!(benches);

library_benchmark_group!(
    name = bench_dapol;
    benchmarks = bench_build_height4, bench_build_height8, bench_build_height16, bench_build_height32, bench_build_height64, bench_generate_height4, bench_generate_height8, bench_generate_height16, bench_generate_height32, bench_generate_height64, bench_verify_height4, bench_verify_height8, bench_verify_height16, bench_verify_height32, bench_verify_height64
);

main!(library_benchmark_groups = bench_dapol);
