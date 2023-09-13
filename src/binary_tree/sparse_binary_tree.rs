//! Sparse binary tree implementation.
//!
//! A sparse binary tree is a binary tree that is *full* but not necessarily
//! *complete* or *perfect* (the definitions of which are taken from the
//! [Wikipedia entry on binary trees](https://en.wikipedia.org/wiki/Binary_tree#Types_of_binary_trees)).
//!
//! The definition given in appendix C.2 (Accumulators) in the DAPOL+ paper
//! defines a Sparse Merkle Tree (SMT) as being a Merkle tree that is *full* but
//! not necessarily *complete* or *perfect*: "In an SMT, users are mapped to and
//! reside in nodes at height 𝐻. Instead of constructing a full binary tree,
//! only tree nodes that are necessary for Merkle proofs exist"
//!
//! The definition given by
//! [Nervo's Rust implementation of an SMT](https://github.com/nervosnetwork/sparse-merkle-tree)
//! says "A sparse Merkle tree is like a standard Merkle tree, except the
//! contained data is indexed, and each datapoint is placed at the leaf that
//! corresponds to that datapoint’s index." (see [medium article](https://medium.com/@kelvinfichter/whats-a-sparse-merkle-tree-acda70aeb837)
//! for more details). This is also a *full* but not necessarily *complete* or
//! *perfect* binary tree, but the nodes must have a deterministic mapping
//! (which is not a requirement in DAPOL+).
//!
//! Either way, in this file we use 'sparse binary tree' to mean a *full* binary
//! tree.
//!
//! The tree is constructed from a vector of leaf nodes, all of which will
//! be on the bottom layer of the tree. The tree is built up from these leaves,
//! padding nodes added wherever needed in order to keep the tree *full*.
//!
//! A node is defined by it's index in the tree, which is an `(x, y)` coordinate.
//! Both `x` & `y` start from 0, `x` increasing from left to right, and `y`
//! increasing from bottom to top. The height of the tree is thus `max(y)+1`.
//! The inputted leaves used to construct the tree must contain the `x`
//! coordinate (their `y` coordinate will be 0).

use std::collections::HashMap;
use std::fmt::Debug;

use super::{Node, Coordinate, Mergeable};

/// Minimum tree height supported.
pub static MIN_HEIGHT: u8 = 2;

// -------------------------------------------------------------------------------------------------
// Main structs and constructor.

