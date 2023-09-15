//! Data structures and methods related to paths in a binary tree.
//!
//! A path in a binary tree goes from a leaf node to the root node. For each
//! node (starting from the leaf node) one follows the path by moving to the
//! parent node; since the root node has no parent this is the end of the path.
//!
//! A path is uniquely determined by the leaf node and only the leaf node. It
//! can thus be referred to as the leaf node's path.

use super::NodeOrientation;
use super::{BinaryTree, Coordinate, Mergeable, Node, MIN_HEIGHT};

use std::fmt::Debug;
use thiserror::Error;

/// All the sibling nodes for the nodes in a leaf node's path.
/// The nodes are ordered from bottom layer (first) to root node (last, not
/// included). The leaf node + the siblings can be used to reconstruct the
/// actual nodes in the path as well as the root node.
#[derive(Debug)]
pub struct Path<C: Clone> {
    pub leaf: Node<C>,
    pub siblings: Vec<Node<C>>,
}

// -------------------------------------------------------------------------------------------------
// Constructor

impl<C: Mergeable + Clone> BinaryTree<C> {
    /// Construct the path up the tree from the leaf node at the given x-coord
    /// on the bottom layer to the root node. Put all the sibling nodes for
    /// the path into a vector and use this vector to create a [Path] struct
    /// and return it. The vector is ordered from bottom layer (first)
    /// to root node (last, not included).
    pub fn build_path_for(&self, leaf_x_coord: u64) -> Result<Path<C>, PathError> {
        let coord = Coordinate {
            x: leaf_x_coord,
            y: 0,
        };

        let leaf = self.get_node(&coord).ok_or(PathError::LeafNotFound)?;

        let mut current_node = leaf;
        let mut siblings = Vec::with_capacity(self.get_height() as usize);

        for y in 0..self.get_height() - 1 {
            let x_coord = match current_node.orientation() {
                NodeOrientation::Left => current_node.coord.x + 1,
                NodeOrientation::Right => current_node.coord.x - 1,
            };

            let sibling_coord = Coordinate { x: x_coord, y };
            siblings.push(
                self.get_node(&sibling_coord)
                    .ok_or(PathError::NodeNotFound {
                        coord: sibling_coord,
                    })?
                    .clone(),
            );

            let parent_coord = current_node.get_parent_coord();
            current_node = self
                .get_node(&parent_coord)
                .ok_or(PathError::NodeNotFound {
                    coord: parent_coord,
                })?;
        }

        Ok(Path {
            leaf: leaf.clone(),
            siblings,
        })
    }
}

// -------------------------------------------------------------------------------------------------
// Methods for Path

/// Construct a [MatchedPairRef] using the 2 given nodes.
/// Only build the pair if the 2 nodes are siblings, otherwise return an error.
fn build_pair<'a, C: Mergeable + Clone>(
    node: &'a Node<C>,
    parent: &'a Node<C>,
) -> Result<MatchedPairRef<'a, C>, PathError> {
    if parent.is_right_sibling_of(node) {
        Ok(MatchedPairRef {
            left: LeftSiblingRef(node),
            right: RightSiblingRef(parent),
        })
    } else if parent.is_left_sibling_of(node) {
        Ok(MatchedPairRef {
            left: LeftSiblingRef(parent),
            right: RightSiblingRef(node),
        })
    } else {
        Err(PathError::InvalidSibling {
            node_that_needs_sibling: parent.coord.clone(),
            sibling_given: node.coord.clone(),
        })
    }
}

impl<C: Mergeable + Clone + PartialEq + Debug> Path<C> {
    /// Verify that the given list of sibling nodes + the base leaf node matches
    /// the given root node.
    ///
    /// This is done by reconstructing each node in the path, from bottom layer
    /// to the root, using the given leaf and sibling nodes, and then
    /// comparing the resulting root node to the given root node.
    ///
    /// An error is returned if the number of siblings is less than the min
    /// amount, or the constructed root node does not match the given one.
    pub fn verify(&self, root: &Node<C>) -> Result<(), PathError> {
        let mut parent = self.leaf.clone();

        if self.siblings.len() < MIN_HEIGHT as usize {
            return Err(PathError::TooFewSiblings);
        }

        for node in &self.siblings {
            let pair = build_pair(node, &parent)?;
            parent = pair.merge();
        }

        if parent == *root {
            Ok(())
        } else {
            Err(PathError::RootMismatch)
        }
    }

