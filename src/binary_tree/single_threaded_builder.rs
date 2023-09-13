use std::collections::HashMap;

use super::{
    Coordinate, MatchedPair, Node, Sibling,
    Mergeable, LeftSibling, RightSibling,
};

/// A pair of sibling nodes, but one might be absent.
struct MaybeUnmatchedPair<C: Mergeable + Clone> {
    left: Option<LeftSibling<C>>,
    right: Option<RightSibling<C>>,
}

/// Create a new tree given the leaves, height and the padding node creation
/// function. New padding nodes are given by a closure. Why a closure?
/// Because creating a padding node may require context outside of this
/// scope, where type C is defined, for example.
// TODO there should be a warning if the height/leaves < min_sparsity (which was set to 2 in
// prev code)
pub fn build_tree<C, F>(
    nodes: Vec<Node<C>>,
    height: u8,
    new_padding_node_content: F,
) -> (HashMap<Coordinate, Node<C>>, Node<C>)
where
    C: Clone + Mergeable,
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
                    Sibling::Left(left_sibling) => pairs.push(MaybeUnmatchedPair {
                        left: Some(left_sibling),
                        right: Option::None,
                    }),
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
