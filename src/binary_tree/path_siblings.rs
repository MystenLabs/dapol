//! PathSiblings [STENT TODO need to redo these docs] in a tree.
//!
//! A path in a binary tree goes from a leaf node to the root node. For each
//! node (starting from the leaf node) one follows the path by moving to the
//! parent node; since the root node has no parent this is the end of the path.
//!
//! A path is uniquely determined by a leaf node. It can thus be referred to as
//! the leaf node's path.
//!
//! The path is built using a builder pattern, which has 2 options in terms of
//! algorithms: sequential and multi-threaded. If the tree store is full (i.e.
//! every node that was used to construct the root node is in the store) then
//! the 2 build algorithms are identical. The difference only comes in when the
//! store is not full (which is useful to save on space) and some nodes need to
//! be regenerated. Both algorithms are the same as those used for tree
//! construction so their implementations can be found in
//! [super][tree_builder][multi_threaded] and
//! [super][tree_builder][single_threaded].

use crate::utils::Consume;
use super::{BinaryTree, Coordinate, Mergeable, Node, MIN_STORE_DEPTH};

use std::fmt::Debug;

// -------------------------------------------------------------------------------------------------
// Main struct.

/// Contains all the information for a path in a [BinaryTree].
///
/// The `siblings` vector contains all the sibling nodes of the nodes in a leaf
/// node's path. The siblings are ordered from bottom layer (first) to root node
/// (last, not included). The leaf node + the siblings can be used to
/// reconstruct the actual nodes in the path as well as the root node.
#[derive(Debug, Serialize, Deserialize)]
pub struct PathSiblings<C> (
    Vec<Node<C>>
);

// -------------------------------------------------------------------------------------------------
// Builder.

/// A builder pattern is used to construct [PathSiblings].
/// Since a path is uniquely determined by a leaf node all we need is the tree
/// and the leaf node's x-coord.
pub struct PathSiblingsBuilder<'a, C> {
    tree: Option<&'a BinaryTree<C>>,
    leaf_x_coord: Option<u64>,
}

impl<'a, C> PathSiblingsBuilder<'a, C> {
    pub fn new() -> Self {
        PathSiblingsBuilder {
            tree: None,
            leaf_x_coord: None,
        }
    }

    pub fn with_tree(mut self, tree: &'a BinaryTree<C>) -> Self {
        self.tree = Some(tree);
        self
    }

    pub fn with_leaf_x_coord(mut self, leaf_x_coord: u64) -> Self {
        self.leaf_x_coord = Some(leaf_x_coord);
        self
    }

    /// High performance build algorithm utilizing parallelization.
    /// Uses the same code in [super][tree_builder][multi_threaded].
    ///
    /// Note that the code only differs to
    /// [build_using_single_threaded_algorithm] if the tree store is not
    /// full and nodes have to be regenerated.
    ///
    /// `new_padding_node_content` is needed to generate new nodes.
    ///
    /// This function defines a closure for building nodes that are not found
    /// in the store, which is then passed to [build].
    pub fn build_using_multi_threaded_algorithm<F>(
        self,
        new_padding_node_content: F,
    ) -> Result<PathSiblings<C>, PathSiblingsBuildError>
    where
        C: Debug + Clone + Mergeable + Send + Sync + 'static,
        F: Fn(&Coordinate) -> C + Send + Sync + 'static,
    {
        use super::tree_builder::multi_threaded::{build_node, RecursionParams};
        use dashmap::DashMap;
        use std::sync::Arc;

        let new_padding_node_content = Arc::new(new_padding_node_content);

        let node_builder = |coord: &Coordinate, tree: &'a BinaryTree<C>| {
            let params = RecursionParams::from_coordinate(coord)
                // We don't want to store anything because the store already exists
                // inside the binary tree struct.
                .with_store_depth(MIN_STORE_DEPTH)
                .with_tree_height(tree.height.clone());

            // TODO This cloning can be optimized away by changing the
            // build_node function to use a pre-populated map instead of the
            // mutable leaves vector.
            let mut leaf_nodes = Vec::<Node<C>>::new();
            for x in params.x_coord_range() {
                tree.get_node(&Coordinate { x, y: 0 }).consume(|node| {
                    leaf_nodes.push(node);
                });
            }

            // if coord.y == 1 {
            // If the above vector is empty then we know this node needs to be a
            // padding node.
            if leaf_nodes.is_empty() {
                return Node {
                    coord: coord.clone(),
                    content: new_padding_node_content(coord),
                };
            }

            build_node(
                params,
                leaf_nodes,
                Arc::clone(&new_padding_node_content),
                Arc::new(DashMap::<Coordinate, Node<C>>::new()),
            )
        };

