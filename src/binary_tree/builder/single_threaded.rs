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

use super::super::{BinaryTree, Coordinate, MatchedPair, Mergeable, Node, Sibling};
use super::{TreeBuildError, TreeBuilder};

// -------------------------------------------------------------------------------------------------
// Main struct.

#[derive(Debug)]
pub struct SingleThreadedBuilder<C, F>
where
    C: Clone,
{
    parent_builder: TreeBuilder<C>,
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
    pub fn new(parent_builder: TreeBuilder<C>) -> Self {
        SingleThreadedBuilder {
            parent_builder,
            padding_node_generator: None,
        }
    }

    /// New padding nodes are given by a closure. Why a closure? Because
    /// creating a padding node may require context outside of this scope, where
    /// type C is defined, for example.
    pub fn with_padding_node_generator(mut self, padding_node_generator: F) -> Self {
        self.padding_node_generator = Some(padding_node_generator);
        self
    }

    pub fn build(self) -> Result<BinaryTree<C>, TreeBuildError> {
        use super::verify_no_duplicate_leaves;

        let (mut input_leaf_nodes, height) = self.parent_builder.verify_and_return_fields()?;

        let leaf_nodes = {
            // Sort by x-coord ascending.
            input_leaf_nodes.sort_by(|a, b| a.x_coord.cmp(&b.x_coord));

            verify_no_duplicate_leaves(&input_leaf_nodes)?;

            // Translate InputLeafNode to Node.
            input_leaf_nodes
                .into_iter()
                .map(|leaf| leaf.to_node())
                .collect::<Vec<Node<C>>>()
        };

        let padding_node_generator = self
            .padding_node_generator
            .ok_or(TreeBuildError::NoPaddingNodeGeneratorProvided)?;

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

// TODO perform manual calculation of a tree and check that it equals the one generated here
// TODO check certain number of leaf nodes are in the tree

#[cfg(test)]
mod tests {
    use super::super::super::num_bottom_layer_nodes;
    use super::super::*;
    use crate::binary_tree::utils::test_utils::{
        full_bottom_layer, get_padding_function, single_leaf, sparse_leaves, TestContent
    };
    use crate::testing_utils::{assert_err_simple, assert_err};

    use primitive_types::H256;
    use rand::{thread_rng, Rng};

    type Func = Box<dyn Fn(&Coordinate) -> TestContent>;

    #[test]
    fn err_when_parent_builder_height_not_set() {
        let height = 4;
        let leaf_nodes = full_bottom_layer(height);
        let res = TreeBuilder::new()
            .with_leaf_nodes(leaf_nodes)
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(get_padding_function())
            .build();

        // cannot use assert_err because it requires Func to have the Debug trait
        assert_err_simple!(res, Err(TreeBuildError::NoHeightProvided));
    }

    #[test]
    fn err_when_parent_builder_leaf_nodes_not_set() {
        let height = 4;
        let res = TreeBuilder::new()
            .with_height(height)
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(get_padding_function())
            .build();

        // cannot use assert_err because it requires Func to have the Debug trait
        assert_err_simple!(res, Err(TreeBuildError::NoLeafNodesProvided));
    }

    #[test]
    fn err_for_empty_leaves() {
        let height = 5;
        let res = TreeBuilder::<TestContent>::new()
            .with_height(height)
            .with_leaf_nodes(Vec::<InputLeafNode<TestContent>>::new())
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(get_padding_function())
            .build();

        assert_err!(res, Err(TreeBuildError::EmptyLeaves));
    }

    #[test]
    fn err_when_height_too_small() {
        assert!(MIN_HEIGHT > 0, "Invalid min height {}", MIN_HEIGHT);
        let height = MIN_HEIGHT - 1;
        let res = TreeBuilder::<TestContent>::new()
            .with_height(height)
            .with_leaf_nodes(vec![single_leaf(1, height)])
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(get_padding_function())
            .build();

        assert_err!(res, Err(TreeBuildError::HeightTooSmall));
    }

    #[test]
    fn err_for_too_many_leaves_with_height_first() {
        let height = 8u8;
        let mut leaf_nodes = full_bottom_layer(height);

        leaf_nodes.push(InputLeafNode::<TestContent> {
            x_coord: num_bottom_layer_nodes(height) + 1,
            content: TestContent {
                hash: H256::random(),
                value: thread_rng().gen(),
            },
        });

        let res = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes)
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(get_padding_function())
            .build();

        assert_err!(res, Err(TreeBuildError::TooManyLeaves));
    }

    #[test]
    fn err_for_duplicate_leaves() {
        let height = 4;
        let mut leaf_nodes = sparse_leaves(height);
        leaf_nodes.push(single_leaf(leaf_nodes.get(0).unwrap().x_coord, height));

        let res = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes)
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(get_padding_function())
            .build();

        // cannot use assert_err because it requires Func to have the Debug trait
        assert_err_simple!(res, Err(TreeBuildError::DuplicateLeaves));
    }

    #[test]
    fn err_when_x_coord_greater_than_max() {
        let height = 4;
        let leaf_node = single_leaf(num_bottom_layer_nodes(height) + 1, height);

        let res = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(vec![leaf_node])
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(get_padding_function())
            .build();

        // cannot use assert_err because it requires Func to have the Debug trait
        assert_err_simple!(res, Err(TreeBuildError::InvalidXCoord));
    }

    #[test]
    fn err_when_no_padding_func_given() {
        let height = 4;
        let leaf_nodes = sparse_leaves(height);

        let res = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes)
            .with_single_threaded_build_algorithm::<Func>()
            .build();

        // cannot use assert_err because it requires Func to have the Debug trait
        assert_err_simple!(res, Err(TreeBuildError::NoPaddingNodeGeneratorProvided));
    }

    // tests that the sorting functionality works
    #[test]
    fn different_ordering_of_leaf_nodes_gives_same_root() {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let height = 4;
        let mut leaf_nodes = sparse_leaves(height);

        let tree = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes.clone())
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(&get_padding_function())
            .build()
            .unwrap();
        let root = tree.get_root();

        leaf_nodes.shuffle(&mut thread_rng());

        let tree = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes)
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(&get_padding_function())
            .build()
            .unwrap();

        assert_eq!(root, tree.get_root());
    }

    #[test]
    fn bottom_layer_leaf_nodes_all_present_in_store() {
        let height = 5;
        let leaf_nodes = sparse_leaves(height);

        let tree = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes.clone())
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(&get_padding_function())
            .build()
            .unwrap();

        for leaf in leaf_nodes {
            tree.get_leaf_node(leaf.x_coord).expect(&format!(
                "Leaf node at x-coord {} is not present in the store",
                leaf.x_coord
            ));
        }
    }
}
