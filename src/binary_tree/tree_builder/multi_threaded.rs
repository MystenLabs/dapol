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
//!
//! Not all of the nodes in the tree are necessarily placed in the store. By
//! default only the non-padding leaf nodes and the nodes in the top half of the
//! tree are placed in the store. This can be increased using the `store_depth`
//! parameter. If `store_depth == 1` then only the root node is stored and if
//! `store_depth == n` then the root node plus the next `n-1` layers from the
//! root node down are stored. So if `store_depth == height` then all the nodes
//! are stored.

use std::fmt::Debug;
use std::ops::Range;

use log::{info, warn};
use logging_timer::stime;

use dashmap::DashMap;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use std::thread;

use super::super::{
    max_bottom_layer_nodes, Coordinate, Height, InputLeafNode, MatchedPair, Mergeable, Node,
    Sibling, Store, MIN_RECOMMENDED_SPARSITY, MIN_STORE_DEPTH,
};
use super::{BinaryTree, TreeBuildError};

static BUG: &'static str = "[Bug in multi-threaded builder]";

// -------------------------------------------------------------------------------------------------
// Tree build function.

/// Construct the binary tree.
///
/// The leaf node vector is cleaned in the following ways:
/// - sorted according to their x-coord
/// - all x-coord <= max
/// - checked for duplicates (duplicate if same x-coords)
#[stime("info", "MultiThreadedBuilder::{}")]
pub fn build_tree<C, F>(
    height: Height,
    store_depth: u8,
    mut input_leaf_nodes: Vec<InputLeafNode<C>>,
    new_padding_node_content: F,
) -> Result<BinaryTree<C>, TreeBuildError>
where
    C: Debug + Clone + Mergeable + Send + Sync + 'static,
    F: Fn(&Coordinate) -> C + Send + Sync + 'static,
{
    use super::verify_no_duplicate_leaves;

    let leaf_nodes = {
        // Sort by x-coord ascending.
        input_leaf_nodes.par_sort_by(|a, b| a.x_coord.cmp(&b.x_coord));

        verify_no_duplicate_leaves(&input_leaf_nodes)?;

        // Translate InputLeafNode to Node.
        input_leaf_nodes
            .into_par_iter()
            .map(|leaf| leaf.to_node())
            .collect::<Vec<Node<C>>>()
    };

    let store = Arc::new(DashMap::<Coordinate, Node<C>>::new());
    let params = RecursionParams::from_tree_height(height.clone()).with_store_depth(store_depth);

    if max_bottom_layer_nodes(&height) / leaf_nodes.len() as u64 <= MIN_RECOMMENDED_SPARSITY as u64
    {
        warn!(
            "Minimum recommended tree sparsity of {} reached, consider increasing tree height",
            MIN_RECOMMENDED_SPARSITY
        );
    }

    let root = build_node(
        params,
        leaf_nodes,
        Arc::new(new_padding_node_content),
        Arc::clone(&store),
    );

    let store = Box::new(DashMapStore {
        map: Arc::into_inner(store).ok_or(TreeBuildError::StoreOwnershipFailure)?,
    });

    Ok(BinaryTree {
        root,
        store,
        height,
    })
}

// -------------------------------------------------------------------------------------------------
// Store.

type Map<C> = DashMap<Coordinate, Node<C>>;

struct DashMapStore<C> {
    map: Map<C>,
}

