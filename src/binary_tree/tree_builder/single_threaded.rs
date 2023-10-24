//! Sequential binary tree builder.
//!
//! It is recommended to rather use [super][multi_threaded] for better
//! performance.
//!
//! The build algorithm starts with the inputted bottom-layer leaf nodes, adds
//! padding nodes where required, and then constructs the next layer by merging
//! pairs of sibling nodes together.
//!
//! Not all of the nodes in the tree are necessarily placed in the store. By
//! default only the non-padding leaf nodes and the nodes in the top half of the
//! tree are placed in the store. This can be increased using the `store_depth`
//! parameter. If `store_depth == 1` then only the root node is stored and if
//! `store_depth == n` then the root node plus the next `n-1` layers from the
//! root node down are stored. So if `store_depth == height` then all the nodes
//! are stored.

use std::collections::HashMap;
use std::fmt::Debug;

use log::warn;
use logging_timer::stime;
use serde::Serialize;

use crate::binary_tree::max_bottom_layer_nodes;

use super::super::{
    BinaryTree, Coordinate, Height, InputLeafNode, MatchedPair, Mergeable, Node, Sibling, Store,
    MIN_RECOMMENDED_SPARSITY,
};
use super::TreeBuildError;

const BUG: &str = "[Bug in single-threaded builder]";

// -------------------------------------------------------------------------------------------------
// Tree build function.

/// Construct the tree using the provided parameters.
///
/// An error is returned if the parameters were not configured correctly
/// (or at all).
///
/// The leaf nodes are sorted by x-coord, checked for duplicates, and
/// converted to the right type.
#[stime("info", "SingleThreadedBuilder::{}")]
pub fn build_tree<C, F>(
    height: Height,
    store_depth: u8,
    mut input_leaf_nodes: Vec<InputLeafNode<C>>,
    new_padding_node_content: F,
) -> Result<BinaryTree<C>, TreeBuildError>
where
    C: Debug + Clone + Serialize + Mergeable + 'static, /* This static is needed for the boxed
                                                         * hashmap. */
    F: Fn(&Coordinate) -> C,
{
    use super::verify_no_duplicate_leaves;

    let leaf_nodes = {
        // Sort by x-coord ascending.
        input_leaf_nodes.sort_by(|a, b| a.x_coord.cmp(&b.x_coord));

        verify_no_duplicate_leaves(&input_leaf_nodes)?;

        // Translate InputLeafNode to Node.
        input_leaf_nodes
            .into_iter()
            .map(|input_node| input_node.into_node())
            .collect::<Vec<Node<C>>>()
    };

    if max_bottom_layer_nodes(&height) / leaf_nodes.len() as u64 <= MIN_RECOMMENDED_SPARSITY as u64
    {
        warn!(
            "Minimum recommended tree sparsity of {} reached, consider increasing tree height",
            MIN_RECOMMENDED_SPARSITY
        );
    }

    let (map, root) = build_node(leaf_nodes, &height, store_depth, &new_padding_node_content);

    Ok(BinaryTree {
        root,
        store: Store::SingleThreadedStore(HashMapStore { map }),
        height,
    })
}

// -------------------------------------------------------------------------------------------------
// Store.

#[derive(Serialize)]
pub struct HashMapStore<C> {
    map: Map<C>,
}

impl<C: Clone> HashMapStore<C> {
    pub fn get_node(&self, coord: &Coordinate) -> Option<Node<C>> {
        self.map.get(coord).map(|n| (*n).clone())
    }
}

// -------------------------------------------------------------------------------------------------
// Supporting structs & methods.

/// A pair of sibling nodes, but one might be absent.
///
/// At least one of the fields is expected to be set. If this is not the case
/// then it is assumed there is a bug in the code using this struct.
struct MaybeUnmatchedPair<C> {
    left: Option<Node<C>>,
    right: Option<Node<C>>,
}

impl<C> MaybeUnmatchedPair<C> {
    /// Convert the partially matched pair into a matched pair.
    ///
    /// If both left and right nodes are not present then the function will
    /// panic because this case indicates a bug in the calling code, which is
    /// not a recoverable scenario.
    ///
    /// If only one of the nodes is not present then it is created as a padding
    /// node.
    fn into_matched_pair<F>(self, new_padding_node_content: &F) -> MatchedPair<C>
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
    fn new_sibling_padding_node<F>(&self, new_padding_node_content: &F) -> Node<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.sibling_coord();
        let content = new_padding_node_content(&coord);
        Node { coord, content }
    }
}

// -------------------------------------------------------------------------------------------------
// Build algorithm.

type Map<C> = HashMap<Coordinate, Node<C>>;
type RootNode<C> = Node<C>;

