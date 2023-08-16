/// All the siblings for a leaf node's path from bottom layer to root node.
// STENT TODO change name to path
pub struct InclusionProof<C: Clone> {
    leaf: Node<C>,
    siblings: Vec<Node<C>>,
    root: Node<C>,
}

impl<C: Mergeable + Default + Clone> SparseBinaryTree<C> {
    // STENT TODO maybe we can compress by using something smaller than u64 for coords
    /// Construct the sibling path for the leaf nod living at the given x-coord on the bottom layer.
    fn create_inclusion_proof(
        &self,
        leaf_x_coord: u64,
    ) -> Result<InclusionProof<C>, InclusionProofError> {
        let coord = Coordinate {
            x: leaf_x_coord,
            y: 0,
        };

        let leaf = self
            .get_node(&coord)
            .ok_or(InclusionProofError::LeafNotFound)?;

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
                    .ok_or(InclusionProofError::NodeNotFound {
                        coord: sibling_coord,
                    })?
                    .clone(),
            );

            let parent_coord = current_node.get_parent_coord();
            current_node =
                self.get_node(&parent_coord)
                    .ok_or(InclusionProofError::NodeNotFound {
                        coord: parent_coord,
                    })?;
        }

        Ok(InclusionProof {
            leaf: leaf.clone(),
            siblings,
            root: self.root.clone(),
        })
    }
}

#[derive(Error, Debug)]
pub enum InclusionProofError {
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

impl<C: Mergeable + Clone + PartialEq + Debug> InclusionProof<C> {
    fn verify(&self) -> Result<(), InclusionProofError> {
        let mut parent = self.leaf.clone();

        if self.siblings.len() < MIN_HEIGHT as usize {
            return Err(InclusionProofError::TooFewSiblings);
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
                Err(InclusionProofError::InvalidSibling {
                    given: node.coord.clone(),
                    calculated: parent.coord,
                })
            }?;
            parent = pair.merge();
        }

        if parent.content == self.root.content {
            Ok(())
        } else {
            Err(InclusionProofError::RootMismatch)
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

    fn check_inclusion_proof(
        tree: &SparseBinaryTree<TestContent>,
        proof: &InclusionProof<TestContent>,
    ) {
        assert_eq!(tree.root, proof.root);
        assert_eq!(proof.siblings.len() as u32, tree.height - 1);
    }

    #[test]
    fn tree_works_for_full_base_layer() {
        let (tree, height) = full_tree();

        let proof = tree
            .create_inclusion_proof(0)
            .expect("Inclusion proof generation should have been successful");
        check_inclusion_proof(&tree, &proof);

        proof
            .verify()
            .expect("Inclusion proof verification should have been successful");
    }

    #[test]
    fn proofs_work_for_sparse_leaves() {
        let tree = tree_with_sparse_leaves();
        let proof = tree
            .create_inclusion_proof(6)
            .expect("Inclusion proof generation should have been successful");
        check_inclusion_proof(&tree, &proof);

        proof
            .verify()
            .expect("Inclusion proof verification should have been successful");
    }

    #[test]
    fn proofs_work_for_single_leaf() {
        let height = 4;

        for i in 0..2usize.pow(height - 1) {
            let tree = tree_with_single_leaf(i as u64, height);
            let proof = tree
                .create_inclusion_proof(i as u64)
                .expect("Inclusion proof generation should have been successful");
            check_inclusion_proof(&tree, &proof);

            proof
                .verify()
                .expect("Inclusion proof verification should have been successful");
        }
    }
}
