use std::fmt::Debug;

use super::{BinaryTree, Coordinate, Mergeable, MIN_HEIGHT};

mod multi_threaded;
use multi_threaded::MultiThreadedBuilder;

mod single_threaded;
use single_threaded::SingleThreadedBuilder;

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
// Implementation.

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
            return Err(TreeBuildError::EmptyLeaves);
        }
        self.leaf_nodes = Some(leaf_nodes);
        Ok(self)
    }

    pub fn multi_threaded<F>(self) -> Result<MultiThreadedBuilder<C, F>, TreeBuildError>
    where
        C: Debug + Send + 'static,
        F: Fn(&Coordinate) -> C + Send + 'static + Sync,
    {
        MultiThreadedBuilder::new(self)
    }

    pub fn single_threaded<F>(self) -> Result<SingleThreadedBuilder<C, F>, TreeBuildError>
    where
        C: Debug,
        F: Fn(&Coordinate) -> C,
    {
        SingleThreadedBuilder::new(self)
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
    #[error("Leaf nodes cannot be empty")]
    EmptyLeaves,
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
