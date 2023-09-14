use std::collections::HashMap;
use std::fmt::Debug;

use super::{Coordinate, Mergeable, Node, BinaryTree};

mod multi_threaded;
mod single_threaded;

/// Minimum tree height supported.
pub static MIN_HEIGHT: u8 = 2;

// -------------------------------------------------------------------------------------------------
// Main structs.

pub struct TreeBuilder<C> {
    height: Option<u8>,
    leaf_nodes: Option<Vec<InputLeafNode<C>>>,
}

/// A simpler version of the [Node] struct that is used as input to instantiate
/// the tree builder.
#[derive(Clone)]
pub struct InputLeafNode<C> {
    pub content: C,
    pub x_coord: u64,
}

// -------------------------------------------------------------------------------------------------
// Tree builder.

pub struct MultiThreadedBuilder<C>
where
    C: Clone,
{
    height: u8,
    leaf_nodes: Vec<Node<C>>,
}

pub struct SingleThreadedBuilder<C>
where
    C: Clone,
{
    height: u8,
    leaf_nodes: Vec<Node<C>>,
}

impl<C> TreeBuilder<C>
where
    C: Clone + Mergeable,
{
    pub fn new() -> Self {
        TreeBuilder {
            height: None,     // TODO default to 32? Maybe here is not the best place
            leaf_nodes: None, // TODO default to empty vec?
        }
    }

    pub fn with_height(mut self, height: u8) -> Result<Self, TreeBuildError> {
        if height < MIN_HEIGHT {
            return Err(TreeBuildError::HeightTooSmall);
        }
        self.height = Some(height);
        Ok(self)
    }

    /// Note the nodes do not have to be pre-sorted, they are sorted here.
    pub fn with_leaf_nodes(
        mut self,
        leaf_nodes: Vec<InputLeafNode<C>>,
    ) -> Result<Self, TreeBuildError> {
        if leaf_nodes.len() < 1 {
            return Err(TreeBuildError::NoLeaves);
        }
        self.leaf_nodes = Some(leaf_nodes);
        Ok(self)
    }

    pub fn multi_threaded(self) -> Result<MultiThreadedBuilder<C>, TreeBuildError> {
        MultiThreadedBuilder::new(self)
    }

    pub fn single_threaded(self) -> Result<SingleThreadedBuilder<C>, TreeBuildError> {
        SingleThreadedBuilder::new(self)
    }
}

impl<C> MultiThreadedBuilder<C>
where
    C: Clone + Mergeable,
{
    fn new(parent_builder: TreeBuilder<C>) -> Result<Self, TreeBuildError> {
        use super::num_bottom_layer_nodes;

        // require certain fields to be set
        let input_leaf_nodes = parent_builder
            .leaf_nodes
            .ok_or(TreeBuildError::NoLeafNodesProvided)?;
        let height = parent_builder
            .height
            .ok_or(TreeBuildError::NoHeightProvided)?;

        let max_leaf_nodes = num_bottom_layer_nodes(height);
        if input_leaf_nodes.len() as u64 > max_leaf_nodes {
            return Err(TreeBuildError::TooManyLeaves);
        }

        // TODO need to parallelize this, it's currently the same as the single-threaded version
        // Construct a sorted vector of leaf nodes and perform parameter correctness checks.
        let mut leaf_nodes = {
            // Translate InputLeafNode to Node.
            let mut leaf_nodes: Vec<Node<C>> = input_leaf_nodes
                .into_iter()
                .map(|leaf| leaf.to_node())
                .collect();

            // Sort by x_coord ascending.
            leaf_nodes.sort_by(|a, b| a.coord.x.cmp(&b.coord.x));

            // Make sure all x_coord < max.
            if leaf_nodes
                .last()
                .is_some_and(|node| node.coord.x >= max_leaf_nodes)
            {
                return Err(TreeBuildError::InvalidXCoord);
            }

            // Ensure no duplicates.
            let duplicate_found = leaf_nodes
                .iter()
                .fold(
                    (max_leaf_nodes, false),
                    |(prev_x_coord, duplicate_found), node| {
                        if duplicate_found || node.coord.x == prev_x_coord {
                            (0, true)
                        } else {
                            (node.coord.x, false)
                        }
                    },
                )
                .1;

            if duplicate_found {
                return Err(TreeBuildError::DuplicateLeaves);
            }

            leaf_nodes
        };

        Ok(MultiThreadedBuilder { height, leaf_nodes })
    }

    pub fn build<F>(self, padding_node_generator: F) -> Result<BinaryTree<C>, TreeBuildError>
    where
        C: Debug + Send + 'static,
        F: Fn(&Coordinate) -> C + Send + 'static + Sync,
    {
        use std::sync::Arc;

        let height = self.height;
        let x_coord_min = 0;
        let x_coord_max = 2u64.pow(height as u32 - 1) - 1;
        let y = height - 1;

        let root = multi_threaded::build_node(
            x_coord_min,
            x_coord_max,
            y,
            height,
            self.leaf_nodes,
            Arc::new(padding_node_generator),
        );

        let store = HashMap::new();

        Ok(BinaryTree {
            root,
            store,
            height,
        })
    }
}

