//! Sparse binary tree implementation.
//!
//! TODO more docs

use ::std::collections::HashMap;
use ::std::fmt::Debug;
use std::str::FromStr;
use std::sync::Mutex;
use thiserror::Error;

/// Minimum tree height supported.
pub static MIN_HEIGHT: u8 = 2;

// ===========================================
// Main structs and constructor.

/// Fundamental structure of the tree, each element of the tree is a Node.
/// The data contained in the node is completely generic, requiring only to have an associated merge function.
#[derive(Clone, Debug, PartialEq)]
pub struct Node<C: Clone> {
    pub coord: Coordinate,
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
    pub y: u8, // from 0 to height
    // TODO this enforces a max tree height of 2^64 so we should make sure that is accounted for in other bits of the code, and make it easy to upgrade this max to something larger in the future
    pub x: u64, // from 0 to 2^y
}

/// Main data structure.
/// All nodes are stored in a hash map, their index in the tree being the key.
#[derive(Debug)]
#[allow(dead_code)]
pub struct SparseBinaryTree<C: Clone> {
    root: Node<C>,
    store: HashMap<Coordinate, Node<C>>,
    height: u8,
}

/// A simpler version of the Node struct that is used by the calling code to pass leaves to the tree constructor.
#[allow(dead_code)]
#[derive(Clone)]
pub struct InputLeafNode<C> {
    pub content: C,
    pub x_coord: u64,
}

// ===========================================
// Constructors.

fn find_split_index<C>(leaves: &Vec<InputLeafNode<C>>, x_coord_mid: u64) -> usize {
    let mut index = 0;
    while leaves
        .get(index)
        // TODO this default false is not good if it gets hit because that means there is a bug
        .map_or(false, |leaf| leaf.x_coord <= x_coord_mid)
    {
        index += 1;
    }
    index
}

use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

static thread_count: Mutex<u32> = Mutex::new(0);