        self.build(node_builder)
    }

    /// Sequential build algorithm.
    /// Uses the same code in [super][tree_builder][single_threaded].
    ///
    /// Note that the code only differs to
    /// [build_using_multi_threaded_algorithm] if the tree store is not full
    /// and nodes have to be regenerated.
    ///
    /// `new_padding_node_content` is needed to generate new nodes.
    pub fn build_using_single_threaded_algorithm<F>(
        self,
        new_padding_node_content: F,
    ) -> Result<PathSiblings<C>, PathSiblingsBuildError>
    where
        C: Debug + Clone + Mergeable,
        F: Fn(&Coordinate) -> C,
    {
        use super::tree_builder::single_threaded::build_node;

        let node_builder = |coord: &Coordinate, tree: &'a BinaryTree<C>| {
            // We don't want to store anything because the store already exists
            // inside the binary tree struct.
            let store_depth = MIN_STORE_DEPTH;

            let (x_coord_min, x_coord_max) = coord.subtree_x_coord_bounds();

            // TODO This copying of leaf nodes could be optimized away by
            // changing the build function to accept a map parameter as apposed
            // to the leaf node vector.
            let mut leaf_nodes = Vec::<Node<C>>::new();
            for x in x_coord_min..x_coord_max + 1 {
                tree.get_node(&Coordinate::bottom_layer_leaf_from(x))
                    .consume(|node| {
                        leaf_nodes.push(node);
                    });
            }

            // If the above vector is empty then we know this node needs to be a
            // padding node.
            if leaf_nodes.is_empty() {
                return Node {
                    coord: coord.clone(),
                    content: new_padding_node_content(coord),
                };
            }

            // TODO The leaf nodes are cloned and put into a store that is
            // dropped. We should have an option to not put anything in the
            // store, maybe by changing store_depth to be an enum.
            let (_, node) = build_node(
                leaf_nodes,
                &coord.to_height(),
                store_depth,
                &new_padding_node_content,
            );

            node
        };

        self.build(node_builder)
    }

    /// Private build function that is to be called only by
    /// [build_using_multi_threaded_algorithm] or
    /// [build_using_single_threaded_algorithm].
    ///
    /// The path is traced from the leaf node to the root node. At every layer
    /// in the tree the sibling node is grabbed from the store (or generated if
    /// it is not in the store) and added to the vector in [PathSiblings].
    ///
    /// Since the store is expected to contain all non-padding leaf nodes an
    /// error will be returned if the leaf node at the given x-coord is not
    /// found in the store.
    fn build<F>(self, node_builder: F) -> Result<PathSiblings<C>, PathSiblingsBuildError>
    where
        C: Debug + Clone,
        F: Fn(&Coordinate, &'a BinaryTree<C>) -> Node<C>,
    {
        let tree = self.tree.ok_or(PathSiblingsBuildError::NoTreeProvided)?;

        // STENT TODO remove leaf stuff
        let leaf_x_coord = self.leaf_x_coord.ok_or(PathSiblingsBuildError::NoLeafProvided)?;
        let leaf_coord = Coordinate::bottom_layer_leaf_from(leaf_x_coord);

        let leaf =
            tree.get_leaf_node(leaf_x_coord)
            .ok_or_else(|| PathSiblingsBuildError::LeafNodeNotFound {
                coord: leaf_coord.clone(),
            })?;

        let mut siblings = Vec::with_capacity(tree.height().as_usize());
        let max_y_coord = tree.height().as_y_coord();
        let mut current_coord = leaf_coord;

        for _y in 0..max_y_coord {
            let sibling_coord = current_coord.sibling_coord();

            let sibling = tree
                .get_node(&sibling_coord)
                .unwrap_or_else(|| node_builder(&sibling_coord, tree));

            siblings.push(sibling);
            current_coord = current_coord.parent_coord();
        }

        Ok(PathSiblings (
            siblings
        ))
    }
}

