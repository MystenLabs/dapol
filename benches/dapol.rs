mod setup;

use criterion::{criterion_group, criterion_main};
use criterion::{BatchSize, BenchmarkId, Criterion, SamplingMode};
use iai_callgrind::{black_box, library_benchmark, library_benchmark_group, main};
use primitive_types::H256;

use dapol::binary_tree::{BinaryTree, Node};
use dapol::node_content::FullNodeContent;
use dapol::{Height, InclusionProof};

use setup::{NUM_USERS, TREE_HEIGHTS};

// BENCHMARKS: CRITERION
// ================================================================================================

fn bench_build_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");

    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);

    // TREE_HEIGHT = 4
    group.bench_function(BenchmarkId::new("height_4", 8), |bench| {
        bench.iter(|| {
            let tree_height = Height::from(4);
            let leaf_nodes = setup::get_input_leaf_nodes(8, &tree_height);
            setup::build_tree(tree_height, leaf_nodes, setup::get_padding_node_content());
            ()
        })
    });

    // TREE_HEIGHT = 8
    group.bench_function(BenchmarkId::new("height_8", 128), |bench| {
        bench.iter(|| {
            let tree_height = Height::from(8);
            let leaf_nodes = setup::get_input_leaf_nodes(128, &tree_height);
            setup::build_tree(tree_height, leaf_nodes, setup::get_padding_node_content());
            ()
        })
    });

    // TREE_HEIGHT = 16 (max. NUM_USERS is 32_768)
    for l in NUM_USERS[0..2].into_iter() {
        group.bench_function(BenchmarkId::new("height_16", l), |bench| {
            bench.iter(|| {
                let tree_height = Height::from(TREE_HEIGHTS[0]);
                let leaf_nodes = setup::get_input_leaf_nodes(*l, &tree_height);
                setup::build_tree(tree_height, leaf_nodes, setup::get_padding_node_content());
                ()
            })
        });
    }

    // TREE_HEIGHT = 32
    for l in NUM_USERS[0..16].into_iter() {
        group.bench_function(BenchmarkId::new("height_32", l), |bench| {
            bench.iter(|| {
                let tree_height = Height::from(TREE_HEIGHTS[1]);
                let leaf_nodes = setup::get_input_leaf_nodes(*l, &tree_height);
                setup::build_tree(tree_height, leaf_nodes, setup::get_padding_node_content());
                ()
            })
        });
    }

    // TREE_HEIGHT = 64
    for l in NUM_USERS[0..16].into_iter() {
        group.bench_function(BenchmarkId::new("height_64", l), |bench| {
            bench.iter(|| {
                let tree_height = Height::from(TREE_HEIGHTS[2]);
                let leaf_nodes = setup::get_input_leaf_nodes(*l, &tree_height);

                setup::build_tree(tree_height, leaf_nodes, setup::get_padding_node_content());
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
        let leaf_nodes = setup::get_full_node_contents();

        let tree = setup::build_tree(height, leaf_nodes.1, setup::get_full_padding_node_content());
        let leaf_node = leaf_nodes.0;

        group.bench_function(BenchmarkId::new("generate_proof", h), |bench| {
            bench.iter(|| {
                setup::generate_proof(&tree, &leaf_node);
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
        let leaf_nodes = setup::get_full_node_contents();

        let tree = setup::build_tree(height, leaf_nodes.1, setup::get_full_padding_node_content());
        let leaf_node = leaf_nodes.0;

        let root_hash = leaf_nodes.3;

        group.bench_function(BenchmarkId::new("verify_proof", h), |bench| {
            bench.iter_batched(
                || setup::generate_proof(&tree, &leaf_node),
                |proof| proof.verify(root_hash),
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// BENCHMARKS: IAI
// ================================================================================================

fn setup_generate(
    tree_height: Height,
) -> (BinaryTree<FullNodeContent>, Node<FullNodeContent>, H256) {
    let leaf_nodes = setup::get_full_node_contents();
    let tree = setup::build_tree(
        tree_height,
        leaf_nodes.1,
        setup::get_full_padding_node_content(),
    );

    (tree, leaf_nodes.0, leaf_nodes.3)
}

fn setup_verify(tree_height: Height) -> (InclusionProof, H256) {
    let leaf_nodes = setup::get_full_node_contents();
    let tree = setup::build_tree(
        tree_height,
        leaf_nodes.1,
        setup::get_full_padding_node_content(),
    );

    (setup::generate_proof(&tree, &leaf_nodes.0), leaf_nodes.3)
}

#[library_benchmark]
fn bench_build_height4() -> () {
    let tree_height = Height::from(4);
    let leaf_nodes = setup::get_input_leaf_nodes(8, &tree_height);
    black_box(setup::build_tree(
        tree_height,
        leaf_nodes,
        setup::get_padding_node_content(),
    ));
}

#[library_benchmark]
fn bench_build_height8() -> () {
    let tree_height = Height::from(8);
    let leaf_nodes = setup::get_input_leaf_nodes(128, &tree_height);
    black_box(setup::build_tree(
        tree_height,
        leaf_nodes,
        setup::get_padding_node_content(),
    ));
}

#[library_benchmark]
fn bench_build_height16() -> () {
    for l in NUM_USERS[0..2].into_iter() {
        let tree_height = Height::from(TREE_HEIGHTS[0]);
        let leaf_nodes = setup::get_input_leaf_nodes(*l, &tree_height);
        black_box(setup::build_tree(
            tree_height,
            leaf_nodes,
            setup::get_padding_node_content(),
        ));
    }
}

#[library_benchmark]
fn bench_build_height32() -> () {
    for l in NUM_USERS[0..16].into_iter() {
        let tree_height = Height::from(TREE_HEIGHTS[1]);
        let leaf_nodes = setup::get_input_leaf_nodes(*l, &tree_height);
        black_box(setup::build_tree(
            tree_height,
            leaf_nodes,
            setup::get_padding_node_content(),
        ));
    }
}

#[library_benchmark]
fn bench_build_height64() -> () {
    for l in NUM_USERS[0..16].into_iter() {
        let tree_height = Height::from(TREE_HEIGHTS[2]);
        let leaf_nodes = setup::get_input_leaf_nodes(*l, &tree_height);
        black_box(setup::build_tree(
            tree_height,
            leaf_nodes,
            setup::get_padding_node_content(),
        ));
    }
}

#[library_benchmark]
fn bench_generate_height4() -> InclusionProof {
    black_box(setup::generate_proof(
        &setup_generate(Height::from(4)).0,
        &setup_generate(Height::from(4)).1,
    ))
}

#[library_benchmark]
fn bench_generate_height8() -> InclusionProof {
    black_box(setup::generate_proof(
        &setup_generate(Height::from(8)).0,
        &setup_generate(Height::from(8)).1,
    ))
}

#[library_benchmark]
fn bench_generate_height16() -> InclusionProof {
    black_box(setup::generate_proof(
        &setup_generate(Height::from(16)).0,
        &setup_generate(Height::from(16)).1,
    ))
}

#[library_benchmark]
fn bench_generate_height32() -> InclusionProof {
    black_box(setup::generate_proof(
        &setup_generate(Height::from(32)).0,
        &setup_generate(Height::from(32)).1,
    ))
}

#[library_benchmark]
fn bench_generate_height64() -> InclusionProof {
    black_box(setup::generate_proof(
        &setup_generate(Height::from(64)).0,
        &setup_generate(Height::from(64)).1,
    ))
}

#[library_benchmark]
fn bench_verify_height4() -> () {
    let proof = setup_verify(Height::from(4)).0;
    let root_hash = setup_verify(Height::from(4)).1;

    black_box(dapol::InclusionProof::verify(&proof, root_hash).expect("Unable to verify proof"))
}

#[library_benchmark]
fn bench_verify_height8() -> () {
    let proof = setup_verify(Height::from(8)).0;
    let root_hash = setup_verify(Height::from(8)).1;

    black_box(dapol::InclusionProof::verify(&proof, root_hash).expect("Unable to verify proof"))
}

#[library_benchmark]
fn bench_verify_height16() -> () {
    let proof = setup_verify(Height::from(16)).0;
    let root_hash = setup_verify(Height::from(16)).1;

    black_box(dapol::InclusionProof::verify(&proof, root_hash).expect("Unable to verify proof"))
}

#[library_benchmark]
fn bench_verify_height32() -> () {
    let proof = setup_verify(Height::from(32)).0;
    let root_hash = setup_verify(Height::from(32)).1;

    black_box(dapol::InclusionProof::verify(&proof, root_hash).expect("Unable to verify proof"))
}

#[library_benchmark]
fn bench_verify_height64() -> () {
    let proof = setup_verify(Height::from(64)).0;
    let root_hash = setup_verify(Height::from(64)).1;

    black_box(dapol::InclusionProof::verify(&proof, root_hash).expect("Unable to verify proof"))
}

criterion_group!(
    benches,
    bench_build_tree,
    bench_generate_proof,
    bench_verify_proof
);

criterion_main!(benches);

library_benchmark_group!(
    name = bench_dapol;
    benchmarks =  bench_build_height4, bench_build_height8, bench_build_height16,  bench_build_height32, bench_build_height64, bench_generate_height4, bench_generate_height8, bench_generate_height16, bench_generate_height32, bench_generate_height64, /* bench_verify_height4, bench_verify_height8, bench_verify_height16, bench_verify_height32, bench_verify_height64 */
);

// main!(library_benchmark_groups = bench_dapol);
