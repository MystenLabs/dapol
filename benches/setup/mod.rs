use core::fmt::Debug;
use dapol::binary_tree::{BinaryTree, Coordinate, InputLeafNode, Mergeable, TreeBuilder};
use dapol::Height;
use serde::Serialize;

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