// bit_map is going to be stored in u64 like so:
// if bit map is 1001 then u64 value will be 2^3 + 2^0
// and last index would be 3
pub fn dive<C: Clone + Mergeable + Send + 'static + Debug, F>(
    x_coord_min: u64,
    x_coord_max: u64,
    y: u8,
    height: u8,
    mut leaves: Vec<InputLeafNode<C>>,
    new_padding_node_content: Arc<F>,
) -> Node<C>
where
    F: Fn(&Coordinate) -> C + Send + 'static + Sync,
{
    assert!(leaves.len() <= 2usize.pow(y as u32));

    // println!("\nfunction call, num leaves {:?}", leaves.len());
    // println!("x_coord_min {:?}", x_coord_min);
    // println!("x_coord_max {:?}", x_coord_max);

    // base case: reached layer above leaves
    if y == 1 {
        // len should never reach 0
        // println!("base case reached");
        let pair = if leaves.len() == 2 {
            let left = LeftSibling::from_node(leaves.remove(0).to_node());
            let right = RightSibling::from_node(leaves.remove(0).to_node());
            MatchedPair { left, right }
        } else {
            let node = Sibling::from_node(leaves.remove(0).to_node());
            match node {
                Sibling::Left(left) => MatchedPair {
                    right: left.new_sibling_padding_node_2(new_padding_node_content),
                    left,
                },
                Sibling::Right(right) => MatchedPair {
                    left: right.new_sibling_padding_node_2(new_padding_node_content),
                    right,
                },
            }
        };

        return pair.merge();
    }

    let x_coord_mid = (x_coord_min + x_coord_max) / 2;
    // println!("x_coord_mid {}", x_coord_mid);
    // count the number of nodes that belong under the left child node
    let left_count = find_split_index(&leaves, x_coord_mid);
    // println!("left_count {}", left_count);

    // if count > 0 for 1st & 2nd half then spawn a new thread to go down the right node
    let pair = if 0 < left_count && left_count < leaves.len() {
        // println!("2 children");
        let right_leaves = leaves.split_off(left_count);
        let left_leaves = leaves;

        // let str = format!("x_coord_mid {} x_coord_max {}", x_coord_mid, x_coord_max);
        // let count = {
        //     let mut value = thread_count.lock().unwrap();
        //     *value += 1;
        //     // println!("STENT thread count {}", value);
        //     value.clone()
        // };

        let f = new_padding_node_content.clone();

        // for right child
        if y > height - 4 {
            let (tx, rx) = mpsc::channel();
            let builder = thread::Builder::new(); //.name(count.to_string());

            builder.spawn(move || {
                println!("thread spawned");
                let node = dive(x_coord_mid + 1, x_coord_max, y - 1, height, right_leaves, f);
                // println!("thread about to send, node {:?}", node);
                tx.send(RightSibling::from_node(node))
                    .map_err(|err| {
                        println!("ERROR STENT SEND {:?}", err);
                        err
                    })
                    .unwrap();
            });
            let left = LeftSibling::from_node(dive(
                x_coord_min,
                x_coord_mid,
                y - 1,
                height,
                left_leaves,
                new_padding_node_content,
            ));

            let right = rx
                .recv()
                .map_err(|err| {
                    println!("ERROR STENT REC {:?}", err);
                    err
                })
                .unwrap();

                MatchedPair { left, right }
        } else {
            let right = RightSibling::from_node(dive(
                x_coord_mid + 1,
                x_coord_max,
                y - 1,
                height,
                right_leaves,
                f,
            ));

            let left = LeftSibling::from_node(dive(
                x_coord_min,
                x_coord_mid,
                y - 1,
                height,
                left_leaves,
                new_padding_node_content,
            ));

            MatchedPair { left, right }
        }
    } else if left_count > 0 {
        // println!("left child");
        // go down left child
        let left = LeftSibling::from_node(dive(
            x_coord_min,
            x_coord_mid,
            y - 1,
            height,
            leaves,
            new_padding_node_content.clone(),
        ));
        let right = left.new_sibling_padding_node_2(new_padding_node_content);
        MatchedPair { left, right }
    } else {
        // println!("right child");
        // go down right child
        let right = RightSibling::from_node(dive(
            x_coord_mid + 1,
            x_coord_max,
            y - 1,
            height,
            leaves,
            new_padding_node_content.clone(),
        ));
        let left = right.new_sibling_padding_node_2(new_padding_node_content);
        MatchedPair { left, right }
    };

    pair.merge()
}

