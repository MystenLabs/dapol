//! Sparse binary tree implementation.
//!
//! A sparse binary tree is a binary tree that is *full* but not necessarily
//! *complete* or *perfect* (the definitions of which are taken from the
//! [Wikipedia entry on binary trees](https://en.wikipedia.org/wiki/Binary_tree#Types_of_binary_trees)).
//!
//! The definition given in appendix C.2 (Accumulators) in the DAPOL+ paper
//! defines a Sparse Merkle Tree (SMT) as being a Merkle tree that is *full* but
//! not necessarily *complete* or *perfect*: "In an SMT, entities are mapped to
//! and reside in nodes at height 𝐻. Instead of constructing a full binary tree,
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
//! A node is defined by it's index in the tree, which is an `(x, y)`
//! coordinate. Both `x` & `y` start from 0, `x` increasing from left to right,
//! and `y` increasing from bottom to top. The height of the tree is thus
//! `max(y)+1`. The inputted leaves used to construct the tree must contain the
//! `x` coordinate (their `y` coordinate will be 0).

use serde::{Deserialize, Serialize};
use std::fmt;

mod tree_builder;
pub use tree_builder::{
    multi_threaded, single_threaded, InputLeafNode, TreeBuildError, TreeBuilder, MIN_STORE_DEPTH,
};

mod path_builder;
pub use path_builder::{Path, PathBuildError, PathError};

mod utils;
pub use utils::max_bottom_layer_nodes;

mod height;
pub use height::{Height, MAX_HEIGHT, MIN_HEIGHT};

use crate::utils::ErrOnSome;

/// Minimum recommended empty-space-to-leaf-node ratio.
///
/// The ratio of max number of bottom-layer nodes to the actual number of leaf
/// nodes given to the protocol is known as *sparsity*.

/// The whole reason a sparse
/// binary tree is used is to help hide the total number of users of the
/// exchange, since the max number of bottom-layer nodes can be calculated
/// from an inclusion proof (giving an upper bound on the number of users).
/// The greater the sparsity the greater the upper bound and the better
/// the total is hidden.
///
/// It is not recommended to have less sparsity than 2 because this means the
/// upper bound is exactly double the actual number.
pub const MIN_RECOMMENDED_SPARSITY: u8 = 2;

// -------------------------------------------------------------------------------------------------
// Main structs.

/// Main data structure.
///
/// The root node and height are important and get their own fields. The other
/// nodes in the tree are not all guaranteed to be stored, nor do we restrict
/// the data-structure used to store them. All non-padding bottom-layer leaf
/// nodes are guaranteed to be stored, but the rest of the nodes are stored
/// according to logic in [tree_builder].
///
/// The generic type `C` is for the content contained within each node.
#[derive(Serialize, Deserialize)]
pub struct BinaryTree<C> {
    root: Node<C>,
    store: Store<C>,
    height: Height,
}

/// Fundamental structure of the tree, each element of the tree is a Node.
/// The data contained in the node is completely generic, requiring only to have
/// an associated merge function.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node<C> {
    pub coord: Coordinate,
    pub content: C,
}

/// Index of a [Node] in the tree.
///
/// `y` is the vertical index of the [Node] with a range of
/// `[0, height)`.
///
/// `x` is the horizontal index of the [Node] with a range of
/// `[0, 2^y]`
#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct Coordinate {
    pub y: u8,
    pub x: height::XCoord,
}

/// Enum representing the different types of stores. Ideally this should be a
/// trait and [BinaryTree] would use the Box + dyn pattern for the store field
/// but this pattern cannot be deserialized. The best tools available to do this
/// are [erased_serde] and [typetag] but none support deserialization of generic
/// traits; for more details see
/// [this issue](https://github.com/dtolnay/typetag/issues/1).
#[derive(Serialize, Deserialize)]
pub enum Store<C> {
    MultiThreadedStore(multi_threaded::DashMapStore<C>),
    SingleThreadedStore(single_threaded::HashMapStore<C>),
}