/// Main data structure.
///
/// Nodes are stored in a hash map, their index in the tree being the key.
/// There is no guarantee that all of the nodes in the tree are stored. For
/// space optimization there may be some nodes that are left out, but the
/// leaf nodes that were originally fed into the tree builder are guaranteed
/// to be stored.
#[derive(Debug)]
pub struct SparseBinaryTree<C: Clone> {
    root: Node<C>,
    store: HashMap<Coordinate, Node<C>>,
    height: u8,
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

pub struct Builder<C, F>
where
    F: Fn(&Coordinate) -> C,
{
    build_method: BuildMethod,
    height: Option<u8>,
    leaf_nodes: Option<Vec<InputLeafNode<C>>>,
    padding_node_generator: Option<F>,
}

pub enum BuildMethod {
    MultiThreaded,
    SingleThreaded,
}

impl<C, F> Builder<C, F>
where
    C: Clone + Mergeable,
    F: Fn(&Coordinate) -> C,
{
    pub fn new() -> Builder<C, F> {
        Builder {
            build_method: BuildMethod::MultiThreaded,
            height: None,
            leaf_nodes: None,
            padding_node_generator: None,
        }
    }

    pub fn with_build_method(mut self, build_method: BuildMethod) -> Builder<C, F> {
        self.build_method = build_method;
        self
    }

    pub fn with_height(mut self, height: u8) -> Result<Builder<C, F>, TreeBuildError> {
        if height < MIN_HEIGHT {
            return Err(TreeBuildError::HeightTooSmall);
        }
        self.height = Some(height);
        Ok(self)
    }

    /// Note the nodes do not have to be sorted in any way, this will be done in the build phase.
    pub fn with_leaf_nodes(
        mut self,
        leaf_nodes: Vec<InputLeafNode<C>>,
    ) -> Result<Builder<C, F>, TreeBuildError> {
        if leaf_nodes.len() < 1 {
            return Err(TreeBuildError::NoLeaves);
        }
        self.leaf_nodes = Some(leaf_nodes);
        Ok(self)
    }

    pub fn with_padding_node_generator(mut self, padding_node_generator: F) -> Builder<C, F> {
        self.padding_node_generator = Some(padding_node_generator);
        self
    }

    pub fn build(self) -> Result<SparseBinaryTree<C>, TreeBuildError> {
        use super::{single_threaded_builder, num_bottom_layer_nodes};

        // require certain fields to be set
        let leaf_nodes = self.leaf_nodes.ok_or(TreeBuildError::NoLeafNodesProvided)?;
        let height = self.height.ok_or(TreeBuildError::NoHeightProvided)?;
        let padding_node_generator = self
            .padding_node_generator
            .ok_or(TreeBuildError::NoPaddingNodeGeneratorProvided)?;

        let max_leaf_nodes = num_bottom_layer_nodes(height);
        if leaf_nodes.len() as u64 > max_leaf_nodes {
            return Err(TreeBuildError::TooManyLeaves);
        }

        // TODO need to parallelize this for when MultiThreaded build type is selected.
        // Construct a sorted vector of leaf nodes and perform parameter correctness checks.
        let mut nodes = {
            // Translate InputLeafNode to Node.
            let mut nodes: Vec<Node<C>> =
                leaf_nodes.into_iter().map(|leaf| leaf.to_node()).collect();

            // Sort by x_coord ascending.
            nodes.sort_by(|a, b| a.coord.x.cmp(&b.coord.x));

            // Make sure all x_coord < max.
            if nodes
                .last()
                .is_some_and(|node| node.coord.x >= max_leaf_nodes)
            {
                return Err(TreeBuildError::InvalidXCoord);
            }

            // Ensure no duplicates.
            let duplicate_found = nodes
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

            nodes
        };

        // TODO use multi-threaded build
        // let tree = SparseBinaryTree::new(leaf_nodes, height, &padding_node_generator)?;

        let (store, root) =
            single_threaded_builder::build_tree(nodes, height, padding_node_generator);

        Ok(SparseBinaryTree {
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
// Accessor methods.

impl<C: Clone> SparseBinaryTree<C> {
    pub fn get_height(&self) -> u8 {
        self.height
    }
    pub fn get_root(&self) -> &Node<C> {
        &self.root
    }
    /// Attempt to find a Node via it's coordinate in the underlying store.
    pub fn get_node(&self, coord: &Coordinate) -> Option<&Node<C>> {
        self.store.get(coord)
    }
    /// Attempt to find a bottom-layer leaf Node via it's x-coordinate in the
    /// underlying store.
    pub fn get_leaf_node(&self, x_coord: u64) -> Option<&Node<C>> {
        let coord = Coordinate { x: x_coord, y: 0 };
        self.get_node(&coord)
    }
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

    fn check_tree(tree: &SparseBinaryTree<TestContent>, height: u8) {
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

        let tree = SparseBinaryTree::new(leaves, height, &get_padding_function());
        assert_err!(tree, Err(SparseBinaryTreeError::TooManyLeaves));
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

        let tree = SparseBinaryTree::new(
            vec![leaf_0, leaf_1, leaf_2],
            height,
            &get_padding_function(),
        );

        assert_err!(tree, Err(SparseBinaryTreeError::DuplicateLeaves));
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

        let tree = SparseBinaryTree::new(vec![leaf_0], height, &get_padding_function());

        assert_err!(tree, Err(SparseBinaryTreeError::HeightTooSmall));
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
