//! Builder pattern for the binary tree.
//!
//! There are 2 options for builder type:
//! - [single-threaded]
//! - [multi-threaded]
//! Both require a vector of leaf nodes (which will live on the bottom layer
//! of the tree) and the tree height.

use std::fmt::Debug;

use super::{num_bottom_layer_nodes, BinaryTree, Coordinate, Mergeable, MIN_HEIGHT};

mod multi_threaded;
use multi_threaded::MultiThreadedBuilder;

mod single_threaded;
use single_threaded::SingleThreadedBuilder;

// -------------------------------------------------------------------------------------------------
// Main structs.

#[derive(Debug)]
pub struct TreeBuilder<C> {
    height: Option<u8>,
    leaf_nodes: Option<Vec<InputLeafNode<C>>>,
}

/// A simpler version of the [Node] struct that is used as input to
/// the tree builder. Since the node parameters are all assumed to be on the
/// bottom layer of the tree only the x-coord is required, the y-coord is fixed
/// and determined by the tree height.
#[derive(Debug, Clone)]
pub struct InputLeafNode<C> {
    pub content: C,
    pub x_coord: u64,
}

// -------------------------------------------------------------------------------------------------
// Implementation.

/// Example:
/// ```
/// let tree = TreeBuilder::new()
///     .with_height(height)?
///     .with_leaf_nodes(leaf_nodes)?
///     .with_single_threaded_build_algorithm()?
///     .with_padding_node_generator(new_padding_node_content)
///     .build()?;
/// ```
impl<C> TreeBuilder<C>
where
    C: Clone + Mergeable,
{
    pub fn new() -> Self {
        TreeBuilder {
            height: None,
            leaf_nodes: None,
        }
    }

    /// Set the height of the tree.
    /// Will return an error if `height` is <= the min allowed height.
    pub fn with_height(mut self, height: u8) -> Result<Self, TreeBuildError> {
        if let Some(leaf_nodes) = &self.leaf_nodes {
            if leaf_nodes.len() > num_bottom_layer_nodes(height) as usize {
                return Err(TreeBuildError::TooManyLeaves);
            }
        }

        if height < MIN_HEIGHT {
            return Err(TreeBuildError::HeightTooSmall);
        }

        self.height = Some(height);
        Ok(self)
    }

    /// The leaf nodes are those that correspond to the data that we are trying
    /// to represent in the tree. All leaf nodes are assumed to be on the bottom
    /// layer of the tree. Note the nodes do not have to be pre-sorted, sorting
    /// will occur downstream.
    /// Will return an error if `leaf_nodes` is empty.
    pub fn with_leaf_nodes(
        mut self,
        leaf_nodes: Vec<InputLeafNode<C>>,
    ) -> Result<Self, TreeBuildError> {
        if let Some(height) = self.height {
            if leaf_nodes.len() > num_bottom_layer_nodes(height) as usize {
                return Err(TreeBuildError::TooManyLeaves);
            }
        }

        if leaf_nodes.len() < 1 {
            return Err(TreeBuildError::EmptyLeaves);
        }

        self.leaf_nodes = Some(leaf_nodes);
        Ok(self)
    }

    /// High performance build algorithm utilizing parallelization.
    pub fn with_multi_threaded_build_algorithm<F>(
        self,
    ) -> Result<MultiThreadedBuilder<C, F>, TreeBuildError>
    where
        C: Debug + Send + 'static,
        F: Fn(&Coordinate) -> C + Send + Sync + 'static,
    {
        MultiThreadedBuilder::new(self)
    }

    /// Regular build algorithm.
    pub fn with_single_threaded_build_algorithm<F>(
        self,
    ) -> Result<SingleThreadedBuilder<C, F>, TreeBuildError>
    where
        C: Debug,
        F: Fn(&Coordinate) -> C,
    {
        SingleThreadedBuilder::new(self)
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

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
    // present or all not or partially present

    use super::super::*;
    use super::*;
    use crate::binary_tree::utils::test_utils::{
        full_bottom_layer, get_padding_function, single_leaf, sparse_leaves, TestContent,
    };

    use crate::testing_utils::assert_err;

    use primitive_types::H256;
    use rand::{thread_rng, Rng};

    // =========================================================================
    // Happy cases for both single- and multi-threaded builders.
    // All tests here compare the trees from the 2 build algorithms, which gives
    // a fair amount of confidence in their correctness.

    #[test]
    fn multi_and_single_give_same_root_sparse_leaves() {
        let height = 8u8;

        let leaf_nodes = sparse_leaves(height);

        let single_threaded = TreeBuilder::new()
            .with_height(height)
            .unwrap()
            .with_leaf_nodes(leaf_nodes.clone())
            .unwrap()
            .with_single_threaded_build_algorithm()
            .unwrap()
            .with_padding_node_generator(get_padding_function())
            .build()
            .unwrap();

        let multi_threaded = TreeBuilder::new()
            .with_height(height)
            .unwrap()
            .with_leaf_nodes(leaf_nodes)
            .unwrap()
            .with_multi_threaded_build_algorithm()
            .unwrap()
            .with_padding_node_generator(get_padding_function())
            .build()
            .unwrap();

        assert_eq!(single_threaded.root, multi_threaded.root);
        assert_eq!(single_threaded.height, multi_threaded.height);
        assert_eq!(single_threaded.height, height);
    }

    #[test]
    fn multi_and_single_give_same_root_full_tree() {
        let height = 8u8;

        let leaf_nodes = full_bottom_layer(height);

        let single_threaded = TreeBuilder::new()
            .with_height(height)
            .unwrap()
            .with_leaf_nodes(leaf_nodes.clone())
            .unwrap()
            .with_single_threaded_build_algorithm()
            .unwrap()
            .with_padding_node_generator(get_padding_function())
            .build()
            .unwrap();

        let multi_threaded = TreeBuilder::new()
            .with_height(height)
            .unwrap()
            .with_leaf_nodes(leaf_nodes)
            .unwrap()
            .with_multi_threaded_build_algorithm()
            .unwrap()
            .with_padding_node_generator(get_padding_function())
            .build()
            .unwrap();

        assert_eq!(single_threaded.root, multi_threaded.root);
        assert_eq!(single_threaded.height, multi_threaded.height);
        assert_eq!(single_threaded.height, height);
    }

    #[test]
    fn multi_and_single_give_same_root_single_leaf() {
        let height = 8u8;

        for i in 0..num_bottom_layer_nodes(height) {
            let leaf_node = vec![single_leaf(i as u64, height)];

            let single_threaded = TreeBuilder::new()
                .with_height(height)
                .unwrap()
                .with_leaf_nodes(leaf_node.clone())
                .unwrap()
                .with_single_threaded_build_algorithm()
                .unwrap()
                .with_padding_node_generator(get_padding_function())
                .build()
                .unwrap();

            let multi_threaded = TreeBuilder::new()
                .with_height(height)
                .unwrap()
                .with_leaf_nodes(leaf_node)
                .unwrap()
                .with_multi_threaded_build_algorithm()
                .unwrap()
                .with_padding_node_generator(get_padding_function())
                .build()
                .unwrap();

            assert_eq!(single_threaded.root, multi_threaded.root);
            assert_eq!(single_threaded.height, multi_threaded.height);
            assert_eq!(single_threaded.height, height);
        }
    }

    // =========================================================================
    // Error cases.

    #[test]
    fn err_for_empty_leaves() {
        let res = TreeBuilder::<TestContent>::new().with_leaf_nodes(Vec::new());
        assert_err!(res, Err(TreeBuildError::EmptyLeaves));
    }

    #[test]
    fn err_when_height_too_small() {
        assert!(MIN_HEIGHT > 0, "Invalid min height {}", MIN_HEIGHT);
        let height = MIN_HEIGHT - 1;
        let res = TreeBuilder::<TestContent>::new().with_height(height);
        assert_err!(res, Err(TreeBuildError::HeightTooSmall));
    }

    #[test]
    fn err_for_too_many_leaves_with_height_first() {
        let height = 8u8;
        let mut leaf_nodes = full_bottom_layer(height);

        leaf_nodes.push(InputLeafNode::<TestContent> {
            x_coord: num_bottom_layer_nodes(height) + 1,
            content: TestContent {
                hash: H256::random(),
                value: thread_rng().gen(),
            },
        });

        let res = TreeBuilder::new()
            .with_height(height)
            .unwrap()
            .with_leaf_nodes(leaf_nodes);

        assert_err!(res, Err(TreeBuildError::TooManyLeaves));
    }

    #[test]
    fn err_for_too_many_leaves_with_height_second() {
        let height = 8u8;
        let mut leaf_nodes = full_bottom_layer(height);

        leaf_nodes.push(InputLeafNode::<TestContent> {
            x_coord: num_bottom_layer_nodes(height) + 1,
            content: TestContent {
                hash: H256::random(),
                value: thread_rng().gen(),
            },
        });

        let res = TreeBuilder::new()
            .with_leaf_nodes(leaf_nodes)
            .unwrap()
            .with_height(height);

        assert_err!(res, Err(TreeBuildError::TooManyLeaves));
    }
}