/// The generic content type of a [Node] must implement this trait to allow 2
/// sibling nodes to be combined to make a new parent node.
pub trait Mergeable {
    fn merge(left_sibling: &Self, right_sibling: &Self) -> Self;
}

// -------------------------------------------------------------------------------------------------
// Accessor methods.

impl<C: Clone> BinaryTree<C> {
    pub fn height(&self) -> &Height {
        &self.height
    }

    pub fn root(&self) -> &Node<C> {
        &self.root
    }

    /// Attempt to find a node in the store via it's coordinate.
    ///
    /// If the store does not contain a node with the given coordinate then
    /// there are 2 possible reasons:
    /// 1. The node was left out by the builder to save space
    /// 2. The coordinate parameter is outside the bounds of the tree
    ///
    /// Implementations of this function may clone the node so it's not advised
    /// to call this if efficiency is required. A reference to the node
    /// cannot be returned in the multi-threaded case because the store
    /// implementation there uses a custom reference type and we do not want
    /// to expose that custom type to the outside calling code.
    pub fn get_node(&self, coord: &Coordinate) -> Option<Node<C>> {
        self.store.get_node(coord)
    }

    /// Attempt to find a bottom-layer leaf Node via it's x-coordinate in the
    /// underlying store.
    ///
    /// If the store does not contain a node with the given coordinate then
    /// there are 2 possible reasons:
    /// 1. The node was left out by the builder to save space
    /// 2. The x-coord parameter is outside the bounds of the tree
    ///
    /// Implementations of this function may clone the node so it's not advised
    /// to call this if efficiency is required. A reference to the node
    /// cannot be returned in the multi-threaded case because the store
    /// implementation there uses a custom reference type and we do not want
    /// to expose that custom type to the outside calling code.
    pub fn get_leaf_node(&self, x_coord: u64) -> Option<Node<C>> {
        let coord = Coordinate { x: x_coord, y: 0 };
        self.get_node(&coord)
    }
}

// -------------------------------------------------------------------------------------------------
// Implementations.

impl Coordinate {
    // TODO 256 bits is not the min space required, 8 + 64 = 72 bits is. So we could
    // decrease the length of the returned byte array.
    /// Copy internal data and return as bytes.
    ///
    /// [Coordinate] is encoded into a 256-bit storage space, using a byte
    /// array. The y-coord takes up a byte only, so it is placed at the
    /// beginning of the array. The x-coord takes up 8 bytes and it occupies
    /// the next 8 elements of the array, directly after the first element.
    /// Both x- & y-coords are given in Little Endian byte order.
    /// https://stackoverflow.com/questions/71788974/concatenating-two-u16s-to-a-single-array-u84
    pub fn as_bytes(&self) -> [u8; 32] {
        let mut c = [0u8; 32];
        let (left, mid) = c.split_at_mut(1);
        left.copy_from_slice(&self.y.to_le_bytes());
        let (mid, _right) = mid.split_at_mut(8);
        mid.copy_from_slice(&self.x.to_le_bytes());
        c
    }

    /// Returns left if a node with this coord is a left sibling and vice versa
    /// for right.
    ///
    /// Since we are working with a binary tree we can tell if the node is a
    /// left sibling of the above layer by checking the x-coord modulus 2.
    /// Since x-coord starts from 0 we check if the modulus is equal to 0.
    fn orientation(&self) -> NodeOrientation {
        if self.x % 2 == 0 {
            NodeOrientation::Left
        } else {
            NodeOrientation::Right
        }
    }

    /// Return the coordinates of the node that would be a sibling to the node
    /// with coordinates equal to `self`, whether that be a right or a left
    /// sibling.
    fn sibling_coord(&self) -> Coordinate {
        let x = match self.orientation() {
            NodeOrientation::Left => self.x + 1,
            NodeOrientation::Right => self.x - 1,
        };
        Coordinate { y: self.y, x }
    }

