//! Binary tree builder that utilizes parallelization to get the best build
//! time.
//!
//! The build algorithm starts from the root node and makes it's way down
//! to the bottom layer, splitting off a new thread at each junction.
//! A recursive function is used to do the traversal since every node above
//! the bottom layer can be viewed as the root node of a sub-tree of the main
//! tree. So every recursive iteration has an associated thread, root node that
//! needs building, and 2 child nodes that it will use to build the root node.
//! Construction of the child nodes is done using a recursive call. The base
//! case happens when a thread reaches a layer above the bottom layer, where the
//! children are the leaf nodes inputted by the original calling code.
//!
//! Because the tree is sparse not all of the paths to the bottom layer need
//! to be traversed--only those paths that will end in a bottom-layer leaf
//! node. At each junction a thread will first
//! determine if it needs to traverse either the left child, the right child
//! or both. If both then it will spawn a new thread to traverse the right
//! child before traversing the left itself, and if only left/right need to be
//! traversed then it will do so itself without spawning a new thread. Note that
//! children that do not need traversal are padding nodes, and are constructed
//! using the closure given by the calling code. Each
//! thread uses a sorted vector of bottom-layer leaf nodes to determine if a
//! child needs traversing: the idea is that at each recursive iteration the
//! vector should contain all the leaf nodes that will live at the bottom of
//! the sub-tree (no more and no less). The first iteration will have all the
//! input leaf nodes, and will split the vector between the left & right
//! recursive calls, each of which will split the vector to their children, etc.

use std::collections::HashMap;
use std::fmt::Debug;

use rayon::prelude::*;
use std::sync::Arc;
use std::thread;

use super::super::{
    num_bottom_layer_nodes, Coordinate, ErrOnSome, ErrUnlessTrue, LeftSibling, MatchedPair,
    Mergeable, Node, NodeOrientation, RightSibling, Sibling,
};
use super::{BinaryTree, TreeBuildError, TreeBuilder};

// -------------------------------------------------------------------------------------------------
// Main struct.

pub struct MultiThreadedBuilder<C, F>
where
    C: Clone,
{
    height: u8,
    leaf_nodes: Vec<Node<C>>,
    padding_node_generator: Option<F>,
}

/// Example:
/// ```
/// let tree = TreeBuilder::new()
///     .with_height(height)?
///     .with_leaf_nodes(leaf_nodes)?
///     .with_single_threaded_build_algorithm()?
///     .with_padding_node_generator(new_padding_node_content)
///     .build()?;
/// ```
/// The type traits on `C` & `F` are required for thread spawning.
impl<C, F> MultiThreadedBuilder<C, F>
where
    C: Debug + Clone + Mergeable + Send + 'static,
    F: Fn(&Coordinate) -> C + Send + Sync + 'static,
{
    /// Constructor for the builder, to be called by the [super][TreeBuilder].
    ///
    /// The leaf node vector is cleaned in the following ways:
    /// - sorted according to their x-coord
    /// - all x-coord <= max
    /// - checked for duplicates (duplicate if same x-coords)
    pub fn new(parent_builder: TreeBuilder<C>) -> Result<Self, TreeBuildError> {
        // Require certain fields to be set on the parent builder.
        let input_leaf_nodes = parent_builder
            .leaf_nodes
            .ok_or(TreeBuildError::NoLeafNodesProvided)?;
        let height = parent_builder
            .height
            .ok_or(TreeBuildError::NoHeightProvided)?;

        let max_leaf_nodes = num_bottom_layer_nodes(height);
        if input_leaf_nodes.len() as u64 > max_leaf_nodes {
            return Err(TreeBuildError::TooManyLeaves);
        }

        let leaf_nodes = {
            // Translate InputLeafNode to Node.
            let mut leaf_nodes: Vec<Node<C>> = input_leaf_nodes
                .into_par_iter()
                .map(|leaf| leaf.to_node())
                .collect();

            // Sort by x_coord ascending.
            leaf_nodes.par_sort_by(|a, b| a.coord.x.cmp(&b.coord.x));

            // Make sure all x_coord < max.
            leaf_nodes
                .last()
                .map(|node| node.coord.x < max_leaf_nodes)
                .err_unless_true(TreeBuildError::InvalidXCoord)?;

            // Ensure no duplicates.
            let i = leaf_nodes.iter();
            let i_plus_1 = {
                let mut i = leaf_nodes.iter();
                i.next();
                i
            };
            i.zip(i_plus_1)
                .find(|(prev, curr)| prev.coord.x == curr.coord.x)
                .err_on_some(TreeBuildError::DuplicateLeaves)?;

            leaf_nodes
        };

        Ok(MultiThreadedBuilder {
            height,
            leaf_nodes,
            padding_node_generator: None,
        })
    }

    /// New padding nodes are given by a closure. Why a closure? Because
    /// creating a padding node may require context outside of this scope, where
    /// type C is defined, for example.
    pub fn with_padding_node_generator(mut self, padding_node_generator: F) -> Self {
        self.padding_node_generator = Some(padding_node_generator);
        self
    }

    /// Construct the binary tree.
    pub fn build(self) -> Result<BinaryTree<C>, TreeBuildError> {
        let padding_node_generator = self
            .padding_node_generator
            .ok_or(TreeBuildError::NoPaddingNodeGeneratorProvided)?;

        let root = build_node(
            RecursionParams::new(self.height),
            self.leaf_nodes,
            Arc::new(padding_node_generator),
        );

        // TODO put some nodes in the store
        let store = HashMap::new();

        Ok(BinaryTree {
            root,
            store,
            height: self.height,
        })
    }
}