impl<C> BinaryTree<C> {
    pub fn path_builder(&self) -> PathSiblingsBuilder<C> {
        PathSiblingsBuilder::new().with_tree(self)
    }
}

// -------------------------------------------------------------------------------------------------
// Implementation.

impl<C: Debug + Clone + Mergeable + PartialEq> PathSiblings<C> {
    // STENT TODO docs
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Verify that the given list of sibling nodes + the base leaf node matches
    /// the given root node.
    ///
    /// This is done by reconstructing each node in the path, from bottom layer
    /// to the root, using the given leaf and sibling nodes, and then
    /// comparing the resulting root node to the given root node.
    ///
    /// An error is returned if the number of siblings is less than the min
    /// amount, or the constructed root node does not match the given one.
        // STENT TODO add leaf as parameter, also change the name of this to something explaining that it is building the path
    // in fact it's probably better to move the verify to InclusionProof because this function is basically the same as the below function
    pub fn verify(&self, leaf: &Node<C>, root: &Node<C>) -> Result<(), PathSiblingsError> {
        use super::MIN_HEIGHT;

        // Cloning is not strictly necessary here but it makes the code cleaner,
        // and it's not an expensive clone.
        let mut parent = leaf.clone();

        if self.len() < MIN_HEIGHT.as_usize() {
            return Err(PathSiblingsError::TooFewSiblings);
        }

        for node in &self.0 {
            let pair = MatchedPairRef::new(node, &parent)?;
            parent = pair.merge();
        }

        if parent == *root {
            Ok(())
        } else {
            Err(PathSiblingsError::RootMismatch)
        }
    }

    /// Return a vector containing only the nodes in the tree path.
    ///
    /// The path nodes have to be constructed using the leaf & sibling nodes in
    /// [PathSiblings] because they are not stored explicitly. The order of the
    /// returned path nodes is bottom first (leaf) and top last (root).
    ///
    /// An error is returned if the [PathSiblings] data is invalid.
    pub fn build_path(&self, leaf: &Node<C>) -> Result<Vec<Node<C>>, PathSiblingsError> {
        // +1 because the root node is included in the returned vector
        let mut nodes = Vec::<Node<C>>::with_capacity(self.len() + 1);

        nodes.push(leaf.clone());

        for node in &self.0 {
            // this should never panic because we pushed the leaf node before the loop
            let parent = nodes
                .last()
                .expect("[Bug in path generation] Empty node vector");
            let pair = MatchedPairRef::new(node, parent)?;
            nodes.push(pair.merge());
        }

        Ok(nodes)
    }
}

// -------------------------------------------------------------------------------------------------
// PathSiblings conversion.

impl<C> PathSiblings<C> {
    /// Convert `PathSiblings<C>` to `PathSiblings<D>`.
    ///
    /// `convert` is called on each of the sibling nodes & leaf node.
    pub fn convert<B: From<C>>(self) -> PathSiblings<B> {
        PathSiblings (
            self
                .0
                .into_iter()
                .map(|node| node.convert())
                .collect()
        )
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

use serde::{Serialize, Deserialize};

#[derive(thiserror::Error, Debug)]
pub enum PathSiblingsBuildError {
    #[error("The builder must be given a padding node generator function before building")]
    NoPaddingNodeContentGeneratorProvided,
    #[error("The builder must be given a tree before building")]
    NoTreeProvided,
    #[error("The builder must be given the x-coord of a leaf node before building")]
    NoLeafProvided,
    #[error("Leaf node not found in the tree ({coord:?})")]
    LeafNodeNotFound { coord: Coordinate },
}

#[derive(thiserror::Error, Debug)]
pub enum PathSiblingsError {
    #[error("Calculated root content does not match provided root content")]
    RootMismatch,
    #[error("Provided node ({sibling_given:?}) is not a sibling of the calculated node ({node_that_needs_sibling:?})")]
    InvalidSibling {
        node_that_needs_sibling: Coordinate,
        sibling_given: Coordinate,
    },
    #[error("Too few siblings")]
    TooFewSiblings,
}

// -------------------------------------------------------------------------------------------------
// Supporting structs and methods.

/// A reference to a left sibling node.
///
/// It is like [super][sparse_binary_tree][LeftSibling] but does not own the
/// underlying node. The purpose of this type is for efficiency gains over
/// [super][sparse_binary_tree][LeftSibling] when ownership of the Node type is
/// not needed.
struct LeftSiblingRef<'a, C>(&'a Node<C>);

/// A reference to a right sibling node.
///
/// It is like [super][sparse_binary_tree][RightSibling] but does not own the
/// underlying node. The purpose of this type is for efficiency gains over
/// [super][sparse_binary_tree][RightSibling] when ownership of the Node type is
/// not needed.
struct RightSiblingRef<'a, C>(&'a Node<C>);

