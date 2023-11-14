use core::fmt::Debug;
use dapol::binary_tree::{BinaryTree, Coordinate, InputLeafNode, Mergeable, TreeBuilder};
use dapol::{Hasher, Height};
use primitive_types::H256;
use serde::Serialize;

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
