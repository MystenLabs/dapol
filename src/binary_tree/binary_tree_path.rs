use super::Mergeable;
use super::MIN_HEIGHT;
use super::{Coordinate, Node};

use std::fmt::Debug;
use thiserror::Error;

// ===========================================
// Errors

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

// ===========================================
// Types

/// All the sibling nodes for a leaf node's path from bottom layer to root node.
#[allow(dead_code)]
pub struct PathSiblings<C: Clone> {
    pub leaf: Node<C>,
    pub siblings: Vec<Node<C>>,
    pub root: Node<C>,
}

/// Ease of use types for efficiency gains when ownership of the Node type is not needed.
#[allow(dead_code)]
struct LeftSiblingRef<'a, C: Clone>(&'a Node<C>);

/// Ease of use types for efficiency gains when ownership of the Node type is not needed.
#[allow(dead_code)]
struct RightSiblingRef<'a, C: Clone>(&'a Node<C>);

/// Ease of use types for efficiency gains when ownership of the Node type is not needed.
#[allow(dead_code)]
struct MatchedPairRef<'a, C: Mergeable + Clone> {
    left: LeftSiblingRef<'a, C>,
    right: RightSiblingRef<'a, C>,
}

// ===========================================
// Implementations

impl<C: Mergeable + Clone + PartialEq + Debug> PathSiblings<C> {
    /// Verify that a path reconstruction using the leaf node and the path siblings results in the same root node.
    #[allow(dead_code)]
    fn verify(&self) -> Result<(), PathSiblingsError> {
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
                    given: node.coord.clone(),
                    calculated: parent.coord.clone(),
                })
            }?;
            parent = pair.merge();
        }

        if parent == self.root {
            Ok(())
        } else {
            Err(PathSiblingsError::RootMismatch)
        }
    }
}

impl<'a, C: Mergeable + Clone> MatchedPairRef<'a, C> {
    /// Create a parent node by merging the 2 nodes in the pair.
    #[allow(dead_code)]
    fn merge(&self) -> Node<C> {
        Node {
            coord: Coordinate {
                y: self.left.0.coord.y + 1,
                x: self.left.0.coord.x / 2,
            },
            content: C::merge(&self.left.0.content, &self.right.0.content),
        }
    }
}

// ===========================================
// Unit tests.

#[cfg(test)]
mod tests {
    use super::*;

    use crate::binary_tree::test_utils::*;
    use crate::binary_tree::BinaryTree;

    fn check_path_siblings(
        tree: &BinaryTree<TestContent>,
        proof: &PathSiblings<TestContent>,
    ) {
        assert_eq!(tree.root, proof.root);
        assert_eq!(proof.siblings.len() as u8, tree.height - 1);
    }

    // fn check_tree()

    #[test]
    fn tree_works_for_full_base_layer() {
        let leaves = full_tree();
        let tree = BinaryTree::build_tree(leaves, 4, &get_padding_function())
            .expect("unable to build tree");

        let proof = tree
            .build_path_siblings(0)
            .expect("Path generation should have been successful");
        check_path_siblings(&tree, &proof);

        proof
            .verify()
            .expect("Path verification should have been successful");
    }

    #[test]
    fn proofs_work_for_sparse_leaves() {
        let leaves = tree_with_sparse_leaves();
        let tree = BinaryTree::build_tree(leaves, 4, &get_padding_function())
            .expect("Unable to build tree");

        let proof = tree
            .build_path_siblings(6)
            .expect("Path generation should have been successful");
        check_path_siblings(&tree, &proof);

        proof
            .verify()
            .expect("Path verification should have been successful");
    }

    #[test]
    fn proofs_work_for_single_leaf() {
        let leaves = vec![tree_with_single_leaf(0)];
        let tree = BinaryTree::build_tree(leaves, 4, &get_padding_function())
            .expect("Unable to build tree");

        let proof = tree
            .build_path_siblings(1)
            .expect("Path generation should have been successful");
        check_path_siblings(&tree, &proof);

        proof
            .verify()
            .expect("Path verification should have been successful");
    }
}
