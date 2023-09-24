//! TODO add module-level documentation
//! TODO add more detailed documentation for all public functions/structs

mod binary_tree_path;
mod dapol_node;
mod test_utils;

use self::binary_tree_path::{PathSiblings, PathSiblingsError};

use std::collections::HashMap;
use thiserror::Error;

// ===========================================
// Constants

/// Minimum tree height supported.
pub const MIN_HEIGHT: u8 = 2;

// ===========================================
// Errors

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum BinaryTreeError {
    #[error("Too many leaves for the given height")]
    TooManyLeaves,
    #[error("Must provide at least 1 leaf")]
    EmptyInput,
    #[error("X coords for leaves must be less than 2^height")]
    InvalidXCoord,
    #[error("Height cannot be smaller than {MIN_HEIGHT:?}")]
    HeightTooSmall,
    #[error("Not allowed to have more than 1 leaf with the same x-coord")]
    DuplicateLeaves,
}

// ===========================================
// Main types

/// The generic content type must implement this trait to allow 2 sibling nodes to be combined to make a new parent node.
pub trait Mergeable {
    fn merge(left_sibling: &Self, right_sibling: &Self) -> Self;
}

/// Fundamental structure of the tree, each element of the tree is a Node.
/// The data contained in the node is completely generic, requiring only to have an associated merge function.
#[derive(Clone, Debug, PartialEq)]
pub struct Node<C: Clone> {
    pub coord: Coordinate,
    pub content: C,
}

/// A simpler version of the Node struct that is used by the calling code to pass leaves to the tree constructor.
#[allow(dead_code)]
pub struct InputLeafNode<C> {
    pub content: C,
    pub x_coord: u64,
}

/// Used to identify the location of a Node
/// y is the vertical index (height) of the Node (0 being the bottom of the tree).
/// x is the horizontal index of the Node (0 being the leftmost index).
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Coordinate {
    pub y: u8, // from 0 to height
    // TODO change to 2^256
    pub x: u64, // from 0 to 2^y
}

/// Main data structure.
/// All nodes are stored in a hash map, their index in the tree being the key.
#[derive(Debug)]
#[allow(dead_code)]
pub struct BinaryTree<C: Clone> {
    root: Node<C>,
    store: HashMap<Coordinate, Node<C>>,
    pub height: u8,
}

// ===========================================
// Supporting types

/// Used to organise nodes into left/right siblings.
enum NodeOrientation {
    Left,
    Right,
}

/// Used to orient nodes inside a sibling pair so that the compiler can guarantee a left node is actually a left node.
enum Sibling<C: Clone> {
    Left(Node<C>),
    Right(Node<C>),
}

/// A pair of sibling nodes, but one might be absent.
struct MaybeUnmatchedPair<C: Mergeable + Clone> {
    left: Option<Node<C>>,
    right: Option<Node<C>>,
}
/// A pair of sibling nodes where both are present.
struct MatchedPair<C: Mergeable + Clone> {
    left: Node<C>,
    right: Node<C>,
}

// ===========================================
// Implementations

impl<C: Mergeable + Clone> Node<C> {
    /// New padding nodes are given by a closure. Why a closure? Because creating a padding node may require context outside of this scope, where type C is defined, for example.
    fn new_sibling_padding_node<F>(&self, new_padding_node_content: &F) -> Node<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.get_sibling_coord();
        let content = new_padding_node_content(&coord);
        Node { coord, content }
    }

    /// Return true if self is a) a left sibling and b) lives just to the left of the other node.
    #[allow(dead_code)]
    fn is_left_sibling_of(&self, other: &Node<C>) -> bool {
        match self.node_orientation() {
            NodeOrientation::Left => {
                self.coord.y == other.coord.y && self.coord.x + 1 == other.coord.x
            }
            NodeOrientation::Right => false,
        }
    }

    /// Return true if self is a) a right sibling and b) lives just to the right of the other node.
    fn is_right_sibling_of(&self, other: &Node<C>) -> bool {
        match self.node_orientation() {
            NodeOrientation::Left => false,
            NodeOrientation::Right => {
                self.coord.x > 0
                    && self.coord.y == other.coord.y
                    && self.coord.x - 1 == other.coord.x
            }
        }
    }

    fn convert<B: Clone + From<C>>(self) -> Node<B> {
        Node {
            content: self.content.into(),
            coord: self.coord,
        }
    }

    // ===========================================
    // Accessor methods

    /// Returns left if this node is a left sibling and vice versa for right.
    /// Since we are working with a binary tree we can tell if the node is a left sibling of the above layer by checking the x_coord modulus 2.
    /// Since x_coord starts from 0 we check if the modulus is equal to 0.
    fn node_orientation(&self) -> NodeOrientation {
        if self.coord.x % 2 == 0 {
            NodeOrientation::Left
        } else {
            NodeOrientation::Right
        }
    }

    /// Return the coordinates of this node's sibling, whether that be a right or a left sibling.
    fn get_sibling_coord(&self) -> Coordinate {
        match self.node_orientation() {
            NodeOrientation::Left => Coordinate {
                y: self.coord.y,
                x: self.coord.x + 1,
            },
            NodeOrientation::Right => Coordinate {
                y: self.coord.y,
                x: self.coord.x - 1,
            },
        }
    }

    /// Return the coordinates of this node's parent.
    /// The x-coord divide-by-2 works for both left _and_ right siblings because of truncation.
    /// Note that this function can be misused if tree height is not used to bound the y-coord from above.
    #[allow(dead_code)]
    fn get_parent_coord(&self) -> Coordinate {
        Coordinate {
            y: self.coord.y + 1,
            x: self.coord.x / 2,
        }
    }
}

