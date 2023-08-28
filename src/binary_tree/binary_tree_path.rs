//! Data structures and methods related to paths in a binary tree.
//!
//! A path in a binary tree goes from a leaf node to the root node. For each node (starting from
//! the leaf node) one follows the path by moving to the node's parent; since the root node has
//! no parent this is the end of the path. A path is uniquely determined by the leaf node.

use super::sparse_binary_tree::{Coordinate, Mergeable, Node, SparseBinaryTree, MIN_HEIGHT};
use ::std::fmt::Debug;
use thiserror::Error;

use super::*;

// -------------------------------------------------------------------------------------------------
// Structs and methods for path from leaf node to root node

/// All the sibling nodes for a leaf node's path from bottom layer to root node.
#[allow(dead_code)]
pub struct PathSiblings<C: Clone> {
    leaf: Node<C>,
    siblings: Vec<Node<C>>,
}

impl<C: Mergeable + Default + Clone> SparseBinaryTree<C> {
    // TODO maybe we can compress the final result here to help keep the proof size as low as possible
    /// Construct the path up the tree from the leaf node at the given x-coord on the bottom layer
    /// to the root node. Put all the sibling nodes for the path into a vector and return it.
    #[allow(dead_code)]
    fn get_siblings_for_path(
        &self,
        leaf_x_coord: u64,
    ) -> Result<PathSiblings<C>, PathSiblingsError> {
        let coord = Coordinate::new(leaf_x_coord, 0);

        let leaf = self
            .get_node(&coord)
            .ok_or(PathSiblingsError::LeafNotFound)?;

        let mut current_node = leaf;
        let mut siblings = Vec::with_capacity(self.get_height() as usize);

        for y in 0..self.get_height() - 1 {
            let x_coord = match current_node.node_orientation() {
                NodeOrientation::Left => current_node.get_x_coord() + 1,
                NodeOrientation::Right => current_node.get_x_coord() - 1,
            };

            let sibling_coord = Coordinate::new(x_coord, y);
            siblings.push(
                self.get_node(&sibling_coord)
                    .ok_or(PathSiblingsError::NodeNotFound {
                        coord: sibling_coord,
                    })?
                    .clone(),
            );

            let parent_coord = current_node.get_parent_coord();
            current_node = self
                .get_node(&parent_coord)
                .ok_or(PathSiblingsError::NodeNotFound {
                    coord: parent_coord,
                })?;
        }

        Ok(PathSiblings {
            leaf: leaf.clone(),
            siblings,
        })
    }
}

impl<C: Mergeable + Clone + PartialEq + Debug> PathSiblings<C> {
    /// Verify that the given list of sibling nodes + the base leaf node matches the given root node.
    ///
    /// This is done by reconstructing each node in the path, from bottom layer to the root, using
    /// the given leaf and sibling nodes, and then comparing the resulting root node to the given
    /// root node.
    #[allow(dead_code)]
    fn verify(&self, root: &Node<C>) -> Result<(), PathSiblingsError> {
        let mut parent = self.leaf.clone();

        if self.siblings.len() < MIN_HEIGHT as usize {
            return Err(PathSiblingsError::TooFewSiblings);
        }

        for node in &self.siblings {
            let pair = if parent.is_right_sibling_of(node) {
                Ok(MatchedPairRef {
                    left: LeftSiblingRef(node),
                    right: RightSiblingRef(&parent),
                })
            } else if parent.is_left_sibling_of(node) {
                Ok(MatchedPairRef {
                    left: LeftSiblingRef(&parent),
                    right: RightSiblingRef(node),
                })
            } else {
                Err(PathSiblingsError::InvalidSibling {
                    given: node.get_coord().clone(),
                    calculated: parent.get_coord().clone(),
                })
            }?;
            parent = pair.merge();
        }

        if parent == *root {
            Ok(())
        } else {
            Err(PathSiblingsError::RootMismatch)
        }
    }
}

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum PathSiblingsError {
    #[error("Provided leaf node not found in the tree")]
    LeafNotFound,
    #[error("Node not found in tree ({coord:?})")]
    NodeNotFound { coord: Coordinate },
    #[error("Calculated root content does not match provided root content")]
    RootMismatch,
    #[error("Provided node ({given:?}) is not a sibling of the calculated node ({calculated:?})")]
    InvalidSibling {
        given: Coordinate,
        calculated: Coordinate,
    },
    #[error("Too few siblings")]
    TooFewSiblings,
}

