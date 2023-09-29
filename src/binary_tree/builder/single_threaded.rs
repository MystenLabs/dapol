//! Sequential binary tree builder.
//!
//! It is recommended to rather use [super][multi_threaded] for better
//! performance.
//!
//! The build algorithm starts with the inputted bottom-layer leaf nodes, adds
//! padding nodes where required, and then constructs the next layer by merging
//! pairs of sibling nodes together.

use dashmap::DashMap;
use std::fmt::Debug;
use std::rc::Rc;

use super::super::{BinaryTree, Coordinate, Map, MatchedPair, Mergeable, Node, Sibling, Store, x_coord_gen};
use super::{TreeBuildError, TreeBuilder};

// -------------------------------------------------------------------------------------------------
// Main struct.

#[derive(Debug)]
pub struct SingleThreadedBuilder<C, F> {
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

        let height = self.parent_builder.get_and_verify_height()?;
        let store_depth = self.parent_builder.get_or_default_store_depth(height);
        let mut input_leaf_nodes = self.parent_builder.get_and_verify_leaf_nodes(height)?;

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

        let padding_node_generator = Rc::new(
            self.padding_node_generator
                .ok_or(TreeBuildError::NoPaddingNodeGeneratorProvided)?,
        );
        // let padding_node_generator =
        //     self.padding_node_generator
        //         .ok_or(TreeBuildError::NoPaddingNodeGeneratorProvided)?;

        let node_generator = {
            let height = height.clone();
            let store_depth = 1;
            let padding_node_generator = Rc::clone(&padding_node_generator);

            move |coord: &Coordinate, store: &Store<C>| {
                // 1. determine range of x-coords for bottom-layer leaf nodes
                let x_coord_min = x_coord_gen(2 * coord.x, coord.y - 1);
                let x_coord_max = x_coord_gen(2 * (coord.x + 1), coord.y - 1);

                // 2. search the store for these leaf nodes
                let mut leaf_nodes: Vec<Node<C>> = Vec::new();
                for x in x_coord_min..x_coord_max {
                    let coord = Coordinate { x, y: 0 };
                    store
                        .node_map
                        .get(&coord)
                        .map(|node| leaf_nodes.push((*node).clone()));
                }

                if leaf_nodes.len() == 0 {
                    // 3. if no nodes are there then create a padding node and return that
                    // TODO need to change the name here to say "content" because it does not generate a node
                    Node {
                        content: padding_node_generator(&coord),
                        coord: coord.clone(),
                    }
                } else {
                    // 4. if there are nodes there then copy them from the store and send them to the build algo
                    let (_, node) =
                        build_tree(leaf_nodes, height, store_depth, Rc::clone(&padding_node_generator));
                    node
                }
            }
        };

        let (map, root) = build_tree(leaf_nodes, height, store_depth, padding_node_generator);

        Ok(BinaryTree {
            root,
            store: Store { node_map: map },
            node_generator: Box::new(node_generator),
            height,
        })
    }
}

// -------------------------------------------------------------------------------------------------
// Supporting structs & methods.

/// A pair of sibling nodes, but one might be absent.
struct MaybeUnmatchedPair<C> {
    left: Option<Node<C>>,
    right: Option<Node<C>>,
}

impl<C> MaybeUnmatchedPair<C> {
    fn to_matched_pair<F>(self, new_padding_node_content: Rc<F>) -> MatchedPair<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        match (self.left, self.right) {
            (Some(left), Some(right)) => MatchedPair { left, right },
            (Some(left), None) => MatchedPair {
                right: left.new_sibling_padding_node(new_padding_node_content),
                left,
            },
            (None, Some(right)) => MatchedPair {
                left: right.new_sibling_padding_node(new_padding_node_content),
                right,
            },
            // If this case is reached then there is a bug in the above fold.
            (None, None) => {
                panic!("{} Invalid pair (None, None) found", BUG)
            }
        }
    }
}

impl<C> Node<C> {
    /// New padding node contents are given by a closure. Why a closure? Because
    /// creating a padding node may require context outside of this scope, where
    /// type C is defined, for example.
    fn new_sibling_padding_node<F>(&self, new_padding_node_content: Rc<F>) -> Node<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.sibling_coord();
        let content = new_padding_node_content(&coord);
        Node { coord, content }
    }
}

static BUG: &'static str = "[Bug in single-threaded builder]";

// -------------------------------------------------------------------------------------------------
// Build algorithm.

type RootNode<C> = Node<C>;