    /// Return the coordinates of the parent to the node that has this
    /// coordinate. The x-coord divide-by-2 works for both left _and_ right
    /// siblings because of truncation. Note that this function can be
    /// misused if tree height is not used to bound the y-coord from above.
    fn parent_coord(&self) -> Coordinate {
        Coordinate {
            y: self.y + 1,
            x: self.x / 2,
        }
    }

    /// Returns the x-coords of the first and last bottom-layer leaf nodes for
    /// the subtree with this coordinate as the root node.
    ///
    /// `x_coord_min` is x-coord for the first leaf.
    /// `x_coord_max` is the x-coord for the last leaf.
    ///
    /// Note that the calculation used to get the x-coords does not depend on
    /// the height of the main tree. This is due to the fact that we know the
    /// `x` value of the current coordinate. The `x` encodes for the main tree
    /// height.
    fn subtree_x_coord_bounds(&self) -> (u64, u64) {
        // This is essentially the number of bottom-layer leaf nodes for the
        // subtree, but shifted right to account for the subtree's position
        // in the main tree.
        let first_leaf_x_coord = |x: u64, y: u8| 2u64.pow(y as u32) * x;

        let x_coord_min = first_leaf_x_coord(self.x, self.y);
        let x_coord_max = first_leaf_x_coord(self.x + 1, self.y) - 1;

        (x_coord_min, x_coord_max)
    }

    /// Return the height for the coordinate.
    /// Why the offset? `y` starts from 0 but height starts from 1.
    fn to_height(&self) -> Height {
        Height::from(self.y + 1)
    }

    /// Generate a new bottom-layer leaf coordinate from the given x-coord.
    fn bottom_layer_leaf_from(x_coord: u64) -> Self {
        Coordinate { x: x_coord, y: 0 }
    }
}

impl<C> Node<C> {
    /// Returns left if this node is a left sibling and vice versa for right.
    /// Since we are working with a binary tree we can tell if the node is a
    /// left sibling of the above layer by checking the x_coord modulus 2.
    /// Since x_coord starts from 0 we check if the modulus is equal to 0.
    fn orientation(&self) -> NodeOrientation {
        self.coord.orientation()
    }

    /// Return true if self is a) a left sibling and b) lives just to the left
    /// of the other node.
    fn is_left_sibling_of(&self, other: &Node<C>) -> bool {
        match self.orientation() {
            NodeOrientation::Left => {
                self.coord.y == other.coord.y && self.coord.x + 1 == other.coord.x
            }
            NodeOrientation::Right => false,
        }
    }

    /// Return true if self is a) a right sibling and b) lives just to the right
    /// of the other node.
    fn is_right_sibling_of(&self, other: &Node<C>) -> bool {
        match self.orientation() {
            NodeOrientation::Left => false,
            NodeOrientation::Right => {
                self.coord.x > 0
                    && self.coord.y == other.coord.y
                    && self.coord.x - 1 == other.coord.x
            }
        }
    }

    /// Return the coordinates of this node's sibling, whether that be a right
    /// or a left sibling.
    fn sibling_coord(&self) -> Coordinate {
        self.coord.sibling_coord()
    }

    /// Return the coordinates of this node's parent.
    /// The x-coord divide-by-2 works for both left _and_ right siblings because
    /// of truncation. Note that this function can be misused if tree height
    /// is not used to bound the y-coord from above.
    fn parent_coord(&self) -> Coordinate {
        self.coord.parent_coord()
    }

    /// Convert a `Node<C>` to a `Node<B>`.
    fn convert<B: From<C>>(self) -> Node<B> {
        Node {
            content: self.content.into(),
            coord: self.coord,
        }
    }
}

impl<C: Clone> Store<C> {
    /// Simply delegate the call to the wrapped store.
    fn get_node(&self, coord: &Coordinate) -> Option<Node<C>> {
        match self {
            Store::MultiThreadedStore(store) => store.get_node(coord),
            Store::SingleThreadedStore(store) => store.get_node(coord),
        }
    }
}

