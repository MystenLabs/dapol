// use core::fmt::Debug;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, SamplingMode};
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
// const NUM_LEAVES: [usize; 3] = [1024, 2048, 4096];

// // BENCHMARKS
// // ================================================================================================

fn build_dapol(c: &mut Criterion) {
    //     let mut group = c.benchmark_group("build");
    //     group.sample_size(10);
    //     group.sampling_mode(SamplingMode::Flat);
    //     group.measurement_time(Duration::from_secs(20));

    //     // bench tree height = 16
    //     let tree_height = 16;
    //     for &num_leaves in NUM_LEAVES.iter() {
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
    //     for &num_leaves in NUM_LEAVES.iter() {
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
}

fn generate_proof(c: &mut Criterion) {
    //     let mut group = c.benchmark_group("prove");
    //     group.sample_size(10);

    //     // this benchmark depends on the tree height and not the number of leaves,
    //     // so we just pick the smallest number of leaves
    //     let num_leaves = NUM_LEAVES[0];
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
}

fn verify_proof(c: &mut Criterion) {
    //     let mut group = c.benchmark_group("verify");
    //     group.sample_size(10);

    //     // this benchmark depends on the tree height and not the number of leaves,
    //     // so we just pick the smallest number of leaves
    //     let num_leaves = NUM_LEAVES[0];
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
}

criterion_group!(dapol_group, build_dapol, generate_proof, verify_proof);
criterion_main!(dapol_group);

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
