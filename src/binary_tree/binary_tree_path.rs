use super::sparse_binary_tree::{Coordinate, Mergeable, Node, SparseBinaryTree, MIN_HEIGHT};
use ::std::fmt::Debug;
use thiserror::Error;

use super::*;

/// All the sibling nodes for a leaf node's path from bottom layer to root node.
#[allow(dead_code)]
pub struct PathSiblings<C: Clone> {
    leaf: Node<C>,
    siblings: Vec<Node<C>>,
    root: Node<C>,
}

impl<C: Mergeable + Default + Clone> SparseBinaryTree<C> {
    // TODO maybe we can compress the final result here to help keep the proof size as low as possible
    /// Construct the path for the leaf node living at the given x-coord on the bottom layer, and collect all the siblings to the path.
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
        let mut siblings = Vec::<Node<C>>::new();

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
            root: self.get_root().clone(),
        })
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
                    given: node.get_coord().clone(),
                    calculated: parent.get_coord().clone(),
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

// ===========================================
// Unit tests.

#[cfg(test)]
mod tests {
    use super::super::test_utils::{
        full_tree, tree_with_single_leaf, tree_with_sparse_leaves,
        TestContent,
    };
    use super::*;

    fn check_path_siblings(
        tree: &SparseBinaryTree<TestContent>,
        proof: &PathSiblings<TestContent>,
    ) {
        assert_eq!(tree.get_root(), &proof.root);
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
            .verify()
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
            .verify()
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
                .verify()
                .expect("Path verification should have been successful");
        }
    }
}
