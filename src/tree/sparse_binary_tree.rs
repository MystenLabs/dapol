use ::std::collections::HashMap;
use ::std::fmt::Debug;
use thiserror::Error;

use super::*;

// STENT TODO possibly use concurrency for tree construction

/// Minimum tree height supported.
pub static MIN_HEIGHT: u8 = 2;

// ===========================================
// Main structs and constructor.

/// Fundamental structure of the tree, each element of the tree is a Node.
/// The data contained in the node is completely generic, requiring only to have an associated merge function.
#[derive(Clone, Debug, PartialEq)]
pub struct Node<C: Clone> {
    // STENT TODO this should not be public but is needed to be used in paths file
    pub coord: Coordinate,
    // STENT TODO this should not be public but is needed to be used in paths file
    pub content: C,
}

/// The generic content type must implement this trait to allow 2 sibling nodes to be combined to make a new parent node.
pub trait Mergeable {
    fn merge(left_sibling: &Self, right_sibling: &Self) -> Self;
}

/// Used to identify the location of a Node
/// y is the vertical index (height) of the Node (0 being the bottom of the tree).
/// x is the horizontal index of the Node (0 being the leftmost index).
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Coordinate {
    // STENT TODO make these bounded, which depends on tree height
    // STENT TODO this should not be public but is needed to be used in paths file
    pub y: u8, // from 0 to height
    // STENT TODO change to 2^256
    // STENT TODO this should not be public but is needed to be used in paths file
    pub x: u64, // from 0 to 2^y
}

/// Main data structure.
/// All nodes are stored in a hash map, their index in the tree being the key.
// STENT TODO change height to u8
#[derive(Debug)]
#[allow(dead_code)]
pub struct SparseBinaryTree<C: Clone> {
    pub root: Node<C>,
    // STENT TODO this should not be public but is needed to be used in paths file
    pub store: HashMap<Coordinate, Node<C>>,
    pub height: u8,
}

/// A simpler version of the Node struct that is used by the calling code to pass leaves to the tree constructor.
#[allow(dead_code)]
pub struct InputLeafNode<C> {
    pub content: C,
    pub x_coord: u64,
}