impl<C: Clone> InputLeafNode<C> {
    /// Convert the simpler node type to the actual Node type.
    fn to_node(self) -> Node<C> {
        Node {
            content: self.content,
            coord: Coordinate {
                x: self.x_coord,
                y: 0,
            },
        }
    }
}

impl<C: Mergeable + Clone> BinaryTree<C> {
    /// Create a new tree given the leaves, height and the padding node creation function.
    /// New padding nodes are given by a closure. Why a closure? Because creating a padding node may require context outside of this scope, where type C is defined, for example.
    #[allow(dead_code)]
    pub fn build_tree<F>(
        leaves: Vec<InputLeafNode<C>>,
        height: u8,
        new_padding_node_content: F,
    ) -> Result<BinaryTree<C>, BinaryTreeError>
    where
        F: Fn(&Coordinate) -> C,
    {
        let mut store = HashMap::new();

        let mut nodes = get_nodes(leaves, height)?;
        let pairs = get_pairs(&nodes);

        for _i in 0..height - 1 {
            nodes = pairs
                .iter()
                .map(|pair| pair.to_matched_pair(&new_padding_node_content))
                .map(|matched_pair| {
                    let parent = matched_pair.merge();
                    store.insert(matched_pair.left.coord.clone(), matched_pair.left);
                    store.insert(matched_pair.right.coord.clone(), matched_pair.right);
                    parent
                })
                .collect();
        }

        // // construct a sorted vector of leaf nodes and perform parameter correctness checks
        // let mut nodes = {
        //     let max_leaves = 2u64.pow(height as u32 - 1);
        //     if leaves.len() as u64 > max_leaves {
        //         return Err(BinaryTreeError::TooManyLeaves);
        //     }

        //     if leaves.len() < 1 {
        //         return Err(BinaryTreeError::EmptyInput);
        //     }

        //     if height < MIN_HEIGHT {
        //         return Err(BinaryTreeError::HeightTooSmall);
        //     }

        //     // translate InputLeafNode to Node
        //     let mut nodes: Vec<Node<C>> = leaves.into_iter().map(|leaf| leaf.to_node()).collect();

        //     // sort by x_coord ascending
        //     nodes.sort_by(|a, b| a.coord.x.cmp(&b.coord.x));

        //     // make sure all x_coord < max
        //     if nodes.last().is_some_and(|node| node.coord.x >= max_leaves) {
        //         return Err(BinaryTreeError::InvalidXCoord);
        //     }

        //     // ensure no duplicates
        //     let duplicate_found = nodes
        //         .iter()
        //         .fold(
        //             (max_leaves, false),
        //             |(prev_x_coord, duplicate_found), node| {
        //                 if duplicate_found || node.coord.x == prev_x_coord {
        //                     (0, true)
        //                 } else {
        //                     (node.coord.x, false)
        //                 }
        //             },
        //         )
        //         .1;
        //     if duplicate_found {
        //         return Err(BinaryTreeError::DuplicateLeaves);
        //     }

        //     nodes
        // };

        // // repeat for each layer of the tree
        // for _i in 0..height - 1 {
        //     // create the next layer up of nodes from the current layer of nodes
        //     nodes = nodes
        //         .into_iter()
        //         // sort nodes into pairs (left & right siblings)
        //         .fold(Vec::<MaybeUnmatchedPair<C>>::new(), |mut pairs, node| {
        //             let sibling = Sibling::from_node(node);
        //             match sibling {
        //                 Sibling::Left(left_sibling) => pairs.push(MaybeUnmatchedPair {
        //                     left: Some(left_sibling.clone()),
        //                     right: Option::None,
        //                 }),
        //                 Sibling::Right(right_sibling) => {
        //                     let is_right_sibling_of_prev_node = pairs
        //                         .last_mut()
        //                         .map(|pair| (&pair.left).as_ref())
        //                         .flatten()
        //                         .is_some_and(|left| {
        //                             right_sibling.clone().is_right_sibling_of(&left)
        //                         });
        //                     if is_right_sibling_of_prev_node {
        //                         pairs
        //                             .last_mut()
        //                             // this case should never be reached because of the way is_right_sibling_of_prev_node is built
        //                             .expect("[Bug in tree constructor] Previous node not found")
        //                             .right = Option::Some(right_sibling.clone());
        //                     } else {
        //                         pairs.push(MaybeUnmatchedPair {
        //                             left: Option::None,
        //                             right: Some(right_sibling.clone()),
        //                         });
        //                     }
        //                 }
        //             }
        //             pairs
        //         })
        //         .into_iter()
        //         // add padding nodes to unmatched pairs
        //         .map(|pair| match (pair.left, pair.right) {
        //             (Some(left), Some(right)) => MatchedPair { left, right },
        //             (Some(left), None) => MatchedPair {
        //                 right: left.new_sibling_padding_node(&new_padding_node_content),
        //                 left,
        //             },
        //             (None, Some(right)) => MatchedPair {
        //                 left: right.new_sibling_padding_node(&new_padding_node_content),
        //                 right,
        //             },
        //             // if this case is reached then there is a bug in the above fold
        //             (None, None) => {
        //                 panic!("[Bug in tree constructor] Invalid pair (None, None) found")
        //             }
        //         })
        //         // create parents for the next loop iteration, and add the pairs to the tree store
        //         .map(|pair| {
        //             let parent = pair.merge();
        //             store.insert(pair.left.coord.clone(), pair.left);
        //             store.insert(pair.right.coord.clone(), pair.right);
        //             parent
        //         })
        //         .collect();
        // }

        // if the root node is not present then there is a bug in the above code
        let root = nodes
            .clone()
            .pop()
            .expect("[Bug in tree constructor] Unable to find root node");

        assert!(
            nodes.len() == 0,
            "[Bug in tree constructor] Should be no nodes left to process"
        );

        store.insert(root.coord.clone(), root.clone());

        Ok(BinaryTree {
            root,
            store,
            height,
        })
    }