// -------------------------------------------------------------------------------------------------
// Helper structs

/// A reference to a left sibling node.
///
/// It is like [super][sparse_binary_tree][LeftSibling] but does not own the underlying node.
/// The purpose of this type is for efficiency gains over [super][sparse_binary_tree][LeftSibling]
/// when ownership of the Node type is not needed.
#[allow(dead_code)]
struct LeftSiblingRef<'a, C: Clone>(&'a Node<C>);

/// A reference to a right sibling node.
///
/// It is like [super][sparse_binary_tree][RightSibling] but does not own the underlying node.
/// The purpose of this type is for efficiency gains over [super][sparse_binary_tree][RightSibling]
/// when ownership of the Node type is not needed.
#[allow(dead_code)]
struct RightSiblingRef<'a, C: Clone>(&'a Node<C>);

/// A reference to a pair of left and right sibling nodes.
///
/// It is like [super][sparse_binary_tree][MatchedPair] but does not own the underlying node.
/// The purpose of this type is for efficiency gains over [super][sparse_binary_tree][MatchedPair]
/// when ownership of the Node type is not needed.
#[allow(dead_code)]
struct MatchedPairRef<'a, C: Mergeable + Clone> {
    left: LeftSiblingRef<'a, C>,
    right: RightSiblingRef<'a, C>,
}

impl<'a, C: Mergeable + Clone> MatchedPairRef<'a, C> {
    /// Create a parent node by merging the 2 nodes in the pair.
    #[allow(dead_code)]
    fn merge(&self) -> Node<C> {
        Node::new(
            Coordinate::new(self.left.0.get_x_coord() / 2, self.left.0.get_y_coord() + 1),
            C::merge(self.left.0.get_content(), self.right.0.get_content()),
        )
    }
}

// -------------------------------------------------------------------------------------------------
// Unit tests

#[cfg(test)]
mod tests {
    use super::super::test_utils::{
        full_tree, tree_with_single_leaf, tree_with_sparse_leaves, TestContent,
    };
    use super::*;

    fn check_path_siblings(
        tree: &SparseBinaryTree<TestContent>,
        proof: &PathSiblings<TestContent>,
    ) {
        assert_eq!(proof.siblings.len() as u8, tree.get_height() - 1);
    }

    #[test]
    fn tree_works_for_full_base_layer() {
        let (tree, _height) = full_tree();

        let proof = tree
            .get_siblings_for_path(0)
            .expect("Path generation should have been successful");
        check_path_siblings(&tree, &proof);

        proof
            .verify(tree.get_root())
            .expect("Path verification should have been successful");
    }

    #[test]
    fn proofs_work_for_sparse_leaves() {
        let (tree, _height) = tree_with_sparse_leaves();

        let proof = tree
            .get_siblings_for_path(6)
            .expect("Path generation should have been successful");
        check_path_siblings(&tree, &proof);

        proof
            .verify(tree.get_root())
            .expect("Path verification should have been successful");
    }

    #[test]
    fn proofs_work_for_single_leaf() {
        let height = 4u8;

        for i in 0..2usize.pow(height as u32 - 1) {
            let tree = tree_with_single_leaf(i as u64, height);
            let proof = tree
                .get_siblings_for_path(i as u64)
                .expect("Path generation should have been successful");
            check_path_siblings(&tree, &proof);

            proof
                .verify(tree.get_root())
                .expect("Path verification should have been successful");
        }
    }
}
