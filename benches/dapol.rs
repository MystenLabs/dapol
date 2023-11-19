mod setup;

use criterion::{criterion_group, criterion_main};
use criterion::{BenchmarkId, Criterion, SamplingMode};
use iai_callgrind::{black_box, library_benchmark, library_benchmark_group, main};
use primitive_types::H256;

use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use dapol::node_content::FullNodeContent;
use dapol::{BinaryTree, EntityId, Height, InclusionProof, MaxThreadCount, Node};

use setup::{NUM_USERS, TREE_HEIGHTS};

// BENCHMARKS: CRITERION
// ================================================================================================

fn bench_build_tree_height(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");
    group.sample_size(10);

    let num_entities = NUM_USERS[2]; // 30_000: max. value for tree height 16

    for h in TREE_HEIGHTS {
        group.bench_function(BenchmarkId::new("tree_height", h), |bench| {
            bench.iter(|| {
                setup::build_ndm_smt(Height::from(h), MaxThreadCount::default(), num_entities);
            })
        });
    }

    group.finish();
}

fn bench_build_tree_max_thread_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");
    group.sample_size(10);

    let tree_height = Height::from(16);
    let num_entities = NUM_USERS[2]; // 30_000: max. value for tree height 16
    let thread_counts: [u8; 7] = [4, 8, 16, 32, 64, 128, 256];

    for t in thread_counts {
        group.bench_function(BenchmarkId::new("max_thread_count", t), |bench| {
            bench.iter(|| {
                setup::build_ndm_smt(tree_height, MaxThreadCount::from(t), num_entities);
            })
        });
    }

    group.finish();
}

fn bench_build_tree_num_users(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);

    let tree_height = Height::from(16);

    for u in NUM_USERS {
        group.bench_function(BenchmarkId::new("num_users", u), |bench| {
            bench.iter(|| {
                setup::build_ndm_smt(tree_height, MaxThreadCount::default(), u);
            })
        });
    }

    group.finish();
}

fn bench_generate_proof_tree_height(c: &mut Criterion) {
    let mut group = c.benchmark_group("prove");
    group.sample_size(10);

    let num_entities = NUM_USERS[2]; // 30_000: max. value for tree height 16

    for h in TREE_HEIGHTS {
        let ndm_smt =
            setup::build_ndm_smt(Height::from(h), MaxThreadCount::default(), num_entities);
        let entity_id = EntityId::from_str("foo").unwrap();

        group.bench_function(BenchmarkId::new("tree_height", h), |bench| {
            bench.iter(|| {
                setup::generate_proof(&ndm_smt, &entity_id);
            })
        });
    }

    group.finish();
}

fn bench_generate_proof_max_thread_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("prove");
    group.sample_size(10);

    let tree_height = Height::from(16);
    let num_entities = NUM_USERS[2]; // 30_000: max. value for tree height 16
    let thread_counts: [u8; 7] = [4, 8, 16, 32, 64, 128, 256];

    for t in thread_counts {
        let ndm_smt = setup::build_ndm_smt(tree_height, MaxThreadCount::from(t), num_entities);
        let entity_id = EntityId::from_str("foo").unwrap();

        group.bench_function(BenchmarkId::new("max_thread_count", t), |bench| {
            bench.iter(|| {
                setup::generate_proof(&ndm_smt, &entity_id);
            })
        });
    }

    group.finish();
}

fn bench_generate_proof_num_users(c: &mut Criterion) {
    let mut group = c.benchmark_group("prove");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);

    let tree_height = Height::from(16);

    for u in NUM_USERS {
        let ndm_smt = setup::build_ndm_smt(tree_height, MaxThreadCount::default(), u);
        let entity_id = EntityId::from_str("foo").unwrap();

        group.bench_function(BenchmarkId::new("num_users", u), |bench| {
            bench.iter(|| {
                setup::generate_proof(&ndm_smt, &entity_id);
            })
        });
    }

    group.finish();
}

fn bench_verify_proof(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify_proof");
    group.sample_size(10);

    for h in TREE_HEIGHTS.into_iter() {
        let height = Height::from(h);
        let leaf_nodes = setup::get_full_node_contents();

        let tree = setup::build_tree(height, leaf_nodes.1, setup::get_full_padding_node_content());
        let leaf_node = leaf_nodes.0;

        let root_hash = leaf_nodes.3;

        let proof = setup::generate_proof(&tree, &leaf_node);

        group.bench_function(BenchmarkId::new("verify_proof", h), |bench| {
            bench.iter(|| proof.verify(root_hash));
        });

        let entity_id = format!("height_{}", h);

        let path = setup::serialize_proof(proof, &entity_id, PathBuf::from("./target"));

        let file_size = fs::metadata(path)
            .expect("Unable to get proof metadata for {entity_id}")
            .len();

        println!("{entity_id} file size: {} kB", file_size / 1024u64)
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
    benchmarks = bench_build_height16,  bench_build_height32, bench_build_height64, bench_generate_height16, bench_generate_height32, bench_generate_height64, /* bench_verify_height16, bench_verify_height32, bench_verify_height64, */
);

// main!(library_benchmark_groups = bench_dapol);