impl<C: Mergeable + Clone> SparseBinaryTree<C> {
    /// Create a new tree given the leaves, height and the padding node creation function.
    /// New padding nodes are given by a closure. Why a closure? Because creating a padding node may require context outside of this scope, where type C is defined, for example.
    // TODO there should be a warning if the height/leaves < min_sparsity (which was set to 2 in prev code)
    #[allow(dead_code)]
    pub fn new<F>(
        leaves: Vec<InputLeafNode<C>>,
        height: u8,
        new_padding_node_content: &F,
    ) -> Result<SparseBinaryTree<C>, SparseBinaryTreeError>
    where
        F: Fn(&Coordinate) -> C,
    {
        let max_leaves = num_bottom_layer_nodes(height);

        // construct a sorted vector of leaf nodes and perform parameter correctness checks
        let mut nodes = {
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
// Accessor methods
// TODO do we really need these getters? If Node is not going to be publicly an
//  available API for this crate then we can just make the struct keys public

impl<C: Clone> Node<C> {
    pub fn get_coord(&self) -> &Coordinate {
        &self.coord
    }
    pub fn get_x_coord(&self) -> u64 {
        self.coord.x
    }
    pub fn get_y_coord(&self) -> u8 {
        self.coord.y
    }
    pub fn get_content(&self) -> &C {
        &self.content
    }
}

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
    /// Attempt to find a bottom-layer leaf Node via it's x-coordinate in the underlying store.
    pub fn get_leaf_node(&self, x_coord: u64) -> Option<&Node<C>> {
        let coord = Coordinate { x: x_coord, y: 0 };
        self.get_node(&coord)
    }
}

// ===========================================
// Supporting structs, types and functions.

/// The maximum number of leaf nodes on the bottom layer of the binary tree.
/// TODO latex `max = 2^(height-1)`
pub fn num_bottom_layer_nodes(height: u8) -> u64 {
    2u64.pow(height as u32 - 1)
}

/// Used to organise nodes into left/right siblings.
pub enum NodeOrientation {
    Left,
    Right,
}

impl Coordinate {
    /// https://stackoverflow.com/questions/71788974/concatenating-two-u16s-to-a-single-array-u84
    pub fn as_bytes(&self) -> [u8; 32] {
        let mut c = [0u8; 32];
        let (left, mid) = c.split_at_mut(1);
        left.copy_from_slice(&self.y.to_le_bytes());
        let (mid, _right) = mid.split_at_mut(8);
        mid.copy_from_slice(&self.x.to_le_bytes());
        c
    }
}

impl<C: Clone> Node<C> {
    /// Return true if self is a) a left sibling and b) lives just to the left of the other node.
    pub fn is_left_sibling_of(&self, other: &Node<C>) -> bool {
        match self.node_orientation() {
            NodeOrientation::Left => {
                self.coord.y == other.coord.y && self.coord.x + 1 == other.coord.x
            }
            NodeOrientation::Right => false,
        }
    }

    /// Return true if self is a) a right sibling and b) lives just to the right of the other node.
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
    pub fn get_sibling_coord(&self) -> Coordinate {
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
    pub fn get_parent_coord(&self) -> Coordinate {
        Coordinate {
            y: self.coord.y + 1,
            x: self.coord.x / 2,
        }
    }
}

impl<C: Clone> Node<C> {
    pub fn convert<B: Clone + From<C>>(self) -> Node<B> {
        Node {
            content: self.content.into(),
            coord: self.coord,
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

/// Used to orient nodes inside a sibling pair so that the compiler can guarantee a left node is actually a left node.
pub enum Sibling<C: Clone> {
    Left(LeftSibling<C>),
    Right(RightSibling<C>),
}

/// Simply holds a Node under the designated 'LeftSibling' name.
pub struct LeftSibling<C: Clone>(Node<C>);

/// Simply holds a Node under the designated 'RightSibling' name.
pub struct RightSibling<C: Clone>(Node<C>);

/// A pair of sibling nodes, but one might be absent.
pub struct MaybeUnmatchedPair<C: Mergeable + Clone> {
    left: Option<LeftSibling<C>>,
    right: Option<RightSibling<C>>,
}

/// A pair of sibling nodes where both are present.
pub struct MatchedPair<C: Mergeable + Clone> {
    left: LeftSibling<C>,
    right: RightSibling<C>,
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
    fn new_sibling_padding_node_2<F>(&self, new_padding_node_content: Arc<F>) -> RightSibling<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.0.get_sibling_coord();
        let content = new_padding_node_content(&coord);
        let node = Node { coord, content };
        RightSibling(node)
    }
    fn from_node(node: Node<C>) -> Self {
        // TODO panic if node is not a left sibling
        Self(node)
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
    fn new_sibling_padding_node_2<F>(&self, new_padding_node_content: Arc<F>) -> LeftSibling<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.0.get_sibling_coord();
        let content = new_padding_node_content(&coord);
        let node = Node { coord, content };
        LeftSibling(node)
    }
    fn from_node(node: Node<C>) -> Self {
        // TODO panic if node is not a left sibling
        Self(node)
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
// Unit tests.

#[cfg(test)]
mod tests {
    // TODO test all edge cases where the first and last 2 nodes are either all present or all not or partially present
    // TODO write a test that checks the total number of nodes in the tree is correct

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