impl<C: Mergeable + Default + Clone> SparseBinaryTree<C> {
    /// Create a new tree given the leaves, height and the padding node creation function.
    /// New padding nodes are given by a closure. Why a closure? Because creating a padding node may require context outside of this scope, where type C is defined, for example.
    #[allow(dead_code)]
    pub fn new<F>(
        leaves: Vec<InputLeafNode<C>>,
        height: u8,
        new_padding_node_content: &F,
    ) -> Result<SparseBinaryTree<C>, SparseBinaryTreeError>
    where
        F: Fn(&Coordinate) -> C,
    {
        // construct a sorted vector of leaf nodes and perform parameter correctness checks
        let mut nodes = {
            let max_leaves = 2u64.pow(height as u32 - 1);
            if leaves.len() as u64 > max_leaves {
                return Err(SparseBinaryTreeError::TooManyLeaves);
            }

            if leaves.len() < 1 {
                return Err(SparseBinaryTreeError::EmptyInput);
            }

            if height < MIN_HEIGHT {
                return Err(SparseBinaryTreeError::HeightTooSmall);
            }

            // translate InputLeafNode to Node
            let mut nodes: Vec<Node<C>> = leaves.into_iter().map(|leaf| leaf.to_node()).collect();

            // sort by x_coord ascending
            nodes.sort_by(|a, b| a.coord.x.cmp(&b.coord.x));

            // make sure all x_coord < max
            if nodes.last().is_some_and(|node| node.coord.x >= max_leaves) {
                return Err(SparseBinaryTreeError::InvalidXCoord);
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
                return Err(SparseBinaryTreeError::DuplicateLeaves);
            }

            nodes
        };

        let mut store = HashMap::new();

        // repeat for each layer of the tree
        for _i in 0..height - 1 {
            // create the next layer up of nodes from the current layer of nodes
            nodes = nodes
                .into_iter()
                // sort nodes into pairs (left & right siblings)
                .fold(Vec::<MaybeUnmatchedPair<C>>::new(), |mut pairs, node| {
                    let sibling = Sibling::from_node(node);
                    match sibling {
                        Sibling::Left(left_sibling) => pairs.push(MaybeUnmatchedPair {
                            left: Some(left_sibling),
                            right: Option::None,
                        }),
                        Sibling::Right(right_sibling) => {
                            let is_right_sibling_of_prev_node = pairs
                                .last_mut()
                                .map(|pair| (&pair.left).as_ref())
                                .flatten()
                                .is_some_and(|left| right_sibling.0.is_right_sibling_of(&left.0));
                            if is_right_sibling_of_prev_node {
                                pairs
                                    .last_mut()
                                    // this case should never be reached because of the way is_right_sibling_of_prev_node is built
                                    .expect("[Bug in tree constructor] Previous node not found")
                                    .right = Option::Some(right_sibling);
                            } else {
                                pairs.push(MaybeUnmatchedPair {
                                    left: Option::None,
                                    right: Some(right_sibling),
                                });
                            }
                        }
                    }
                    pairs
                })
                .into_iter()
                // add padding nodes to unmatched pairs
                .map(|pair| match (pair.left, pair.right) {
                    (Some(left), Some(right)) => MatchedPair { left, right },
                    (Some(left), None) => MatchedPair {
                        right: left.new_sibling_padding_node(new_padding_node_content),
                        left,
                    },
                    (None, Some(right)) => MatchedPair {
                        left: right.new_sibling_padding_node(new_padding_node_content),
                        right,
                    },
                    // if this case is reached then there is a bug in the above fold
                    (None, None) => {
                        panic!("[Bug in tree constructor] Invalid pair (None, None) found")
                    }
                })
                // create parents for the next loop iteration, and add the pairs to the tree store
                .map(|pair| {
                    let parent = pair.merge();
                    // STENT TODO not sure if we can get rid of these clones
                    store.insert(pair.left.0.coord.clone(), pair.left.0);
                    store.insert(pair.right.0.coord.clone(), pair.right.0);
                    parent
                })
                .collect();
        }

        // if the root node is not present then there is a bug in the above code
        let root = nodes
            .pop()
            .expect("[Bug in tree constructor] Unable to find root node");

        assert!(
            nodes.len() == 0,
            "[Bug in tree constructor] Should be no nodes left to process"
        );

        store.insert(root.coord.clone(), root.clone());

        Ok(SparseBinaryTree {
            root,
            store,
            height,
        })
    }
}

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum SparseBinaryTreeError {
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
// Supporting structs, types and functions.

impl<C: Clone> Node<C> {
    /// Returns left if this node is a left sibling and vice versa for right.
    /// Since we are working with a binary tree we can tell if the node is a left sibling of the above layer by checking the x_coord modulus 2.
    /// Since x_coord starts from 0 we check if the modulus is equal to 0.
    // STENT TODO this should not be public but is needed to be used in paths file
    pub fn node_orientation(&self) -> NodeOrientation {
        if self.coord.x % 2 == 0 {
            NodeOrientation::Left
        } else {
            NodeOrientation::Right
        }
    }

    /// Return true if self is a) a left sibling and b) lives just to the left of the other node.
    // STENT TODO this should not be public but is needed to be used in paths file
    #[allow(dead_code)]
    pub fn is_left_sibling_of(&self, other: &Node<C>) -> bool {
        match self.node_orientation() {
            NodeOrientation::Left => {
                self.coord.y == other.coord.y && self.coord.x + 1 == other.coord.x
            }
            NodeOrientation::Right => false,
        }
    }

    /// Return true if self is a) a right sibling and b) lives just to the right of the other node.
    // STENT TODO this should not be public but is needed to be used in paths file
    pub fn is_right_sibling_of(&self, other: &Node<C>) -> bool {
        match self.node_orientation() {
            NodeOrientation::Left => false,
            NodeOrientation::Right => {
                self.coord.x > 0
                    && self.coord.y == other.coord.y
                    && self.coord.x - 1 == other.coord.x
            }
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
    // STENT TODO this should not be public but is needed to be used in paths file
    #[allow(dead_code)]
    pub fn get_parent_coord(&self) -> Coordinate {
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

impl<C: Clone> LeftSibling<C> {
    /// New padding nodes are given by a closure. Why a closure? Because creating a padding node may require context outside of this scope, where type C is defined, for example.
    fn new_sibling_padding_node<F>(&self, new_padding_node_content: &F) -> RightSibling<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.0.get_sibling_coord();
        let content = new_padding_node_content(&coord);
        let node = Node { coord, content };
        RightSibling(node)
    }
}

impl<C: Clone> RightSibling<C> {
    /// New padding nodes are given by a closure. Why a closure? Because creating a padding node may require context outside of this scope, where type C is defined, for example.
    fn new_sibling_padding_node<F>(&self, new_padding_node_content: &F) -> LeftSibling<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.0.get_sibling_coord();
        let content = new_padding_node_content(&coord);
        let node = Node { coord, content };
        LeftSibling(node)
    }
}

impl<C: Clone> Sibling<C> {
    /// Move a generic node into the left/right sibling type.
    fn from_node(node: Node<C>) -> Self {
        match node.node_orientation() {
            NodeOrientation::Left => Sibling::Left(LeftSibling(node)),
            NodeOrientation::Right => Sibling::Right(RightSibling(node)),
        }
    }
}

impl<C: Mergeable + Clone> MatchedPair<C> {
    /// Create a parent node by merging the 2 nodes in the pair.
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
// Inclusion proof generation and verification.

// STENT TODO maybe put all this inclusion stuff in a different module/file
//   what is the best practice here?

// ===========================================
// Unit tests.

#[cfg(test)]
mod tests {
    // STENT TODO test all edge cases where the first and last 2 nodes are either all present or all not or partially present

    use super::*;

    use super::super::test_utils::{
        tree_with_single_leaf, tree_with_sparse_leaves, TestContent, full_tree, H256, get_padding_function
    };

    use crate::testing_utils::assert_err;

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

        for i in 0..2usize.pow(height as u32 - 1) {
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

        for i in 0..(2usize.pow(height as u32 - 1) + 1) {
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
}
