use super::Mergeable;
use super::MIN_HEIGHT;
use super::{Coordinate, InputLeafNode, MaybeUnmatchedPair, Node, SparseBinaryTreeError};

use std::collections::HashMap;

pub struct TreeBuilder<C: Clone> {
    pub nodes: Vec<Node<C>>,
    height: u8,
}

impl<C: Mergeable + Clone> TreeBuilder<C> {
    pub fn from_input_leaf_nodes(
        leaves: Vec<InputLeafNode<C>>,
        height: u8,
    ) -> Result<Self, SparseBinaryTreeError> {
        let max_leaves = 2u64.pow(height as u32 - 1);

        if leaves.len() as u64 > max_leaves {
            return Err(SparseBinaryTreeError::TooManyLeaves);
        }

        if leaves.len() < 1 {
            return Err(SparseBinaryTreeError::EmptyInput);
        }

        if height < MIN_HEIGHT {
            return Err(SparseBinaryTreeError::HeightTooSmall);
        }

        // translate InputLeafNode to Node
        let mut nodes: Vec<Node<C>> = leaves.into_iter().map(|leaf| leaf.to_node()).collect();

        // sort by x_coord ascending
        nodes.sort_by(|a, b| a.coord.x.cmp(&b.coord.x));

        // make sure all x_coord < max
        if nodes.last().is_some_and(|node| node.coord.x >= max_leaves) {
            return Err(SparseBinaryTreeError::InvalidXCoord);
        }

        // ensure no duplicates
        let duplicate_found = nodes
            .iter()
            .fold(
                (max_leaves, false),
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
            return Err(SparseBinaryTreeError::DuplicateLeaves);
        }

        Ok(Self { nodes, height })
    }

    pub fn to_parent_nodes<F>(
        &self,
        new_padding_node_content: &F,
    ) -> (Vec<Node<C>>, HashMap<Coordinate, Node<C>>)
    where
        F: Fn(&Coordinate) -> C,
    {
        let mut parent_nodes: Vec<Node<C>> = Vec::new();
        let mut store: HashMap<Coordinate, Node<C>> = HashMap::new();

        for _i in 0..self.height - 1 {
            for node in self.nodes.clone() {
                let pairs = MaybeUnmatchedPair::build_pairs(node.clone());

                for pair in pairs {
                    let matched_pair = pair.to_matched_pair(new_padding_node_content);

                    let parent = matched_pair.merge();

                    store.insert(matched_pair.left.clone().coord, matched_pair.left);
                    store.insert(matched_pair.right.clone().coord, matched_pair.right);

                    parent_nodes.push(parent);
                }
            }
        }

        (parent_nodes, store)
    }
}