/// A reference to a pair of left and right sibling nodes.
///
/// It is like [super][sparse_binary_tree][MatchedPair] but does not own the
/// underlying node. The purpose of this type is for efficiency gains over
/// [super][sparse_binary_tree][MatchedPair] when ownership of the Node type is
/// not needed.
struct MatchedPairRef<'a, C> {
    left: LeftSiblingRef<'a, C>,
    right: RightSiblingRef<'a, C>,
}

impl<'a, C: Mergeable> MatchedPairRef<'a, C> {
    /// Create a parent node by merging the 2 nodes in the pair.
    fn merge(&self) -> Node<C> {
        Node {
            coord: Coordinate {
                x: self.left.0.coord.x / 2,
                y: self.left.0.coord.y + 1,
            },
            content: C::merge(&self.left.0.content, &self.right.0.content),
        }
    }

    /// Construct a [MatchedPairRef] using the 2 given nodes.
    /// Only build the pair if the 2 nodes are siblings, otherwise return an
    /// error.
    fn new(left: &'a Node<C>, right: &'a Node<C>) -> Result<Self, PathSiblingsError>
    where
        C: Clone,
    {
        if right.is_right_sibling_of(left) {
            Ok(MatchedPairRef {
                left: LeftSiblingRef(left),
                right: RightSiblingRef(right),
            })
        } else if right.is_left_sibling_of(left) {
            Ok(MatchedPairRef {
                left: LeftSiblingRef(right),
                right: RightSiblingRef(left),
            })
        } else {
            Err(PathSiblingsError::InvalidSibling {
                node_that_needs_sibling: right.coord.clone(),
                sibling_given: left.coord.clone(),
            })
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

// TODO need to test that when the node is expected to be in the store the build
// function is not called (need to have mocking for this)

// TODO Fuzz on the tree height, and the store depth.

// TODO tests for multi tree build then single path build, and vice versa.

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use crate::binary_tree::utils::test_utils::{
        full_bottom_layer, get_padding_function, single_leaf, sparse_leaves, TestContent,
    };

    // STENT TODO change name of proof
    fn check_path_siblings(tree: &BinaryTree<TestContent>, proof: &PathSiblings<TestContent>) {
        assert_eq!(proof.len() as u8, tree.height().as_y_coord());
    }

    #[test]
    fn path_works_for_full_base_layer_single_threaded() {
        let height = Height::from(8u8);

        let leaf_nodes = full_bottom_layer(&height);

        let tree_single_threaded = TreeBuilder::new()
            .with_height(height)
            .with_store_depth(MIN_STORE_DEPTH)
            .with_leaf_nodes(leaf_nodes.clone())
            .build_using_single_threaded_algorithm(get_padding_function())
            .unwrap();

        let proof = tree_single_threaded
            .path_builder()
            .with_leaf_x_coord(10)
            .build_using_single_threaded_algorithm(get_padding_function())
            .expect("PathSiblings generation should have been successful");

        check_path_siblings(&tree_single_threaded, &proof);

        proof
            .verify(tree_single_threaded.root())
            .expect("PathSiblings verification should have been successful");
    }

    #[test]
    fn path_works_for_full_base_layer_multi_threaded() {
        let height = Height::from(8u8);

        let leaf_nodes = full_bottom_layer(&height);

        let tree_multi_threaded = TreeBuilder::new()
            .with_height(height)
            .with_store_depth(MIN_STORE_DEPTH)
            .with_leaf_nodes(leaf_nodes.clone())
            .build_using_multi_threaded_algorithm(get_padding_function())
            .unwrap();

        let proof = tree_multi_threaded
            .path_builder()
            .with_leaf_x_coord(10)
            .build_using_multi_threaded_algorithm(get_padding_function())
            .expect("PathSiblings generation should have been successful");

        check_path_siblings(&tree_multi_threaded, &proof);

        proof
            .verify(tree_multi_threaded.root())
            .expect("PathSiblings verification should have been successful");
    }

    #[test]
    fn path_works_for_sparse_leaves_single_threaded() {
        let height = Height::from(8u8);

        let leaf_nodes = sparse_leaves(&height);

        let tree_single_threaded = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes.clone())
            .with_store_depth(MIN_STORE_DEPTH)
            .build_using_single_threaded_algorithm(get_padding_function())
            .unwrap();

        let proof = tree_single_threaded
            .path_builder()
            .with_leaf_x_coord(6)
            .build_using_single_threaded_algorithm(get_padding_function())
            .expect("PathSiblings generation should have been successful");

        check_path_siblings(&tree_single_threaded, &proof);

        proof
            .verify(tree_single_threaded.root())
            .expect("PathSiblings verification should have been successful");
    }

    #[test]
    fn path_works_for_sparse_leaves_multi_threaded() {
        let height = Height::from(8u8);

        let leaf_nodes = sparse_leaves(&height);

        let tree_multi_threaded = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes.clone())
            .with_store_depth(MIN_STORE_DEPTH)
            .build_using_multi_threaded_algorithm(get_padding_function())
            .unwrap();

        let proof = tree_multi_threaded
            .path_builder()
            .with_leaf_x_coord(6)
            .build_using_multi_threaded_algorithm(get_padding_function())
            .expect("PathSiblings generation should have been successful");

        check_path_siblings(&tree_multi_threaded, &proof);

        proof
            .verify(tree_multi_threaded.root())
            .expect("PathSiblings verification should have been successful");
    }