    // TODO maybe we can compress the final result here to help keep the proof size as low as possible
    /// Construct the path for the leaf node living at the given x-coord on the bottom layer, and collect all the siblings to the path.
    #[allow(dead_code)]
    fn build_path_siblings(&self, leaf_x_coord: u64) -> Result<PathSiblings<C>, PathSiblingsError> {
        let coord = Coordinate {
            y: 0,
            x: leaf_x_coord,
        };

        let leaf = self
            .get_node(&coord)
            .ok_or(PathSiblingsError::LeafNotFound)?;

        let mut current_node = leaf;
        let mut siblings = Vec::<Node<C>>::new();

        for y in 0..self.height - 1 {
            let x = match current_node.node_orientation() {
                NodeOrientation::Left => current_node.coord.x + 1,
                NodeOrientation::Right => current_node.coord.x - 1,
            };

            let sibling_coord = Coordinate { y, x };

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
            root: self.root.clone(),
        })
    }

    // ===========================================
    // Accessor methods

    /// Attempt to find a Node via it's coordinate in the underlying store.
    fn get_node(&self, coord: &Coordinate) -> Option<&Node<C>> {
        self.store.get(coord)
    }
}

impl<C: Mergeable + Clone> Sibling<C> {
    /// Move a generic node into the left/right sibling type.
    fn from_node(node: Node<C>) -> Self {
        match node.node_orientation() {
            NodeOrientation::Left => Sibling::Left(node),
            NodeOrientation::Right => Sibling::Right(node),
        }
    }
}

impl<C: Mergeable + Clone> MaybeUnmatchedPair<C> {
    pub fn to_matched_pair<F>(&self, new_padding_node_content: &F) -> MatchedPair<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        match (&self.left, &self.right) {
            (Some(left), Some(right)) => MatchedPair {
                left: left.clone(),
                right: right.clone(),
            },
            (Some(left), None) => MatchedPair {
                right: left.new_sibling_padding_node(new_padding_node_content),
                left: left.clone(),
            },
            (None, Some(right)) => MatchedPair {
                left: right.new_sibling_padding_node(new_padding_node_content),
                right: right.clone(),
            },
            // if this case is reached then there is a bug in the above fold
            (None, None) => {
                panic!("[Bug in tree constructor] Invalid pair (None, None) found")
            }
        }
    }
}