impl<C: fmt::Debug + Clone> fmt::Debug for BinaryTree<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "root: {:?}, height: {:?}", self.root, self.height)
    }
}

// -------------------------------------------------------------------------------------------------
// Supporting structs & implementations.

/// Used to organise nodes into left/right siblings.
#[derive(Debug, PartialEq)]
enum NodeOrientation {
    Left,
    Right,
}

/// Used to orient nodes inside a sibling pair so that the compiler can
/// guarantee a left node is actually a left node.
enum Sibling<C> {
    Left(Node<C>),
    Right(Node<C>),
}

// TODO we should have a `from` function for this with an error check, just to
// be extra careful
/// A pair of sibling nodes.
struct MatchedPair<C> {
    left: Node<C>,
    right: Node<C>,
}

impl<C> Sibling<C> {
    /// Move a generic node into the left/right sibling type.
    fn from_node(node: Node<C>) -> Self {
        match node.orientation() {
            NodeOrientation::Left => Sibling::Left(node),
            NodeOrientation::Right => Sibling::Right(node),
        }
    }
}

impl<C: Mergeable> MatchedPair<C> {
    /// Create a parent node by merging the 2 nodes in the pair.
    fn merge(&self) -> Node<C> {
        Node {
            coord: self.left.parent_coord(),
            content: C::merge(&self.left.content, &self.right.content),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_tree::utils::test_utils::single_leaf;

    #[test]
    fn coord_byte_conversion_correct() {
        let x = 258;
        let y = 12;
        let coord = Coordinate { x, y };
        let bytes = coord.as_bytes();

        assert_eq!(bytes.len(), 32, "Byte array should be 256 bits");

        assert_eq!(
            bytes[0], y,
            "1st element of byte array should be equal to y-coord"
        );

        assert_eq!(
            bytes[1], 2,
            "2nd element of byte array should be equal to least significant byte of x-coord"
        ); // 256, x-coord

        assert_eq!(
            bytes[2], 1,
            "3rd element of byte array should be equal to most significant byte of x-coord"
        ); // 2, x-coord

        for item in bytes.iter().skip(3) {
            assert_eq!(
                *item, 0,
                "4th-last elements of byte array should be equal to 0"
            );
        }
    }

    // TODO repeat for Coordinate::orientation
    #[test]
    fn node_orientation_correctly_determined() {
        // TODO can fuzz on any even number
        let x_coord = 0;
        let left_node = single_leaf(x_coord).into_node();
        assert_eq!(left_node.orientation(), NodeOrientation::Left);

        // TODO can fuzz on any odd number
        let x_coord = 1;
        let right_node = single_leaf(x_coord).into_node();
        assert_eq!(right_node.orientation(), NodeOrientation::Right);
    }

    // TODO do for internal nodes
    // TODO fuzz on the one x-coord then calculate the other one from this
    #[test]
    fn is_sibling_of_works() {
        let height = Height::from(5);

        let x_coord = 16;
        let left_node = single_leaf(x_coord).into_node();
        let x_coord = 17;
        let right_node = single_leaf(x_coord).into_node();

        assert!(right_node.is_right_sibling_of(&left_node));
        assert!(!right_node.is_left_sibling_of(&left_node));
        assert!(left_node.is_left_sibling_of(&right_node));
        assert!(!left_node.is_right_sibling_of(&right_node));

        // check no other nodes trigger true for sibling check
        for i in 0..max_bottom_layer_nodes(&height) {
            let node = single_leaf(i).into_node();
            if left_node.coord.x != i && right_node.coord.x != i {
                assert!(!right_node.is_right_sibling_of(&node));
                assert!(!right_node.is_left_sibling_of(&node));
                assert!(!left_node.is_left_sibling_of(&node));
                assert!(!left_node.is_right_sibling_of(&node));
            }
        }
    }

    // TODO do for internal node
    // TODO do for root node
    // TODO fuzz on the x,y coord
    #[test]
    fn sibling_coord_calculated_correctly() {
        let x_coord = 5;
        let right_node = single_leaf(x_coord).into_node();
        let sibling_coord = right_node.sibling_coord();
        assert_eq!(
            sibling_coord.y, 0,
            "Sibling should be on the bottom layer (y-coord == 0)"
        );
        assert_eq!(sibling_coord.x, 4, "Sibling's x-coord should be 1 less than the node's x-coord because the node is a right sibling");

        let x_coord = 0;
        let left_node = single_leaf(x_coord).into_node();
        let sibling_coord = left_node.sibling_coord();
        assert_eq!(
            sibling_coord.y, 0,
            "Sibling should be on the bottom layer (y-coord == 0)"
        );
        assert_eq!(sibling_coord.x, 1, "Sibling's x-coord should be 1 more than the node's x-coord because the node is a left sibling");
    }

    // TODO repeat for Coordinate::parent_coord
    // TODO do for internal node
    // TODO do for root node
    // TODO fuzz on the x,y coord
    #[test]
    fn parent_coord_calculated_correctly() {
        let x_coord = 5;
        let right_node = single_leaf(x_coord).into_node();
        let right_parent_coord = right_node.parent_coord();

        let x_coord = 4;
        let left_node = single_leaf(x_coord).into_node();
        let left_parent_coord = left_node.parent_coord();

        assert_eq!(
            right_parent_coord, left_parent_coord,
            "Left and right siblings should have same parent coord"
        );
        assert_eq!(
            right_parent_coord.y, 1,
            "Parent's y-coord should be 1 more than child's"
        );
        assert_eq!(
            right_parent_coord.x, 2,
            "Parent's x-coord should be half the child's"
        );
    }

    // TODO fuzz on x-coord
    #[test]
    fn input_node_correctly_converted_into_node() {
        let x_coord = 5;
        let input_node = single_leaf(x_coord);
        let content = input_node.content.clone();
        let node = input_node.into_node();

        assert_eq!(
            node.coord.x, 5,
            "Node's x-coord should match input leaf node's"
        );
        assert_eq!(
            node.coord.y, 0,
            "Node's y-coord should be 0 because all input nodes are assumed to be on bottom layer"
        );
        assert_eq!(content, node.content);
    }

    // TODO fuzz on the x-coord, we just need to make sure the value is even or odd
    // depending on the case
    #[test]
    fn sibling_from_node_works() {
        let x_coord = 11;
        let right_node = single_leaf(x_coord).into_node();
        let sibling = Sibling::from_node(right_node);
        match sibling {
            Sibling::Left(_) => panic!("Node should be a right sibling"),
            Sibling::Right(_) => {}
        }

        let x_coord = 16;
        let left_node = single_leaf(x_coord).into_node();
        let sibling = Sibling::from_node(left_node);
        match sibling {
            Sibling::Right(_) => panic!("Node should be a left sibling"),
            Sibling::Left(_) => {}
        }
    }

    // TODO fuzz on the 1 x-coord then calculate the other one from this
    #[test]
    fn matched_pair_merge_works() {
        let x_coord = 17;
        let right = single_leaf(x_coord).into_node();

        let x_coord = 16;
        let left = single_leaf(x_coord).into_node();

        let pair = MatchedPair { left, right };
        let parent = pair.merge();

        assert_eq!(
            parent.coord.y, 1,
            "Parent's y-coord should be 1 more than child's"
        );
        assert_eq!(
            parent.coord.x, 8,
            "Parent's x-coord should be half the child's"
        );
    }

    #[test]
    fn subtree_bounds_works() {
        let coord = Coordinate { x: 2, y: 2 };
        let (lower, upper) = coord.subtree_x_coord_bounds();
        assert_eq!(lower, 8, "Incorrect lower x-coord bound for subtree");
        assert_eq!(upper, 11, "Incorrect upper x-coord bound for subtree");
    }
}