/// Construct a new binary tree.
///
/// If `leaf_nodes` is empty or has length greater than what the tree height
/// allows then there will be panic. The builder is expected to
/// handle this case gracefully and this function is not public so a panic
/// is acceptable here.
/// Every element of `leaf_nodes` is assumed to have y-coord of 0. The function
/// will panic otherwise because this means there is a bug in the calling code.
///
/// The nodes are stored in a hashmap, which is returned along with the root
/// node (which is also stored in the hashmap).
///
/// `store_depth` determines how many layers are placed in the store. If
/// `store_depth == 1` then only the root node is stored and if
/// `store_depth == 2` then the root node and the next layer down are stored.
///
/// The min `store_depth` is 1. The function will panic if this is not the case.
///
/// Note that all bottom layer nodes are stored, both the inputted leaf
/// nodes and their accompanying padding nodes.
pub fn build_node<C, F>(
    leaf_nodes: Vec<Node<C>>,
    height: &Height,
    store_depth: u8,
    new_padding_node_content: &F,
) -> (Map<C>, RootNode<C>)
where
    C: Debug + Clone + Mergeable,
    F: Fn(&Coordinate) -> C,
{
    {
        // Some simple parameter checks.

        let max = max_bottom_layer_nodes(height);

        assert!(
            leaf_nodes.len() <= max as usize,
            "{} Too many leaf nodes",
            BUG
        );

        assert!(!leaf_nodes.is_empty(), "{} Empty leaf nodes", BUG);

        // All y-coords are 0.
        if let Some(node) = leaf_nodes.iter().find(|node| node.coord.y != 0) {
            panic!(
                "{} Node expected to have y-coord of 0 but was {}",
                BUG, node.coord.y
            );
        }

        use crate::binary_tree::MIN_STORE_DEPTH;
        assert!(
            store_depth >= MIN_STORE_DEPTH,
            "{} Store depth cannot be less than {} since the root node is always stored",
            BUG,
            MIN_STORE_DEPTH
        );
        assert!(
            store_depth <= height.as_raw_int(),
            "{} Store depth cannot exceed the height of the tree",
            BUG
        );
    }

    let mut map = HashMap::new();
    let mut nodes = leaf_nodes;

    // Repeat for each layer of the tree, except the root node layer.
    let max_y_coord = height.as_y_coord();
    for y in 0..max_y_coord {
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
                            .and_then(|pair| pair.left.as_ref())
                            .is_some_and(|left| right_sibling.is_right_sibling_of(left));

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
            .map(|pair| pair.into_matched_pair(&new_padding_node_content))
            // Create parents for the next loop iteration, and add the pairs to the tree store.
            .map(|pair| {
                let parent = pair.merge();
                // TODO may be able to further optimize by leaving out the padding leaf nodes
                // from the store.
                // Only insert nodes in the store if
                // a) node is a bottom layer leaf node (including padding nodes)
                // b) node is in one of the top X layers where X = store_depth
                // NOTE this includes the root node.
                if y == 0 || y >= height.as_raw_int() - store_depth {
                    map.insert(pair.left.coord.clone(), pair.left);
                    map.insert(pair.right.coord.clone(), pair.right);
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
        nodes.is_empty(),
        "{} Should be no nodes left to process",
        BUG
    );

    (map, root)
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

// TODO perform manual calculation of a tree and check that it equals the one
// generated here TODO check certain number of leaf nodes are in the tree

#[cfg(test)]
mod tests {
    use super::super::super::max_bottom_layer_nodes;
    use super::super::*;
    use crate::binary_tree::utils::test_utils::{
        full_bottom_layer, get_padding_function, single_leaf, sparse_leaves, TestContent,
    };
    use crate::test_utils::{assert_err, assert_err_simple};

    use primitive_types::H256;
    use rand::{thread_rng, Rng};

    #[test]
    fn err_when_parent_builder_height_not_set() {
        let height = Height::from(4);
        let leaf_nodes = full_bottom_layer(&height);
        let res = TreeBuilder::new()
            .with_leaf_nodes(leaf_nodes)
            .build_using_single_threaded_algorithm(get_padding_function());

        // cannot use assert_err because it requires Func to have the Debug trait
        assert_err_simple!(res, Err(TreeBuildError::NoHeightProvided));
    }

    #[test]
    fn err_when_parent_builder_leaf_nodes_not_set() {
        let height = Height::from(4);
        let res = TreeBuilder::new()
            .with_height(height)
            .build_using_single_threaded_algorithm(get_padding_function());

        // cannot use assert_err because it requires Func to have the Debug trait
        assert_err_simple!(res, Err(TreeBuildError::NoLeafNodesProvided));
    }

    #[test]
    fn err_for_empty_leaves() {
        let height = Height::from(5);
        let res = TreeBuilder::<TestContent>::new()
            .with_height(height)
            .with_leaf_nodes(Vec::<InputLeafNode<TestContent>>::new())
            .build_using_single_threaded_algorithm(get_padding_function());

        assert_err!(res, Err(TreeBuildError::EmptyLeaves));
    }

    #[test]
    fn err_for_too_many_leaves_with_height_first() {
        let height = Height::from(8u8);
        let mut leaf_nodes = full_bottom_layer(&height);

        leaf_nodes.push(InputLeafNode::<TestContent> {
            x_coord: max_bottom_layer_nodes(&height) + 1,
            content: TestContent {
                hash: H256::random(),
                value: thread_rng().gen(),
            },
        });

        let res = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes)
            .build_using_single_threaded_algorithm(get_padding_function());

        assert_err!(res, Err(TreeBuildError::TooManyLeaves));
    }

    #[test]
    fn err_for_duplicate_leaves() {
        let height = Height::from(4);
        let mut leaf_nodes = sparse_leaves(&height);
        leaf_nodes.push(single_leaf(leaf_nodes.get(0).unwrap().x_coord));

        let res = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes)
            .build_using_single_threaded_algorithm(get_padding_function());

        // cannot use assert_err because it requires Func to have the Debug trait
        assert_err_simple!(res, Err(TreeBuildError::DuplicateLeaves));
    }

    #[test]
    fn err_when_x_coord_greater_than_max() {
        let height = Height::from(4);
        let leaf_node = single_leaf(max_bottom_layer_nodes(&height) + 1);

        let res = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(vec![leaf_node])
            .build_using_single_threaded_algorithm(get_padding_function());

        // cannot use assert_err because it requires Func to have the Debug trait
        assert_err_simple!(res, Err(TreeBuildError::InvalidXCoord));
    }

    // tests that the sorting functionality works
    #[test]
    fn different_ordering_of_leaf_nodes_gives_same_root() {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let height = Height::from(4);
        let mut leaf_nodes = sparse_leaves(&height);

        let tree = TreeBuilder::new()
            .with_height(height.clone())
            .with_leaf_nodes(leaf_nodes.clone())
            .build_using_single_threaded_algorithm(&get_padding_function())
            .unwrap();
        let root = tree.root();

        leaf_nodes.shuffle(&mut thread_rng());

        let tree = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes)
            .build_using_single_threaded_algorithm(&get_padding_function())
            .unwrap();

        assert_eq!(root, tree.root());
    }

    #[test]
    fn bottom_layer_leaf_nodes_all_present_in_store() {
        let height = Height::from(5);
        let leaf_nodes = sparse_leaves(&height);

        let tree = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes.clone())
            .build_using_single_threaded_algorithm(&get_padding_function())
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
        let height = Height::from(8);
        let leaf_nodes = full_bottom_layer(&height);

        let tree = TreeBuilder::new()
            .with_height(height.clone())
            .with_leaf_nodes(leaf_nodes.clone())
            .build_using_single_threaded_algorithm(&get_padding_function())
            .unwrap();

        let middle_layer = height.as_raw_int() / 2;
        let layer_below_root = height.as_raw_int() - 1;

        // These nodes should be in the store.
        for y in middle_layer..layer_below_root {
            for x in 0..2u64.pow((height.as_raw_int() - y - 1) as u32) {
                let coord = Coordinate { x, y };
                tree.store
                    .get_node(&coord)
                    .unwrap_or_else(|| panic!("{:?} was expected to be in the store", coord));
            }
        }

        // These nodes should not be in the store.
        // Why 1 and not 0? Because leaf nodes are checked in another test.
        for y in 1..middle_layer {
            for x in 0..2u64.pow((height.as_raw_int() - y - 1) as u32) {
                let coord = Coordinate { x, y };
                if tree.store.get_node(&coord).is_some() {
                    panic!("{:?} was expected to not be in the store", coord);
                }
            }
        }
    }

    #[test]
    fn expected_internal_nodes_are_in_the_store_for_custom_store_depth() {
        let height = Height::from(8);
        let leaf_nodes = full_bottom_layer(&height);
        // TODO fuzz on this store depth
        let store_depth = 1;

        let tree = TreeBuilder::new()
            .with_height(height.clone())
            .with_leaf_nodes(leaf_nodes.clone())
            .with_store_depth(store_depth)
            .build_using_single_threaded_algorithm(&get_padding_function())
            .unwrap();

        let layer_below_root = height.as_raw_int() - 1;

        // Only the leaf nodes should be in the store.
        for x in 0..2u64.pow((height.as_raw_int() - 1) as u32) {
            let coord = Coordinate { x, y: 0 };
            tree.store
                .get_node(&coord)
                .unwrap_or_else(|| panic!("{:?} was expected to be in the store", coord));
        }

        // All internal nodes should not be in the store.
        for y in 1..layer_below_root {
            for x in 0..2u64.pow((height.as_raw_int() - y - 1) as u32) {
                let coord = Coordinate { x, y };
                if tree.store.get_node(&coord).is_some() {
                    panic!("{:?} was expected to not be in the store", coord);
                }
            }
        }
    }

    // TODO check padding nodes on bottom layer are not in the store unless
    // store depth is the max
}
