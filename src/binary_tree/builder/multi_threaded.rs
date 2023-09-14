use std::fmt::Debug;

use std::sync::Arc;
use std::thread;

use super::super::{
    num_bottom_layer_nodes, Coordinate, LeftSibling, MatchedPair, Mergeable, Node, RightSibling,
    Sibling, NodeOrientation,
};

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
    /// Since this code is only used internally for tree construction, and this state is unrecoverable, panicking is the best option. It is a sanity check and should never actually happen unless code is changed.
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
    /// Since this code is only used internally for tree construction, and this state is unrecoverable, panicking is the best option. It is a sanity check and should never actually happen unless code is changed.
    fn from_node(node: Node<C>) -> Self {
        match node.node_orientation() {
            NodeOrientation::Left => panic!("TODO not right node"),
            NodeOrientation::Right => Self(node),
        }
    }
}

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

        assert!(x_coord_max % 2 == 1,
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
            // TODO make this 4 a variable, actually we should make a struct that contains a bunch of the static data not needed in every iteration of the recursion
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
