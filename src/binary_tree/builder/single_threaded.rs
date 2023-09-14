use std::collections::HashMap;
use std::fmt::Debug;

use super::super::{
    BinaryTree, Coordinate, LeftSibling, MatchedPair, Mergeable, Node, RightSibling, Sibling,
};
use super::{TreeBuildError, TreeBuilder};

// -------------------------------------------------------------------------------------------------
// Main struct.

pub struct SingleThreadedBuilder<C>
where
    C: Clone,
{
    height: u8,
    leaf_nodes: Vec<Node<C>>,
}

impl<C> SingleThreadedBuilder<C>
where
    C: Clone + Mergeable,
{
    pub fn new(parent_builder: TreeBuilder<C>) -> Result<Self, TreeBuildError> {
        use super::super::num_bottom_layer_nodes;

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
        let leaf_nodes = {
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

        Ok(SingleThreadedBuilder { height, leaf_nodes })
    }

    pub fn build<F>(self, padding_node_generator: F) -> Result<BinaryTree<C>, TreeBuildError>
    where
        C: Debug,
        F: Fn(&Coordinate) -> C,
    {
        let height = self.height;
        let leaf_nodes = self.leaf_nodes;
        let (store, root) = build_tree(leaf_nodes, height, padding_node_generator);

        Ok(BinaryTree {
            root,
            store,
            height,
        })
    }
}

// -------------------------------------------------------------------------------------------------
// Supporting structs.

/// A pair of sibling nodes, but one might be absent.
struct MaybeUnmatchedPair<C: Mergeable + Clone> {
    left: Option<LeftSibling<C>>,
    right: Option<RightSibling<C>>,
}

impl<C: Clone> LeftSibling<C> {
    /// New padding nodes are given by a closure. Why a closure? Because
    /// creating a padding node may require context outside of this scope, where
    /// type C is defined, for example.
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
    /// New padding nodes are given by a closure. Why a closure? Because
    /// creating a padding node may require context outside of this scope, where
    /// type C is defined, for example.
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

// -------------------------------------------------------------------------------------------------
// Build algorithm.

/// Create a new tree given the leaves, height and the padding node creation
/// function. New padding nodes are given by a closure. Why a closure?
/// Because creating a padding node may require context outside of this
/// scope, where type C is defined, for example.
// TODO there should be a warning if the height/leaves < min_sparsity (which was
// set to 2 in prev code)
fn build_tree<C, F>(
    mut nodes: Vec<Node<C>>,
    height: u8,
    new_padding_node_content: F,
) -> (HashMap<Coordinate, Node<C>>, Node<C>)
where
    C: Debug + Clone + Mergeable,
    F: Fn(&Coordinate) -> C,
{
    let mut store = HashMap::new();

    // Repeat for each layer of the tree.
    for _i in 0..height - 1 {
        // Create the next layer up of nodes from the current layer of nodes.
        nodes = nodes
            .into_iter()
            // Sort nodes into pairs (left & right siblings).
            .fold(Vec::<MaybeUnmatchedPair<C>>::new(), |mut pairs, node| {
                let sibling = Sibling::from_node(node);
                match sibling {
                    // If we have found a left sibling then create a new pair.
                    Sibling::Left(left_sibling) => pairs.push(MaybeUnmatchedPair {
                        left: Some(left_sibling),
                        right: Option::None,
                    }),
                    // If we have found a right sibling then either add to an existing pair with a
                    // left sibling or create a new pair containing only the right sibling.
                    Sibling::Right(right_sibling) => {
                        let is_right_sibling_of_prev_node = pairs
                            .last_mut()
                            .map(|pair| (&pair.left).as_ref())
                            .flatten()
                            .is_some_and(|left| right_sibling.0.is_right_sibling_of(&left.0));

                        if is_right_sibling_of_prev_node {
                            pairs
                                .last_mut()
                                // This case should never be reached because of the way
                                // is_right_sibling_of_prev_node is built.
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
            // Add padding nodes to unmatched pairs.
            .map(|pair| match (pair.left, pair.right) {
                (Some(left), Some(right)) => MatchedPair { left, right },
                (Some(left), None) => MatchedPair {
                    right: left.new_sibling_padding_node(&new_padding_node_content),
                    left,
                },
                (None, Some(right)) => MatchedPair {
                    left: right.new_sibling_padding_node(&new_padding_node_content),
                    right,
                },
                // If this case is reached then there is a bug in the above fold.
                (None, None) => {
                    panic!("[Bug in tree constructor] Invalid pair (None, None) found")
                }
            })
            // Create parents for the next loop iteration, and add the pairs to the tree store.
            .map(|pair| {
                let parent = pair.merge();
                store.insert(pair.left.0.coord.clone(), pair.left.0);
                store.insert(pair.right.0.coord.clone(), pair.right.0);
                parent
            })
            .collect();
    }

    // If the root node is not present then there is a bug in the above code.
    let root = nodes
        .pop()
        .expect("[Bug in tree constructor] Unable to find root node");

    assert!(
        nodes.len() == 0,
        "[Bug in tree constructor] Should be no nodes left to process"
    );

    store.insert(root.coord.clone(), root.clone());

    (store, root)
}