/// Construct a new binary tree.
///
/// If `leaf_nodes` is empty or has length greater than what the tree height
/// allows then there will be panic. The builder is expected to
/// handle this case gracefully and this function is not public so a panic
/// is acceptable here.
/// Every element of `leaf_nodes` is assumed to have y-coord of 0; there is
/// only a naive check for this (only first node is checked).
///
/// The nodes are stored in a hashmap, which is returned along with the root node
/// (which is also stored in the hashmap).
///
/// `store_depth` determines how many layers are placed in the store. If
/// `store_depth == 1` then only the root node is stored and if
/// `store_depth == 2` then the root node and the next layer down are stored.
///
/// Note that the root node is not actually put in the hashmap because it is
/// returned along with the hashmap, but it is considered to be stored so
/// `store_depth` must at least be 1.
/// Also note that all bottom layer nodes are stored, both the inputted leaf
/// nodes and their accompanying padding nodes.
///
// TODO there should be a warning if the height/leaves < min_sparsity (which was
// set to 2 in prev code)
fn build_tree<C, F>(
    leaf_nodes: Vec<Node<C>>,
    height: u8,
    store_depth: u8,
    new_padding_node_content: Rc<F>,
) -> (Map<C>, RootNode<C>)
where
    C: Debug + Clone + Mergeable,
    F: Fn(&Coordinate) -> C,
{
    {
        // Some simple parameter checks.

        use super::super::num_bottom_layer_nodes;
        assert!(
            leaf_nodes.len() <= num_bottom_layer_nodes(height) as usize,
            "{} Too many leaf nodes",
            BUG
        );

        if let Some(node) = leaf_nodes.first() {
            assert_eq!(
                node.coord.y, 0,
                "{} Node expected to have y-coord of 0 but was {}",
                BUG, node.coord.y
            );
        } else {
            panic!("{} Empty leaf nodes", BUG);
        }

        assert!(
            store_depth >= 1,
            "{} Store depth cannot be less than 1 since the root node is always stored",
            BUG
        );
        assert!(
            store_depth <= height,
            "{} Store depth cannot exceed the height of the tree",
            BUG
        );
    }

    let mut store = DashMap::new();
    let mut nodes = leaf_nodes;

    // Repeat for each layer of the tree, except the root node layer.
    let layer_below_root = height - 1;
    for i in 0..layer_below_root {
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
                                .unwrap_or_else(|| panic!("{} Previous node not found", BUG))
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
            .map(|pair| pair.to_matched_pair(new_padding_node_content))
            // Create parents for the next loop iteration, and add the pairs to the tree store.
            .map(|pair| {
                let parent = pair.merge();
                // TODO may be able to further optimize by leaving out the padding leaf nodes from the store
                // Only insert nodes in the store if
                // a) node is a bottom layer leaf node (including padding nodes)
                // b) node is in one of the top X layers where X = store_depth
                if i == 0 || i >= layer_below_root - (store_depth - 1) {
                    store.insert(pair.left.coord.clone(), pair.left);
                    store.insert(pair.right.coord.clone(), pair.right);
                }
                parent
            })
            .collect();
    }

    // If the root node is not present then there is a bug in the above code.
    let root = nodes
        .pop()
        .unwrap_or_else(|| panic!("{} Unable to find root node", BUG));

    assert!(
        nodes.len() == 0,
        "{} Should be no nodes left to process",
        BUG
    );

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
        full_bottom_layer, get_padding_function, single_leaf, sparse_leaves, TestContent,
    };
    use crate::testing_utils::{assert_err, assert_err_simple};

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

    #[test]
    fn expected_internal_nodes_are_in_the_store_for_default_store_depth() {
        let height = 8;
        let leaf_nodes = full_bottom_layer(height);

        let tree = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes.clone())
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(&get_padding_function())
            .build()
            .unwrap();

        let middle_layer = height / 2;
        let layer_below_root = height - 1;

        // These nodes should be in the store.
        for y in middle_layer..layer_below_root {
            for x in 0..2u64.pow((height - y - 1) as u32) {
                let coord = Coordinate { x, y };
                tree.store
                    .node_map
                    .get(&coord)
                    .unwrap_or_else(|| panic!("{:?} was expected to be in the store", coord));
            }
        }

        // These nodes should not be in the store.
        // Why 1 and not 0? Because leaf nodes are checked in another test.
        for y in 1..middle_layer {
            for x in 0..2u64.pow((height - y - 1) as u32) {
                let coord = Coordinate { x, y };
                if tree.store.node_map.get(&coord).is_some() {
                    panic!("{:?} was expected to not be in the store", coord);
                }
            }
        }
    }

    #[test]
    fn expected_internal_nodes_are_in_the_store_for_custom_store_depth() {
        let height = 8;
        let leaf_nodes = full_bottom_layer(height);
        let store_depth = 1;

        let tree = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes.clone())
            .with_store_depth(store_depth)
            .with_single_threaded_build_algorithm()
            .with_padding_node_generator(&get_padding_function())
            .build()
            .unwrap();

        let layer_below_root = height - 1;

        // Only the leaf nodes should be in the store.
        for x in 0..2u64.pow((height - 1) as u32) {
            let coord = Coordinate { x, y: 0 };
            tree.store
                .node_map
                .get(&coord)
                .unwrap_or_else(|| panic!("{:?} was expected to be in the store", coord));
        }

        // All internal nodes should not be in the store.
        for y in 1..layer_below_root {
            for x in 0..2u64.pow((height - y - 1) as u32) {
                let coord = Coordinate { x, y };
                if tree.store.node_map.get(&coord).is_some() {
                    panic!("{:?} was expected to not be in the store", coord);
                }
            }
        }
    }
}