// -------------------------------------------------------------------------------------------------
// Supporting functions, structs, etc.

/// Returns the index `i` in `nodes` where `nodes[i].coord.x <= x_coord_mid`
/// but `nodes[i+1].coord.x > x_coord_mid`.
/// Requires `nodes` to be sorted according to the x-coord field.
/// If all nodes satisfy `node.coord.x <= mid` then `AllNodes` is returned.
/// If no nodes satisfy `node.coord.x <= mid` then `NoNodes` is returned.
// TODO can be optimized using a binary search
fn get_num_nodes_left_of<C: Clone>(x_coord_mid: u64, nodes: &Vec<Node<C>>) -> NumNodes {
    nodes
        .iter()
        .rposition(|leaf| leaf.coord.x <= x_coord_mid)
        .map_or(NumNodes::NoNodes, |index| {
            if index == nodes.len() - 1 {
                NumNodes::AllNodes
            } else {
                NumNodes::SomeNodes(index)
            }
        })
}

enum NumNodes {
    AllNodes,
    NoNodes,
    SomeNodes(usize),
}

/// Each node (except the root node) has a sibling. This is guaranteed by
/// the fact that we use a *full* binary tree. Each node is either a
/// left or a right sibling.
impl<C: Clone> LeftSibling<C> {
    /// New padding nodes are given by a closure. Why a closure? Because
    /// creating a padding node may require context outside of this scope, where
    /// type C is defined, for example.
    fn new_sibling_padding_node_arc<F>(&self, new_padding_node_content: Arc<F>) -> RightSibling<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.0.get_sibling_coord();
        let content = new_padding_node_content(&coord);
        let node = Node { coord, content };
        RightSibling(node)
    }

    /// Create a new left sibling.
    ///
    /// Panic if the given node is not a left sibling node.
    /// Since this code is only used internally for tree construction, and this
    /// state is unrecoverable, panicking is the best option. It is a sanity
    /// check and should never actually happen unless code is changed.
    fn from_node(node: Node<C>) -> Self {
        // TODO change the name of this function: remove 'node'
        match node.orientation() {
            NodeOrientation::Right => panic!(
                "[bug in multi-threaded node builder] Given node was expected to be a left sibling"
            ),
            NodeOrientation::Left => Self(node),
        }
    }
}

/// Each node (except the root node) has a sibling. This is guaranteed by
/// the fact that we use a *full* binary tree. Each node is either a
/// left or a right sibling.
impl<C: Clone> RightSibling<C> {
    /// New padding nodes are given by a closure. Why a closure? Because
    /// creating a padding node may require context outside of this scope, where
    /// type C is defined, for example.
    fn new_sibling_padding_node_arc<F>(&self, new_padding_node_content: Arc<F>) -> LeftSibling<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.0.get_sibling_coord();
        let content = new_padding_node_content(&coord);
        let node = Node { coord, content };
        LeftSibling(node)
    }

    /// Create a new right sibling.
    ///
    /// Panic if the given node is not a right sibling node.
    /// Since this code is only used internally for tree construction, and this
    /// state is unrecoverable, panicking is the best option. It is a sanity
    /// check and should never actually happen unless code is changed.
    fn from_node(node: Node<C>) -> Self {
        match node.orientation() {
            NodeOrientation::Left => panic!(
                "[bug in multi-threaded node builder] Given node was expected to be a left sibling"
            ),
            NodeOrientation::Right => Self(node),
        }
    }
}