impl<C: Mergeable + Clone> MatchedPair<C> {
    /// Create a parent node by merging the 2 nodes in the pair.
    fn merge(&self) -> Node<C> {
        Node {
            coord: Coordinate {
                y: self.left.coord.y + 1,
                x: self.left.coord.x / 2,
            },
            content: C::merge(&self.left.content, &self.right.content),
        }
    }
}

// Helpers

fn get_nodes<C: Clone>(
    leaves: Vec<InputLeafNode<C>>,
    height: u8,
) -> Result<Vec<Node<C>>, BinaryTreeError> {
    let max_leaves = 2u64.pow(height as u32 - 1);
    if leaves.len() as u64 > max_leaves {
        return Err(BinaryTreeError::TooManyLeaves);
    }

    if leaves.len() < 1 {
        return Err(BinaryTreeError::EmptyInput);
    }

    if height < MIN_HEIGHT {
        return Err(BinaryTreeError::HeightTooSmall);
    }

    // translate InputLeafNode to Node
    let mut nodes: Vec<Node<C>> = leaves.into_iter().map(|leaf| leaf.to_node()).collect();

    // sort by x_coord ascending
    nodes.sort_by(|a, b| a.coord.x.cmp(&b.coord.x));

    // make sure all x_coord < max
    if nodes.last().is_some_and(|node| node.coord.x >= max_leaves) {
        return Err(BinaryTreeError::InvalidXCoord);
    }

    // ensure no duplicates
    let duplicate_found = nodes
        .iter()
        .fold(
            (max_leaves, false),
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
        return Err(BinaryTreeError::DuplicateLeaves);
    }

    Ok(nodes)
}

fn get_pairs<C: Mergeable + Clone>(nodes: &Vec<Node<C>>) -> Vec<MaybeUnmatchedPair<C>> {
    let mut pairs: Vec<MaybeUnmatchedPair<C>> = Vec::new();

    for node in nodes {
        let sibling = Sibling::from_node(node.clone());
        match sibling {
            Sibling::Left(left_sibling) => pairs.push(MaybeUnmatchedPair {
                left: Some(left_sibling.clone()),
                right: Option::None,
            }),
            Sibling::Right(right_sibling) => {
                let is_right_sibling_of_prev_node = pairs
                    .last_mut()
                    .map(|pair| (&pair.left).as_ref())
                    .flatten()
                    .is_some_and(|left| right_sibling.clone().is_right_sibling_of(&left));
                if is_right_sibling_of_prev_node {
                    pairs
                        .last_mut()
                        // this case should never be reached because of the way is_right_sibling_of_prev_node is built
                        .expect("[Bug in tree constructor] Previous node not found")
                        .right = Option::Some(right_sibling.clone());
                } else {
                    pairs.push(MaybeUnmatchedPair {
                        left: Option::None,
                        right: Some(right_sibling.clone()),
                    });
                }
            }
        }
    }

    pairs
}

// ===========================================
// Unit tests.

#[cfg(test)]
mod tests {
    // TODO test all edge cases where the first and last 2 nodes are either all present or all not or partially present

    use super::test_utils::*;
    use super::*;

    use crate::testing_utils::assert_err;

    fn check_leaves(leaves: Vec<InputLeafNode<TestContent>>, length: usize) {
        assert_eq!(leaves.len(), length)
    }

    #[test]
    fn tree_works_for_full_base_layer() {
        let leaves = full_tree();
        check_leaves(leaves, 8);
    }

    #[test]
    fn tree_works_for_single_leaf() {
        let leaves = vec![tree_with_single_leaf(0)];
        check_leaves(leaves, 1);
    }

    #[test]
    fn tree_works_for_sparse_leaves() {
        let leaves = tree_with_sparse_leaves();
        check_leaves(leaves, 4);
    }

    #[test]
    fn too_many_leaf_nodes_gives_err() {
        let height = 4u8;

        let mut leaves = Vec::<InputLeafNode<TestContent>>::new();

        for i in 0..(2usize.pow(height as u32 - 1) + 1) {
            leaves.push(InputLeafNode::<TestContent> {
                x_coord: i as u64,
                content: TestContent {
                    hash: H256::default(),
                    value: i as u32,
                },
            });
        }

        let tree = BinaryTree::build_tree(leaves, height, &get_padding_function());
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

        let tree = BinaryTree::build_tree(
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

        let tree = BinaryTree::build_tree(vec![leaf_0], height, &get_padding_function());

        assert_err!(tree, Err(BinaryTreeError::HeightTooSmall));
    }
}
