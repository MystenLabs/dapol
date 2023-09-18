//! Sequential binary tree builder.
//!
//! It is recommended to rather use [super][multi_threaded] for better
//! performance.
//!
//! The build algorithm starts with the inputted bottom-layer leaf nodes, adds
//! padding nodes where required, and then constructs the next layer by merging
//! pairs of sibling nodes together.

use std::collections::HashMap;
use std::fmt::Debug;

use super::super::{
    BinaryTree, Coordinate, ErrOnSome, ErrUnlessTrue, MatchedPair, Mergeable, Node, Sibling,
};
use super::{TreeBuildError, TreeBuilder};

// -------------------------------------------------------------------------------------------------
// Main struct.

pub struct SingleThreadedBuilder<C, F>
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
impl<C, F> SingleThreadedBuilder<C, F>
where
    C: Debug + Clone + Mergeable,
    F: Fn(&Coordinate) -> C,
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
        if input_leaf_nodes.len() == 0 {
            return Err(TreeBuildError::EmptyLeaves);
        }

        let leaf_nodes = {
            // Translate InputLeafNode to Node.
            let mut leaf_nodes: Vec<Node<C>> = input_leaf_nodes
                .into_iter()
                .map(|leaf| leaf.to_node())
                .collect();

            // Sort by x-coord ascending.
            leaf_nodes.sort_by(|a, b| a.coord.x.cmp(&b.coord.x));

            // Make sure all x-coord < max.
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

        Ok(SingleThreadedBuilder {
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

    pub fn build(self) -> Result<BinaryTree<C>, TreeBuildError> {
        let padding_node_generator = self
            .padding_node_generator
            .ok_or(TreeBuildError::NoPaddingNodeGeneratorProvided)?;

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
// Supporting structs & methods.

/// A pair of sibling nodes, but one might be absent.
struct MaybeUnmatchedPair<C: Mergeable + Clone> {
    left: Option<Node<C>>,
    right: Option<Node<C>>,
}

impl<C: Mergeable + Clone> MaybeUnmatchedPair<C> {
    fn to_matched_pair<F>(self, new_padding_node_content: &F) -> MatchedPair<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        match (self.left, self.right) {
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
        }
    }
}

impl<C: Clone> Node<C> {
    /// New padding node contents are given by a closure. Why a closure? Because
    /// creating a padding node may require context outside of this scope, where
    /// type C is defined, for example.
    fn new_sibling_padding_node<F>(&self, new_padding_node_content: &F) -> Node<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.get_sibling_coord();
        let content = new_padding_node_content(&coord);
        Node { coord, content }
    }
}

// -------------------------------------------------------------------------------------------------
// Build algorithm.

type Store<C> = HashMap<Coordinate, Node<C>>;
type RootNode<C> = Node<C>;

/// Construct a new binary tree.
///
/// The nodes are stored in a hashmap, which is returned along with the root node
/// (which is also stored in the hashmap).
// TODO there should be a warning if the height/leaves < min_sparsity (which was
// set to 2 in prev code)
fn build_tree<C, F>(
    mut nodes: Vec<Node<C>>,
    height: u8,
    new_padding_node_content: F,
) -> (Store<C>, RootNode<C>)
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
                            .is_some_and(|left| right_sibling.is_right_sibling_of(&left));

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
            .map(|pair| pair.to_matched_pair(&new_padding_node_content))
            // Create parents for the next loop iteration, and add the pairs to the tree store.
            .map(|pair| {
                let parent = pair.merge();
                store.insert(pair.left.coord.clone(), pair.left);
                store.insert(pair.right.coord.clone(), pair.right);
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

// -------------------------------------------------------------------------------------------------
// Unit tests.

// TODO new - err when no height or leaves given, or leaves empty, or leaves greater than max
// TODO new - err for duplicates
// TODO new - input nodes in different order result in tree builder with same ordering leaf node field (sorting works)
// TODO new - err when x-coord greater than max
// TODO build - no padding gen func gives err
// TODO inner build - input leaves are all present in the resulting tree
