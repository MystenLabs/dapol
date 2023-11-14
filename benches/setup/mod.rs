use dapol::binary_tree::{BinaryTree, Coordinate, InputLeafNode, Mergeable, TreeBuilder};
use dapol::{Hasher, Height};

use log::error;
use primitive_types::H256;
use serde::Serialize;

use core::fmt::Debug;

pub(crate) const TREE_HEIGHTS: [u8; 5] = [4, 8, 16, 32, 64];
pub(crate) const NUM_USERS: (u32, u32, u32, [u32; 5], [u32; 5], [u32; 5], [u32; 5]) = (
    250_000_000,
    125_000_000,
    100_000_000,
    [10_000_000, 20_000_000, 40_000_000, 60_000_000, 80_000_000],
    [1_000_000, 2_000_000, 4_000_000, 6_000_000, 8_000_000],
    [100_000, 200_000, 400_000, 600_000, 800_000],
    [10_000, 20_000, 40_000, 60_000, 80_000],
);

// This is used to determine the number of threads to spawn in the
// multi-threaded builder.
pub(crate) fn MAX_THREAD_COUNT() -> Option<u8> {
    dapol::utils::DEFAULT_PARALLELISM_APPROX.with(|opt| {
        *opt.borrow_mut() = std::thread::available_parallelism()
            .map_err(|err| {
                error!("Problem accessing machine parallelism: {}", err);
                err
            })
            .map_or(None, |par| Some(par.get() as u8));
        opt.clone().into_inner()
    })
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct BenchTestContent {
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

pub(crate) fn build_tree<F>(
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

pub(crate) fn get_leaf_nodes(
    num_leaves: usize,
    height: Height,
) -> Vec<InputLeafNode<BenchTestContent>> {
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

fn get_padding_node_content() -> impl Fn(&Coordinate) -> BenchTestContent {
    |_coord: &Coordinate| -> BenchTestContent {
        BenchTestContent {
            value: 0,
            hash: H256::default(),
        }
    }
}