impl<C: Clone> Store<C> for DashMapStore<C> {
    fn get_node(&self, coord: &Coordinate) -> Option<Node<C>> {
        self.map.get(coord).map(|n| n.clone())
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
fn get_num_nodes_left_of<C>(x_coord_mid: u64, nodes: &Vec<Node<C>>) -> NumNodes {
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

impl<C> Node<C> {
    /// New padding node contents are given by a closure. Why a closure? Because
    /// creating a padding node may require context outside of this scope, where
    /// type `C` is defined, for example.
    fn new_sibling_padding_node_arc<F>(&self, new_padding_node_content: Arc<F>) -> Node<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = self.sibling_coord();
        let content = new_padding_node_content(&coord);
        Node { coord, content }
    }
}

impl<C: Mergeable> MatchedPair<C> {
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

    /// Create a new pair from 2 sibling nodes.
    ///
    /// Panic if the given nodes are not a siblings.
    /// Since this code is only used internally for tree construction, and this
    /// state is unrecoverable, panicking is the best option. It is a sanity
    /// check and should never actually happen unless code is changed.
    fn from_siblings(left: Node<C>, right: Node<C>) -> Self {
        if !left.is_left_sibling_of(&right) {
            panic!(
                "{} The given left node is not a left sibling of the given right node",
                BUG
            )
        }
        MatchedPair { left, right }
    }
}

// -------------------------------------------------------------------------------------------------
// Build algorithm.

/// Parameters for the recursive build function.
///
/// Every iteration of [build_node] relates to a particular layer in the tree,
/// and `y_coord` is exactly what defines this layer.
///
/// The x-coord fields relate to the bottom layer of the tree.
///
/// `x_coord_min` is the left-most x-coord of the bottom layer nodes of the
/// subtree whose root node is the current one being generated by the recursive
/// iteration. `x_coord_max` is the right-most x-coord of the bottom layer nodes
/// of the same subtree.
///
/// `x_coord_mid` is used to split the leaves into left and right vectors.
/// Nodes in the left vector have x-coord <= mid, and
/// those in the right vector have x-coord > mid.
///
/// `max_thread_count` is there to prevent more threads being spawned
/// than there are cores to execute them. If too many threads are spawned then
/// the parallelization can actually be detrimental to the run-time. Threads
#[derive(Clone)]
pub struct RecursionParams {
    x_coord_min: u64,
    x_coord_mid: u64,
    x_coord_max: u64,
    y_coord: u8,
    thread_count: Arc<Mutex<u8>>,
    max_thread_count: u8,
    store_depth: u8,
    height: Height,
}

/// The default max number of threads.
/// This value is used in the case where the number of threads cannot be
/// determined from the underlying hardware. 4 was chosen as the default because
/// most modern (circa 2023) architectures will have at least 4 cores.
const DEFAULT_MAX_THREAD_COUNT: u8 = 4;

/// Private functions for use within this file only.
impl RecursionParams {
    /// Construct the parameters given only the height of the tree.
    ///
    /// - `x_coord_min` points to the start of the bottom layer.
    /// - `x_coord_max` points to the end of the bottom layer.
    /// - `x_coord_mid` is set to the middle of `x_coord_min` & `x_coord_max`.
    /// - `y_coord` is set to `height - 1` because the recursion starts from the
    /// root node.
    /// - `tread_count` is set to 1 (not 0) to account for the main thread.
    /// - `max_thread_count` is set based on how much [parallelism] the
    /// underlying machine is able to offer.
    /// - `store_depth` defaults to the min value.
    ///
    /// [`parallelism`]: std::thread::available_parallelism
    fn from_tree_height(height: Height) -> Self {
        // Start from the first node.
        let x_coord_min = 0;
        // x-coords start from 0, hence the `- 1`.
        let x_coord_max = max_bottom_layer_nodes(&height) - 1;
        let x_coord_mid = (x_coord_min + x_coord_max) / 2;
        // y-coords also start from 0, hence the `- 1`.
        let y_coord = height.as_y_coord();

        let mut max_thread_count = DEFAULT_MAX_THREAD_COUNT;
        crate::DEFAULT_PARALLELISM_APPROX.with(|opt| {
            match *opt.borrow() {
                None =>
                    warn!("No default parallelism found, defaulting to {}", max_thread_count)
                ,
                Some(par) => {
                    max_thread_count = par;
                    info!(
                        "Available parallelism detected: {}. This will be the max number of threads spawned.",
                        max_thread_count
                    );
                }
            }
        });

        RecursionParams {
            x_coord_min,
            x_coord_mid,
            x_coord_max,
            y_coord,
            // TODO need to unit test that this number matches actual thread count
            thread_count: Arc::new(Mutex::new(1)),
            max_thread_count,
            store_depth: MIN_STORE_DEPTH,
            height,
        }
    }

    /// Convert the params for the node which is the focus of the current
    /// iteration to params for that node's left child.
    fn into_left_child(mut self) -> Self {
        self.x_coord_max = self.x_coord_mid;
        self.x_coord_mid = (self.x_coord_min + self.x_coord_max) / 2;
        self.y_coord -= 1;
        self
    }

    /// Convert the params for the node which is the focus of the current
    /// iteration to params for that node's right child.
    fn into_right_child(mut self) -> Self {
        self.x_coord_min = self.x_coord_mid + 1;
        self.x_coord_mid = (self.x_coord_min + self.x_coord_max) / 2;
        self.y_coord -= 1;
        self
    }
}

/// Public functions available to [super][super][path_builder].
impl RecursionParams {
    pub fn from_coordinate(coord: &Coordinate) -> Self {
        use super::super::MAX_HEIGHT;

        let (x_coord_min, x_coord_max) = coord.subtree_x_coord_bounds();
        let x_coord_mid = (x_coord_min + x_coord_max) / 2;

        RecursionParams {
            x_coord_min,
            x_coord_mid,
            x_coord_max,
            y_coord: coord.y,
            thread_count: Arc::new(Mutex::new(0)),
            max_thread_count: 1,
            store_depth: MIN_STORE_DEPTH,
            height: MAX_HEIGHT.clone(),
        }
    }

    pub fn x_coord_range(&self) -> Range<u64> {
        self.x_coord_min..self.x_coord_max + 1
    }

    pub fn with_store_depth(mut self, store_depth: u8) -> Self {
        self.store_depth = store_depth;
        self
    }

    pub fn with_tree_height(mut self, height: Height) -> Self {
        self.height = height;
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
pub fn build_node<C, F>(
    params: RecursionParams,
    mut leaves: Vec<Node<C>>,
    new_padding_node_content: Arc<F>,
    map: Arc<Map<C>>,
) -> Node<C>
where
    C: Debug + Clone + Mergeable + Send + Sync + 'static,
    F: Fn(&Coordinate) -> C + Send + Sync + 'static,
{
    {
        let max_nodes = max_bottom_layer_nodes(&Height::from_y_coord(params.y_coord));
        assert!(
            leaves.len() <= max_nodes as usize,
            "{} Leaf node count ({}) exceeds layer max node number ({})",
            BUG,
            leaves.len(),
            max_nodes
        );

        assert_ne!(leaves.len(), 0, "{} Number of leaf nodes cannot be 0", BUG);

        assert!(
            params.x_coord_min % 2 == 0,
            "{} x_coord_min ({}) must be a multiple of 2 or 0",
            BUG,
            params.x_coord_min
        );

        assert!(
            params.x_coord_max % 2 == 1,
            "{} x_coord_max ({}) must not be a multiple of 2",
            BUG,
            params.x_coord_max
        );

        let v = params.x_coord_max - params.x_coord_min + 1;
        assert!(
            (v & (v - 1)) == 0,
            "{} x_coord_max - x_coord_min + 1 ({}) must be a power of 2",
            BUG,
            v
        );
    }

    // Base case: reached the 2nd-to-bottom layer.
    // There are either 2 or 1 leaves left (which is checked above).
    if params.y_coord == 1 {
        let pair = if leaves.len() == 2 {
            let right = leaves.pop().unwrap();
            let left = leaves.pop().unwrap();

            map.insert(left.coord.clone(), left.clone());
            map.insert(right.coord.clone(), right.clone());

            MatchedPair::from_siblings(left, right)
        } else {
            let node = leaves.pop().unwrap();

            // Only the leaf node is placed in the store, it's sibling pad node
            // is left out.
            map.insert(node.coord.clone(), node.clone());

            MatchedPair::from_node(node, new_padding_node_content)
        };

        return pair.merge();
    }

    // NOTE this includes the root node.
    let within_store_depth_for_children =
        params.y_coord - 1 >= params.height.as_raw_int() - params.store_depth;

    let pair = match get_num_nodes_left_of(params.x_coord_mid, &leaves) {
        NumNodes::SomeNodes(index) => {
            let right_leaves = leaves.split_off(index + 1);
            let left_leaves = leaves;

            let new_padding_node_content_ref = Arc::clone(&new_padding_node_content);

            // Split off a thread to build the right child, but only do this if the thread
            // count is less than the max allowed.
            if *params.thread_count.lock().unwrap() < params.max_thread_count {
                {
                    *params.thread_count.lock().unwrap() += 1;
                }
                let params_clone = params.clone();
                let map_ref = Arc::clone(&map);

                let right_handler = thread::spawn(move || -> Node<C> {
                    build_node(
                        params_clone.into_right_child(),
                        right_leaves,
                        new_padding_node_content_ref,
                        map_ref,
                    )
                });

                let left = build_node(
                    params.into_left_child(),
                    left_leaves,
                    new_padding_node_content,
                    Arc::clone(&map),
                );

                // If there is a problem joining onto the thread then there is no way to recover
                // so panic.
                let right = right_handler
                    .join()
                    .unwrap_or_else(|_| panic!("{} Couldn't join on the associated thread", BUG));

                MatchedPair { left, right }
            } else {
                let right = build_node(
                    params.clone().into_right_child(),
                    right_leaves,
                    new_padding_node_content_ref,
                    Arc::clone(&map),
                );

                let left = build_node(
                    params.into_left_child(),
                    left_leaves,
                    new_padding_node_content,
                    Arc::clone(&map),
                );

                MatchedPair { left, right }
            }
        }
        NumNodes::AllNodes => {
            // Go down left child only (there are no leaves living on the right side).
            let left = build_node(
                params.into_left_child(),
                leaves,
                new_padding_node_content.clone(),
                Arc::clone(&map),
            );
            let right = left.new_sibling_padding_node_arc(new_padding_node_content);
            MatchedPair { left, right }
        }
        NumNodes::NoNodes => {
            // Go down right child only (there are no leaves living on the left side).
            let right = build_node(
                params.into_right_child(),
                leaves,
                new_padding_node_content.clone(),
                Arc::clone(&map),
            );
            let left = right.new_sibling_padding_node_arc(new_padding_node_content);
            MatchedPair { left, right }
        }
    };

    if within_store_depth_for_children {
        map.insert(pair.left.coord.clone(), pair.left.clone());
        map.insert(pair.right.coord.clone(), pair.right.clone());
    }

    pair.merge()
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

// TODO check all leaf nodes are in the store, and that the desired level of
// nodes is in the store TODO check certain number of leaf nodes are in the tree
// TODO recursive function err - num leaf nodes exceeds max
// TODO recursive function err - empty leaf nodes
// TODO recursive function err - NOT x-coord min multiple of 2 or 0
// TODO recursive function err - NOT x-coord max multiple of 2
// TODO recursive function err - max - min must be power of 2

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
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
            .build_using_multi_threaded_algorithm(get_padding_function());

        // cannot use assert_err because it requires Func to have the Debug trait
        assert_err_simple!(res, Err(TreeBuildError::NoHeightProvided));
    }

    #[test]
    fn err_when_parent_builder_leaf_nodes_not_set() {
        let height = Height::from(4);
        let res = TreeBuilder::new()
            .with_height(height)
            .build_using_multi_threaded_algorithm(get_padding_function());

        // cannot use assert_err because it requires Func to have the Debug trait
        assert_err_simple!(res, Err(TreeBuildError::NoLeafNodesProvided));
    }

    #[test]
    fn err_for_empty_leaves() {
        let height = Height::from(5);
        let res = TreeBuilder::<TestContent>::new()
            .with_height(height)
            .with_leaf_nodes(Vec::<InputLeafNode<TestContent>>::new())
            .build_using_multi_threaded_algorithm(get_padding_function());

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
            .build_using_multi_threaded_algorithm(get_padding_function());

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
            .build_using_multi_threaded_algorithm(get_padding_function());

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
            .build_using_multi_threaded_algorithm(get_padding_function());

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
            .build_using_multi_threaded_algorithm(get_padding_function())
            .unwrap();
        let root = tree.root();

        leaf_nodes.shuffle(&mut thread_rng());

        let tree = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes)
            .build_using_multi_threaded_algorithm(get_padding_function())
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
            .build_using_multi_threaded_algorithm(get_padding_function())
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
            .build_using_multi_threaded_algorithm(get_padding_function())
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
            .build_using_multi_threaded_algorithm(get_padding_function())
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
}
