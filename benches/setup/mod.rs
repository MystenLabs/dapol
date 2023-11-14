use core::fmt::Debug;
use dapol::binary_tree::{BinaryTree, Coordinate, InputLeafNode, Mergeable, TreeBuilder};
use dapol::{Hasher, Height};
use primitive_types::H256;
use serde::Serialize;

pub(crate) const TREE_HEIGHTS: [u8; 5] = [4, 8, 16, 32, 64];

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

pub(crate) fn build_tree<C, F>(
    height: Height,
    leaf_nodes: Vec<InputLeafNode<C>>,
    padding_node_content: F,
) -> BinaryTree<C>
where
    C: Clone + Mergeable + Debug + Serialize + Send + Sync + 'static,
    F: Fn(&Coordinate) -> C + Send + Sync + 'static,
{
    let builder = TreeBuilder::<C>::new()
        .with_height(height)
        .with_leaf_nodes(leaf_nodes);

    let tree = builder
        .build_using_multi_threaded_algorithm(padding_node_content)
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
