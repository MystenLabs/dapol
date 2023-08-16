/// All the sibling nodes for a leaf node's path from bottom layer to root node.
pub struct PathSiblings<C: Clone> {
    leaf: Node<C>,
    siblings: Vec<Node<C>>,
    root: Node<C>,
}

impl<C: Mergeable + Default + Clone> SparseBinaryTree<C> {
    // STENT TODO maybe we can compress by using something smaller than u64 for coords
    /// Construct the path for the leaf node living at the given x-coord on the bottom layer, and collect all the siblings to the path.
    fn get_siblings_for_path(
        &self,
        leaf_x_coord: u64,
    ) -> Result<PathSiblings<C>, PathSiblingsError> {
        let coord = Coordinate {
            x: leaf_x_coord,
            y: 0,
        };

        let leaf = self
            .get_node(&coord)
            .ok_or(PathSiblingsError::LeafNotFound)?;

        let mut current_node = leaf;
        let mut siblings = Vec::<Node<C>>::new();

        for y in 0..self.height - 1 {
            let x_coord = match current_node.node_orientation() {
                NodeOrientation::Left => current_node.coord.x + 1,
                NodeOrientation::Right => current_node.coord.x - 1,
            };

            let sibling_coord = Coordinate { y, x: x_coord };
            siblings.push(
                self.get_node(&sibling_coord)
                    .ok_or(PathSiblingsError::NodeNotFound {
                        coord: sibling_coord,
                    })?
                    .clone(),
            );

            let parent_coord = current_node.get_parent_coord();
            current_node =
                self.get_node(&parent_coord)
                    .ok_or(PathSiblingsError::NodeNotFound {
                        coord: parent_coord,
                    })?;
        }

        Ok(PathSiblings {
            leaf: leaf.clone(),
            siblings,
            root: self.root.clone(),
        })
    }
}

#[derive(Error, Debug)]
pub enum PathSiblingsError {
    #[error("Provided leaf node not found in the tree")]
    LeafNotFound,
    #[error("Node not found in tree ({coord:?})")]
    NodeNotFound { coord: Coordinate },
    #[error("Calculated root content does not match provided root content")]
    RootMismatch,
    #[error("Provided node ({given:?}) is not a sibling of the calculated node ({calculated:?})")]
    InvalidSibling {
        given: Coordinate,
        calculated: Coordinate,
    },
    #[error("Too few siblings")]
    TooFewSiblings,
}

impl<C: Mergeable + Clone> SparseBinaryTree<C> {
    /// Attempt to find a Node via it's coordinate in the underlying store.
    fn get_node(&self, coord: &Coordinate) -> Option<&Node<C>> {
        self.store.get(coord)
    }
}

impl<C: Mergeable + Clone + PartialEq + Debug> PathSiblings<C> {
    /// Verify that a path reconstruction using the leaf node and the path siblings results in the same root node content.
    fn verify(&self) -> Result<(), PathSiblingsError> {
        let mut parent = self.leaf.clone();

        if self.siblings.len() < MIN_HEIGHT as usize {
            return Err(PathSiblingsError::TooFewSiblings);
        }

        for node in &self.siblings {
            let pair = if parent.is_right_sibling_of(node) {
                Ok(MatchedPairRef {
                    left: LeftSiblingRef(node),
                    right: RightSiblingRef(&parent),
                })
            } else if parent.is_left_sibling_of(node) {
                Ok(MatchedPairRef {
                    left: LeftSiblingRef(&parent),
                    right: RightSiblingRef(node),
                })
            } else {
                Err(PathSiblingsError::InvalidSibling {
                    given: node.coord.clone(),
                    calculated: parent.coord,
                })
            }?;
            parent = pair.merge();
        }

        if parent.content == self.root.content {
            Ok(())
        } else {
            Err(PathSiblingsError::RootMismatch)
        }
    }
}

// ===========================================
// Unit tests.

#[cfg(test)]
mod tests {
    use super::super::sparse_binary_tree::tests::{
        tree_with_single_leaf, tree_with_sparse_leaves, TestContent,
    };
    use super::*;

    fn check_path_siblings(
        tree: &SparseBinaryTree<TestContent>,
        proof: &PathSiblings<TestContent>,
    ) {
        assert_eq!(tree.root, proof.root);
        assert_eq!(proof.siblings.len() as u32, tree.height - 1);
    }

    #[test]
    fn tree_works_for_full_base_layer() {
        let (tree, height) = full_tree();

        let proof = tree
            .get_siblings_for_path(0)
            .expect("Path generation should have been successful");
        check_path_siblings(&tree, &proof);

        proof
            .verify()
            .expect("Path verification should have been successful");
    }

    #[test]
    fn proofs_work_for_sparse_leaves() {
        let tree = tree_with_sparse_leaves();
        let proof = tree
            .get_siblings_for_path(6)
            .expect("Path generation should have been successful");
        check_path_siblings(&tree, &proof);

        proof
            .verify()
            .expect("Path verification should have been successful");
    }

    #[test]
    fn proofs_work_for_single_leaf() {
        let height = 4;

        for i in 0..2usize.pow(height - 1) {
            let tree = tree_with_single_leaf(i as u64, height);
            let proof = tree
                .get_siblings_for_path(i as u64)
                .expect("Path generation should have been successful");
            check_path_siblings(&tree, &proof);

            proof
                .verify()
                .expect("Path verification should have been successful");
        }
    }
}
