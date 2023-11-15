mod setup;

use criterion::{Criterion, SamplingMode};
use setup::TREE_HEIGHTS;
use std::time::Duration;

fn bench_build_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");

    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.measurement_time(Duration::from_secs(20));

    let tree_height = TREE_HEIGHTS[0];
}
