use std::fmt::Debug;
use std::collections::HashMap;

use std::sync::Arc;
use std::thread;

use super::super::{
    num_bottom_layer_nodes, Coordinate, LeftSibling, MatchedPair, Mergeable, Node, NodeOrientation,
    RightSibling, Sibling,
};
use super::{TreeBuilder, TreeBuildError, BinaryTree};

// -------------------------------------------------------------------------------------------------
// Main struct.

pub struct MultiThreadedBuilder<C>
where
    C: Clone,
{
    height: u8,
    leaf_nodes: Vec<Node<C>>,
}

impl<C> MultiThreadedBuilder<C>
where
    C: Clone + Mergeable,
{
    pub fn new(parent_builder: TreeBuilder<C>) -> Result<Self, TreeBuildError> {
        // require certain fields to be set
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

        // TODO need to parallelize this, it's currently the same as the single-threaded
        // version Construct a sorted vector of leaf nodes and perform parameter
        // correctness checks.
        let mut leaf_nodes = {
            // Translate InputLeafNode to Node.
            let mut leaf_nodes: Vec<Node<C>> = input_leaf_nodes
                .into_iter()
                .map(|leaf| leaf.to_node())
                .collect();

            // Sort by x_coord ascending.
            leaf_nodes.sort_by(|a, b| a.coord.x.cmp(&b.coord.x));

            // Make sure all x_coord < max.
            if leaf_nodes
                .last()
                .is_some_and(|node| node.coord.x >= max_leaf_nodes)
            {
                return Err(TreeBuildError::InvalidXCoord);
            }

            // Ensure no duplicates.
            let duplicate_found = leaf_nodes
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

            leaf_nodes
        };

        Ok(MultiThreadedBuilder { height, leaf_nodes })
    }

    pub fn build<F>(self, padding_node_generator: F) -> Result<BinaryTree<C>, TreeBuildError>
    where
        C: Debug + Send + 'static,
        F: Fn(&Coordinate) -> C + Send + 'static + Sync,
    {
        let height = self.height;
        let x_coord_min = 0;
        let x_coord_max = 2u64.pow(height as u32 - 1) - 1;
        let y = height - 1;

        let root = build_node(
            x_coord_min,
            x_coord_max,
            y,
            height,
            self.leaf_nodes,
            Arc::new(padding_node_generator),
        );

        let store = HashMap::new();

        Ok(BinaryTree {
            root,
            store,
            height,
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
        match node.node_orientation() {
            NodeOrientation::Right => panic!("[bug in the ] not left node"),
            NodeOrientation::Left => Self(node),
        }
    }
}

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
        match node.node_orientation() {
            NodeOrientation::Left => panic!("TODO not right node"),
            NodeOrientation::Right => Self(node),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Build algorithm.

// TODO need more docs
/// Recursive multi-threaded function for building a node by exploring the tree
/// from top-to-bottom.
///
/// `leaves` must be sorted according to the nodes' x-coords. There is no panic
/// protection that checks for this.
///
/// Height is a natural number, while y is a counting number.
///
/// Node length should never exceed the max number of bottom-layer nodes for a
/// sub-tree with height `y` since this means there are more nodes than can fit
/// into the sub-tree. Similarly, node length should never reach 0 since that
/// means we did not need do any work for this sub-tree but we entered the
/// function anyway. If either case is reached then either there is a bug in the
/// original calling code or there is a bug in the splitting algorithm in this
/// function. There is no recovery from these 2 states so we panic.
pub fn build_node<C, F>(
    x_coord_min: u64,
    x_coord_max: u64,
    y: u8,
    height: u8,
    mut leaves: Vec<Node<C>>,
    new_padding_node_content: Arc<F>,
) -> Node<C>
where
    C: Debug + Clone + Mergeable + Send + 'static,
    F: Fn(&Coordinate) -> C + Send + Sync + 'static,
{
    // TODO maybe we should return a result instead of panicking for these asserts?
    {
        let max_nodes = num_bottom_layer_nodes(y + 1);
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
            x_coord_min % 2 == 0,
            "[bug in multi-threaded node builder] x_coord_min ({}) must be a multiple of 2 or 0",
            x_coord_min
        );

        assert!(
            x_coord_max % 2 == 1,
            "[bug in multi-threaded node builder] x_coord_max ({}) must not be a multiple of 2",
            x_coord_max
        );
    }

    // Base case: reached the 2nd-to-bottom layer.
    if y == 1 {
        let pair = if leaves.len() == 2 {
            let left = LeftSibling::from_node(leaves.remove(0));
            let right = RightSibling::from_node(leaves.remove(0));
            MatchedPair { left, right }
        } else {
            let node = Sibling::from_node(leaves.remove(0));
            match node {
                Sibling::Left(left) => MatchedPair {
                    right: left.new_sibling_padding_node_arc(new_padding_node_content),
                    left,
                },
                Sibling::Right(right) => MatchedPair {
                    left: right.new_sibling_padding_node_arc(new_padding_node_content),
                    right,
                },
            }
        };

        return pair.merge();
    }

    // This value is used to split the leaves into left and right.
    // Nodes in the left vector have x-coord <= mid, and
    // those in the right vector have x-coord > mid.
    let x_coord_mid = (x_coord_min + x_coord_max) / 2;

    let pair = match get_num_nodes_left_of(x_coord_mid, &leaves) {
        NumNodes::SomeNodes(index) => {
            let right_leaves = leaves.split_off(index + 1);
            let left_leaves = leaves;

            let new_padding_node_content_ref = Arc::clone(&new_padding_node_content);

            // Split off a thread to build the right child, but only do this if we are above
            // a certain height otherwise we are at risk of spawning too many threads.
            // TODO make this 4 a variable, actually we should make a struct that contains a
            // bunch of the static data not needed in every iteration of the recursion
            if y > height - 4 {
                let right_handler = thread::spawn(move || -> RightSibling<C> {
                    println!("thread spawned");
                    let node = build_node(
                        x_coord_mid + 1,
                        x_coord_max,
                        y - 1,
                        height,
                        right_leaves,
                        new_padding_node_content_ref,
                    );
                    RightSibling::from_node(node)
                });

                let left = LeftSibling::from_node(build_node(
                    x_coord_min,
                    x_coord_mid,
                    y - 1,
                    height,
                    left_leaves,
                    new_padding_node_content,
                ));

                // If there is a problem joining onto the thread then there is no way to recover
                // so panic.
                let right = right_handler
                    .join()
                    .expect("Couldn't join on the associated thread");

                MatchedPair { left, right }
            } else {
                let right = RightSibling::from_node(build_node(
                    x_coord_mid + 1,
                    x_coord_max,
                    y - 1,
                    height,
                    right_leaves,
                    new_padding_node_content_ref,
                ));

                let left = LeftSibling::from_node(build_node(
                    x_coord_min,
                    x_coord_mid,
                    y - 1,
                    height,
                    left_leaves,
                    new_padding_node_content,
                ));

                MatchedPair { left, right }
            }
        }
        NumNodes::AllNodes => {
            // Go down left child only (there are no leaves living on the right side).
            let left = LeftSibling::from_node(build_node(
                x_coord_min,
                x_coord_mid,
                y - 1,
                height,
                leaves,
                new_padding_node_content.clone(),
            ));
            let right = left.new_sibling_padding_node_arc(new_padding_node_content);
            MatchedPair { left, right }
        }
        NumNodes::NoNodes => {
            // Go down right child only (there are no leaves living on the left side).
            let right = RightSibling::from_node(build_node(
                x_coord_mid + 1,
                x_coord_max,
                y - 1,
                height,
                leaves,
                new_padding_node_content.clone(),
            ));
            let left = right.new_sibling_padding_node_arc(new_padding_node_content);
            MatchedPair { left, right }
        }
    };

    pair.merge()
}