    #[test]
    fn path_works_for_single_leaf_single_threaded() {
        let height = Height::from(8u8);

        for i in 0..max_bottom_layer_nodes(&height) {
            let leaf_node = vec![single_leaf(i)];

            let tree_single_threaded = TreeBuilder::new()
                .with_height(height.clone())
                .with_leaf_nodes(leaf_node.clone())
                .with_store_depth(MIN_STORE_DEPTH)
                .build_using_single_threaded_algorithm(get_padding_function())
                .unwrap();

            let proof = tree_single_threaded
                .path_builder()
                .with_leaf_x_coord(i)
                .build_using_single_threaded_algorithm(get_padding_function())
                .expect("PathSiblings generation should have been successful");

            check_path_siblings(&tree_single_threaded, &proof);

            proof
                .verify(tree_single_threaded.root())
                .expect("PathSiblings verification should have been successful");
        }
    }

    #[test]
    fn path_works_for_multi_leaf_multi_threaded() {
        let height = Height::from(8u8);

        for i in 0..max_bottom_layer_nodes(&height) {
            let leaf_node = vec![single_leaf(i)];

            let tree_multi_threaded = TreeBuilder::new()
                .with_height(height.clone())
                .with_leaf_nodes(leaf_node.clone())
                .with_store_depth(MIN_STORE_DEPTH)
                .build_using_multi_threaded_algorithm(get_padding_function())
                .unwrap();

            let proof = tree_multi_threaded
                .path_builder()
                .with_leaf_x_coord(i)
                .build_using_multi_threaded_algorithm(get_padding_function())
                .expect("PathSiblings generation should have been successful");

            check_path_siblings(&tree_multi_threaded, &proof);

            proof
                .verify(tree_multi_threaded.root())
                .expect("PathSiblings verification should have been successful");
        }
    }
}