impl<C: Mergeable + Clone> MatchedPair<C> {
    /// Create a pair of left and right sibling nodes from only 1 node and the
    /// padding node generation function.
    ///
    /// This function is made to be used by multiple threads that share
    /// `new_padding_node_content`.
    fn from_node<F>(node: Node<C>, new_padding_node_content: Arc<F>) -> Self
    where
        C: Send + 'static,
        F: Fn(&Coordinate) -> C + Send + Sync + 'static,
    {
        let sibling = Sibling::from_node(node);
        match sibling {
            Sibling::Left(left) => MatchedPair {
                right: left.new_sibling_padding_node_arc(new_padding_node_content),
                left,
            },
            Sibling::Right(right) => MatchedPair {
                left: right.new_sibling_padding_node_arc(new_padding_node_content),
                right,
            },
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Build algorithm.

/// Parameters for the recursive build function.
///
/// `x_coord_mid` is used to split the leaves into left and right vectors.
/// Nodes in the left vector have x-coord <= mid, and
/// those in the right vector have x-coord > mid.
///
/// `max_thread_spawn_height` is there to prevent more threads being spawned
/// than there are cores to execute them. If too many threads are spawned then
/// the parallelization can actually be detrimental to the run-time.
#[derive(Clone)]
struct RecursionParams {
    pub x_coord_min: u64,
    pub x_coord_mid: u64,
    pub x_coord_max: u64,
    pub y: u8,
    pub height: u8,
    pub max_thread_spawn_height: u8,
}

impl RecursionParams {
    fn new(height: u8) -> Self {
        let x_coord_min = 0;
        // x-coords start from 0, hence the `- 1`.
        let x_coord_max = num_bottom_layer_nodes(height) - 1;
        let x_coord_mid = (x_coord_min + x_coord_max) / 2;
        // y-coords also start from 0, hence the `- 1`.
        let y = height - 1;
        // TODO should be a parameter
        let max_thread_spawn_height = height - 4;

        RecursionParams {
            x_coord_min,
            x_coord_mid,
            x_coord_max,
            y,
            height,
            max_thread_spawn_height,
        }
    }

    fn into_left_child(mut self) -> Self {
        self.x_coord_max = self.x_coord_mid;
        self.x_coord_mid = (self.x_coord_min + self.x_coord_max) / 2;
        self.y -= 1;
        self
    }

    fn into_right_child(mut self) -> Self {
        self.x_coord_min = self.x_coord_mid + 1;
        self.x_coord_mid = (self.x_coord_min + self.x_coord_max) / 2;
        self.y -= 1;
        self
    }
}

/// Recursive, multi-threaded function for building a node by exploring the tree
/// from top-to-bottom. See docs at the top of the file for an explanation of
/// how it works.
///
/// `x_coord_min` and `x_coord_max` are the bounds of the sub-tree with respect
/// to the x-coords of the bottom layer of the main tree. Thus
/// `x_coord_max - x_coord_min - 1` will always be a power of 2. Example: if you
/// have a tree with a height of 5 then its bottom layer nodes will have
/// x-coord ranging from 0 to 15 (min & max), and the sub-tree whose root node
/// is the right child of the main tree's root node will have leaf nodes whose
/// x-coords range from 8 to 15 (min & max).
///
/// `height` is a natural number (1 onwards), while `y` is a counting number (0
/// onwards). `height` represents the height of the whole tree, while `y` is
/// is the height of the sub-tree associated with a specific recursive
/// iteration.
///
/// `leaves` must be sorted according to the nodes' x-coords. There is no panic
/// protection that checks for this.
///
/// Node length should never exceed the max number of bottom-layer nodes for a
/// sub-tree with height `y` since this means there are more nodes than can fit
/// into the sub-tree. Similarly, node length should never reach 0 since that
/// means we did not need do any work for this sub-tree but we entered the
/// function anyway. If either case is reached then either there is a bug in the
/// original calling code or there is a bug in the splitting algorithm in this
/// function. There is no recovery from these 2 states so we panic.
fn build_node<C, F>(
    params: RecursionParams,
    mut leaves: Vec<Node<C>>,
    new_padding_node_content: Arc<F>,
) -> Node<C>
where
    C: Debug + Clone + Mergeable + Send + 'static,
    F: Fn(&Coordinate) -> C + Send + Sync + 'static,
{
    {
        let max_nodes = num_bottom_layer_nodes(params.y + 1);
        assert!(
            leaves.len() <= max_nodes as usize,
            "[bug in multi-threaded node builder] Leaf node count ({}) exceeds layer max node number ({})",
            leaves.len(),
            max_nodes
        );

        assert_ne!(
            leaves.len(),
            0,
            "[bug in multi-threaded node builder] Leaf node length cannot be 0"
        );

        assert!(
            params.x_coord_min % 2 == 0,
            "[bug in multi-threaded node builder] x_coord_min ({}) must be a multiple of 2 or 0",
            params.x_coord_min
        );

        assert!(
            params.x_coord_max % 2 == 1,
            "[bug in multi-threaded node builder] x_coord_max ({}) must not be a multiple of 2",
            params.x_coord_max
        );

        let v = params.x_coord_max - params.x_coord_min + 1;
        assert!(
            (v & (v - 1)) == 0,
            "[bug in multi-threaded node builder] x_coord_max - x_coord_min + 1 ({}) must be a power of 2",
            v
        );
    }

    // Base case: reached the 2nd-to-bottom layer.
    // There are either 2 or 1 leaves left (which is checked above).
    if params.y == 1 {
        let pair = if leaves.len() == 2 {
            let left = LeftSibling::from_node(leaves.remove(0));
            let right = RightSibling::from_node(leaves.remove(0));
            MatchedPair { left, right }
        } else {
            MatchedPair::from_node(leaves.remove(0), new_padding_node_content)
        };

        return pair.merge();
    }

    let pair = match get_num_nodes_left_of(params.x_coord_mid, &leaves) {
        NumNodes::SomeNodes(index) => {
            let right_leaves = leaves.split_off(index + 1);
            let left_leaves = leaves;

            let new_padding_node_content_ref = Arc::clone(&new_padding_node_content);

            // Split off a thread to build the right child, but only do this if we are above
            // a certain height otherwise we are at risk of spawning too many threads.
            if params.y > params.max_thread_spawn_height {
                let params_clone = params.clone();
                let right_handler = thread::spawn(move || -> RightSibling<C> {
                    // TODO get rid of this after testing
                    println!("thread spawned");
                    let node = build_node(
                        params_clone.into_right_child(),
                        right_leaves,
                        new_padding_node_content_ref,
                    );
                    RightSibling::from_node(node)
                });

                let left = LeftSibling::from_node(build_node(
                    params.into_left_child(),
                    left_leaves,
                    new_padding_node_content,
                ));

                // If there is a problem joining onto the thread then there is no way to recover
                // so panic.
                let right = right_handler.join().expect(
                    "[bug in multi-threaded node builder] Couldn't join on the associated thread",
                );

                MatchedPair { left, right }
            } else {
                let right = RightSibling::from_node(build_node(
                    params.clone().into_right_child(),
                    right_leaves,
                    new_padding_node_content_ref,
                ));

                let left = LeftSibling::from_node(build_node(
                    params.into_left_child(),
                    left_leaves,
                    new_padding_node_content,
                ));

                MatchedPair { left, right }
            }
        }
        NumNodes::AllNodes => {
            // Go down left child only (there are no leaves living on the right side).
            let left = LeftSibling::from_node(build_node(
                params.into_left_child(),
                leaves,
                new_padding_node_content.clone(),
            ));
            let right = left.new_sibling_padding_node_arc(new_padding_node_content);
            MatchedPair { left, right }
        }
        NumNodes::NoNodes => {
            // Go down right child only (there are no leaves living on the left side).
            let right = RightSibling::from_node(build_node(
                params.into_right_child(),
                leaves,
                new_padding_node_content.clone(),
            ));
            let left = right.new_sibling_padding_node_arc(new_padding_node_content);
            MatchedPair { left, right }
        }
    };

    pair.merge()
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

// TODO