    /// Return a vector containing only the nodes the tree path.
    ///
    /// The path nodes have to be constructed using the leaf & sibling nodes in
    /// [Path] because they are not stored explicitly. The order of the
    /// returned path nodes is bottom first (leaf) and top last (root).
    ///
    /// An error is returned if the [Path] data is invalid.
    pub fn nodes_from_bottom_to_top(&self) -> Result<Vec<Node<C>>, PathError> {
        // +1 because the root node is included in the returned vector
        let mut nodes = Vec::<Node<C>>::with_capacity(self.siblings.len() + 1);

        nodes.push(self.leaf.clone());

        for node in &self.siblings {
            // this should never panic because we pushed the leaf node before the loop
            let parent = nodes
                .last()
                .expect("[Bug in path generation] Empty node vector");
            let pair = build_pair(node, parent)?;
            nodes.push(pair.merge());
        }

        Ok(nodes)
    }
}

// -------------------------------------------------------------------------------------------------
// Conversion

impl<C: Clone> Path<C> {
    /// Convert `Path<C>` to `Path<D>`.
    ///
    /// `convert` is called on each of the sibling nodes & leaf node.
    pub fn convert<B: Clone + From<C>>(self) -> Path<B> {
        Path {
            siblings: self
                .siblings
                .into_iter()
                .map(|node| node.convert())
                .collect(),
            leaf: self.leaf.convert(),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Errors

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum PathError {
    #[error("Provided leaf node not found in the tree")]
    LeafNotFound,
    #[error("Node not found in tree ({coord:?})")]
    NodeNotFound { coord: Coordinate },
    #[error("Calculated root content does not match provided root content")]
    RootMismatch,
    #[error("Provided node ({sibling_given:?}) is not a sibling of the calculated node ({node_that_needs_sibling:?})")]
    InvalidSibling {
        node_that_needs_sibling: Coordinate,
        sibling_given: Coordinate,
    },
    #[error("Too few siblings")]
    TooFewSiblings,
}

// -------------------------------------------------------------------------------------------------
// Helper structs

/// A reference to a left sibling node.
///
/// It is like [super][sparse_binary_tree][LeftSibling] but does not own the
/// underlying node. The purpose of this type is for efficiency gains over
/// [super][sparse_binary_tree][LeftSibling] when ownership of the Node type is
/// not needed.
#[allow(dead_code)]
struct LeftSiblingRef<'a, C: Clone>(&'a Node<C>);

/// A reference to a right sibling node.
///
/// It is like [super][sparse_binary_tree][RightSibling] but does not own the
/// underlying node. The purpose of this type is for efficiency gains over
/// [super][sparse_binary_tree][RightSibling] when ownership of the Node type is
/// not needed.
#[allow(dead_code)]
struct RightSiblingRef<'a, C: Clone>(&'a Node<C>);

/// A reference to a pair of left and right sibling nodes.
///
/// It is like [super][sparse_binary_tree][MatchedPair] but does not own the
/// underlying node. The purpose of this type is for efficiency gains over
/// [super][sparse_binary_tree][MatchedPair] when ownership of the Node type is
/// not needed.
#[allow(dead_code)]
struct MatchedPairRef<'a, C: Mergeable + Clone> {
    left: LeftSiblingRef<'a, C>,
    right: RightSiblingRef<'a, C>,
}

impl<'a, C: Mergeable + Clone> MatchedPairRef<'a, C> {
    /// Create a parent node by merging the 2 nodes in the pair.
    #[allow(dead_code)]
    fn merge(&self) -> Node<C> {
        Node {
            coord: Coordinate {
                x: self.left.0.coord.x / 2,
                y: self.left.0.coord.y + 1,
            },
            content: C::merge(&self.left.0.content, &self.right.0.content),
        }
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

    fn check_path_siblings(tree: &BinaryTree<TestContent>, proof: &Path<TestContent>) {
        assert_eq!(proof.siblings.len() as u8, tree.get_height() - 1);
    }

    #[test]
    fn tree_works_for_full_base_layer() {
        let (tree, _height) = full_tree();

        let proof = tree
            .build_path_for(0)
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
            .build_path_for(6)
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
                .build_path_for(i as u64)
                .expect("Path generation should have been successful");
            check_path_siblings(&tree, &proof);

            proof
                .verify(tree.get_root())
                .expect("Path verification should have been successful");
        }
    }
}