impl<C> SingleThreadedBuilder<C>
where
    C: Clone + Mergeable,
{
    fn new(parent_builder: TreeBuilder<C>) -> Result<Self, TreeBuildError> {
        use super::num_bottom_layer_nodes;

        // require certain fields to be set
        let input_leaf_nodes = parent_builder
            .leaf_nodes
            .ok_or(TreeBuildError::NoLeafNodesProvided)?;
        let height = parent_builder
            .height
            .ok_or(TreeBuildError::NoHeightProvided)?;

        let max_leaf_nodes = num_bottom_layer_nodes(height);
        if input_leaf_nodes.len() as u64 > max_leaf_nodes {
            return Err(TreeBuildError::TooManyLeaves);
        }

        // TODO need to parallelize this, it's currently the same as the single-threaded version
        // Construct a sorted vector of leaf nodes and perform parameter correctness checks.
        let mut leaf_nodes = {
            // Translate InputLeafNode to Node.
            let mut leaf_nodes: Vec<Node<C>> = input_leaf_nodes
                .into_iter()
                .map(|leaf| leaf.to_node())
                .collect();

            // Sort by x_coord ascending.
            leaf_nodes.sort_by(|a, b| a.coord.x.cmp(&b.coord.x));

            // Make sure all x_coord < max.
            if leaf_nodes
                .last()
                .is_some_and(|node| node.coord.x >= max_leaf_nodes)
            {
                return Err(TreeBuildError::InvalidXCoord);
            }

            // Ensure no duplicates.
            let duplicate_found = leaf_nodes
                .iter()
                .fold(
                    (max_leaf_nodes, false),
                    |(prev_x_coord, duplicate_found), node| {
                        if duplicate_found || node.coord.x == prev_x_coord {
                            (0, true)
                        } else {
                            (node.coord.x, false)
                        }
                    },
                )
                .1;

            if duplicate_found {
                return Err(TreeBuildError::DuplicateLeaves);
            }

            leaf_nodes
        };

        Ok(SingleThreadedBuilder { height, leaf_nodes })
    }

    pub fn build<F>(self, padding_node_generator: F) -> Result<BinaryTree<C>, TreeBuildError>
    where
        C: Debug,
        F: Fn(&Coordinate) -> C,
    {
        let height = self.height;
        let mut leaf_nodes = self.leaf_nodes;
        let (store, root) =
            single_threaded::build_tree(leaf_nodes, height, padding_node_generator);

        Ok(BinaryTree {
            root,
            store,
            height,
        })
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TreeBuildError {
    #[error("The builder must be given leaf nodes before building")]
    NoLeafNodesProvided,
    #[error("The builder must be given a height before building")]
    NoHeightProvided,
    #[error("The builder must be given a padding node generator function before building")]
    NoPaddingNodeGeneratorProvided,
    #[error("Too many leaves for the given height")]
    TooManyLeaves,
    #[error("Must provide at least 1 leaf")]
    NoLeaves,
    #[error("X coords for leaves must be less than 2^height")]
    InvalidXCoord,
    #[error("Height cannot be smaller than {MIN_HEIGHT:?}")]
    HeightTooSmall,
    #[error("Not allowed to have more than 1 leaf with the same x-coord")]
    DuplicateLeaves,
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

#[cfg(test)]
mod tests {
    // TODO test all edge cases where the first and last 2 nodes are either all
    // present or all not or partially present TODO write a test that checks the
    // total number of nodes in the tree is correct

    use super::super::test_utils::{
        full_tree, get_padding_function, tree_with_single_leaf, tree_with_sparse_leaves,
        TestContent,
    };
    use super::*;
    use crate::testing_utils::assert_err;

    use primitive_types::H256;

    fn check_tree(tree: &BinaryTree<TestContent>, height: u8) {
        assert_eq!(tree.height, height);
    }

    #[test]
    fn tree_works_for_full_base_layer() {
        let (tree, height) = full_tree();
        check_tree(&tree, height);
    }

    #[test]
    fn tree_works_for_single_leaf() {
        let height = 4u8;

        for i in 0..num_bottom_layer_nodes(height) {
            let tree = tree_with_single_leaf(i as u64, height);
            check_tree(&tree, height);
        }
    }

    #[test]
    fn tree_works_for_sparse_leaves() {
        let (tree, height) = tree_with_sparse_leaves();
        check_tree(&tree, height);
    }

    #[test]
    fn too_many_leaf_nodes_gives_err() {
        let height = 4u8;

        let mut leaves = Vec::<InputLeafNode<TestContent>>::new();

        for i in 0..(num_bottom_layer_nodes(height) + 1) {
            leaves.push(InputLeafNode::<TestContent> {
                x_coord: i as u64,
                content: TestContent {
                    hash: H256::default(),
                    value: i as u32,
                },
            });
        }

        let tree = BinaryTree::new(leaves, height, &get_padding_function());
        assert_err!(tree, Err(BinaryTreeError::TooManyLeaves));
    }

    #[test]
    fn duplicate_leaves_gives_err() {
        let height = 4u8;

        let leaf_0 = InputLeafNode::<TestContent> {
            x_coord: 7,
            content: TestContent {
                hash: H256::default(),
                value: 1,
            },
        };
        let leaf_1 = InputLeafNode::<TestContent> {
            x_coord: 1,
            content: TestContent {
                hash: H256::default(),
                value: 2,
            },
        };
        let leaf_2 = InputLeafNode::<TestContent> {
            x_coord: 7,
            content: TestContent {
                hash: H256::default(),
                value: 3,
            },
        };

        let tree = BinaryTree::new(
            vec![leaf_0, leaf_1, leaf_2],
            height,
            &get_padding_function(),
        );

        assert_err!(tree, Err(BinaryTreeError::DuplicateLeaves));
    }

    #[test]
    fn small_height_gives_err() {
        let height = 1u8;

        let leaf_0 = InputLeafNode::<TestContent> {
            x_coord: 0,
            content: TestContent {
                hash: H256::default(),
                value: 1,
            },
        };

        let tree = BinaryTree::new(vec![leaf_0], height, &get_padding_function());

        assert_err!(tree, Err(BinaryTreeError::HeightTooSmall));
    }

    #[test]
    fn concurrent_tree() {
        let height = 4u8;
        let mut leaves = Vec::<InputLeafNode<TestContent>>::new();

        for i in 0..(num_bottom_layer_nodes(height)) {
            if i < 4 {
                leaves.push(InputLeafNode::<TestContent> {
                    x_coord: i as u64,
                    content: TestContent {
                        hash: H256::default(),
                        value: i as u32,
                    },
                });
            }
        }

        dive(
            0,
            2u64.pow(height as u32 - 1) - 1,
            leaves,
            Arc::new(get_padding_function()),
        );
    }
}
