mod setup;

use criterion::{BenchmarkId, Criterion, SamplingMode};
use dapol::Height;
use setup::{get_leaf_nodes, NUM_LEAVES, TREE_HEIGHTS};
use std::time::Duration;

fn bench_build_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");

    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.measurement_time(Duration::from_secs(20));

    let num_leaves = NUM_LEAVES.0; // 16 (i.e., 2^TREE_HEIGHTS[0])
    group.bench_function(BenchmarkId::new("height_4", num_leaves), |bench| {
        bench.iter(|| {
            let tree_height = Height::from(TREE_HEIGHTS[0]); // 4
            let leaf_nodes = get_leaf_nodes(num_leaves, &tree_height);
            setup::build_tree(tree_height, leaf_nodes);
            ()
        })
    });

    // shadow var above
    let num_leaves = NUM_LEAVES.1; // 256 (i.e., 2^TREE_HEIGHTS[1])
    group.bench_function(BenchmarkId::new("height_8", num_leaves), |bench| {
        bench.iter(|| {
            let tree_height = Height::from(TREE_HEIGHTS[1]); // 8
            let leaf_nodes = get_leaf_nodes(num_leaves, &tree_height);
            setup::build_tree(tree_height, leaf_nodes);
            ()
        })
    });
}
