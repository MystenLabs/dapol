use ::std::collections::HashMap;
use ::std::fmt::Debug;
use thiserror::Error;

use crate::tree::errors::{TreeError, TreeResult};

static MIN_HEIGHT: usize = 2;

pub trait Mergeable {
    fn merge(left_sibling: &Self, right_sibling: &Self) -> Self;
}

pub struct SparseBinaryTree<C: Clone> {
    root: Node<C>,
    store: HashMap<Coordinate, Node<C>>,
    height: u32,
}

impl<C: Mergeable + Clone> SparseBinaryTree<C> {
    fn get_node(&self, coord: &Coordinate) -> Option<&Node<C>> {
        self.store.get(coord)
    }
}

// STENT TODO maybe rename this to TreeIndex
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Coordinate {
    // STENT TODO make these bounded, which depends on tree height
    y: u32, // from 0 to height
    x: u64, // from 0 to 2^y
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node<C: Clone> {
    coord: Coordinate,
    content: C,
}

impl<C: Default + Clone> Node<C> {
    fn default_root_node(height: u32) -> Self {
        Node {
            coord: Coordinate { y: height, x: 0 },
            content: C::default(),
        }
    }
}

impl<C: Mergeable + Clone> Node<C> {
    // STENT TODO this closure's implementation of the padding functionality is experimental. Should we rather have a struct-wide generic type? what about the hash function for the merge? it has a similar situation
    //  it seems this is the best way because it can take _multiple_ contexts (coord + whatever context where it is defined)
    fn new_sibling_padding_node<F>(&self, new_padding_node_content: &F) -> Self
    where
        F: Fn(&Coordinate) -> C,
    {
        let coord = if self.is_left_sibling() {
            self.get_right_sibling_coord()
        } else {
            self.get_left_sibling_coord()
        };
        let content = new_padding_node_content(&coord);
        Node { coord, content }
    }
}

impl<C: Clone> Node<C> {
    // returns true if this node is a right sibling
    // since we are working with a binary tree we can tell if the node is a right sibling of the above layer by checking the x_coord modulus 2
    // since x_coord starts from 0 we check if the modulus is equal to 1
    fn is_right_sibling(&self) -> bool {
        self.coord.x % 2 == 1
    }

    // returns true if this node is a left sibling
    // since we are working with a binary tree we can tell if the node is a right sibling of the above layer by checking the x_coord modulus 2
    // since x_coord starts from 0 we check if the modulus is equal to 0
    fn is_left_sibling(&self) -> bool {
        self.coord.x % 2 == 0
    }

    // return true if the given node lives just to the right of self
    fn is_left_sibling_of(&self, other: &Node<C>) -> bool {
        self.is_left_sibling() && self.coord.y == other.coord.y && self.coord.x + 1 == other.coord.x
    }

    // return true if the given node lives just to the left of self
    fn is_right_sibling_of(&self, other: &Node<C>) -> bool {
        self.is_right_sibling() && self.coord.x > 0 && self.coord.y == other.coord.y && self.coord.x - 1 == other.coord.x
    }

    // self must be a right sibling, otherwise will panic
    fn get_left_sibling_coord(&self) -> Coordinate {
        if !self.is_right_sibling() {
            panic!("Cannot call this function on a left sibling");
        }
        Coordinate {
            y: self.coord.y,
            x: self.coord.x - 1,
        }
    }

    fn get_right_sibling_coord(&self) -> Coordinate {
        if !self.is_left_sibling() {
            panic!("Cannot call this function on a right sibling");
        }
        Coordinate {
            y: self.coord.y,
            x: self.coord.x + 1,
        }
    }

    fn get_parent_coord(&self) -> Coordinate {
        Coordinate {
            y: self.coord.y + 1,
            x: self.coord.x / 2,
        }
    }
}

pub struct InputLeafNode<C> {
    content: C,
    x_coord: u64,
}

impl<C: Clone> InputLeafNode<C> {
    fn to_node(self) -> Node<C> {
        Node {
            content: self.content,
            coord: Coordinate {
                x: self.x_coord,
                y: 0,
            },
        }
    }
}

struct MaybeUnmatchedPair<C: Mergeable + Clone> {
    left: Option<Node<C>>,
    right: Option<Node<C>>,
}

struct MatchedPair<C: Mergeable + Clone> {
    left: Node<C>,
    right: Node<C>,
}

impl<C: Mergeable + Clone> MatchedPair<C> {
    // create a parent node by merging this node with it's sibling
    // STENT TODO note that this makes the assumption that the other node is a sibling
    //   maybe we can have the merge function on the pair type rather
    fn merge(&self) -> Node<C> {
        MatchedPairRef {
            left: &self.left,
            right: &self.right,
        }
        .merge()
    }
}

struct MatchedPairRef<'a, C: Mergeable + Clone> {
    left: &'a Node<C>,
    right: &'a Node<C>,
}

impl<'a, C: Mergeable + Clone> MatchedPairRef<'a, C> {
    // create a parent node by merging this node with it's sibling
    fn merge(&self) -> Node<C> {
        assert!(self.left.is_left_sibling_of(self.right), "STENT TODO");
        Node {
            coord: Coordinate {
                y: self.left.coord.y + 1,
                x: self.left.coord.x / 2,
            },
            content: C::merge(&self.left.content, &self.right.content),
        }
    }
}

impl<C: Mergeable + Default + Clone> SparseBinaryTree<C> {
    pub fn new<F>(
        leaves: Vec<InputLeafNode<C>>,
        height: u32,
        new_padding_node_content: &F,
    ) -> SparseBinaryTree<C>
    where
        F: Fn(&Coordinate) -> C,
    {
        let max_leaves = 2usize.pow(height - 1);
        // STENT TODO possibly return Result instead
        assert!(
            leaves.len() <= max_leaves,
            "Too many leaves for the given height"
        );
        let mut store = HashMap::new();

        // translate InputLeafNode to Node
        let mut nodes: Vec<Node<C>> = leaves.into_iter().map(|leaf| leaf.to_node()).collect();
        // STENT TODO check this sorts in the correct direction
        nodes.sort_by(|a, b| a.coord.x.cmp(&b.coord.x));

        for _i in 0..height - 1 {
            nodes = nodes
                .into_iter()
                // sort nodes into pairs (left & right siblings)
                .fold(Vec::<MaybeUnmatchedPair<C>>::new(), |mut pairs, node| {
                    if node.is_left_sibling() {
                        pairs.push(MaybeUnmatchedPair {
                            left: Some(node),
                            right: Option::None,
                        });
                    } else {
                        let is_right_sibling_of_prev_node = pairs
                            .last_mut()
                            .map(|pair| (&pair.left).as_ref())
                            .flatten()
                            .is_some_and(|left| left.coord.x + 1 == node.coord.x);
                        if is_right_sibling_of_prev_node {
                            pairs
                                .last_mut()
                                // this case should never be reached because of the way is_right_sibling_of_prev_node is built
                                .expect("[Bug in tree constructor] Previous node not found")
                                .right = Option::Some(node);
                        } else {
                            pairs.push(MaybeUnmatchedPair {
                                left: Option::None,
                                right: Some(node),
                            });
                        }
                    }
                    pairs
                })
                .into_iter()
                // add padding nodes to unmatched pairs
                .map(|pair| match (pair.left, pair.right) {
                    (Some(left), Some(right)) => MatchedPair { left, right },
                    (Some(left), None) => MatchedPair {
                        right: left.new_sibling_padding_node(new_padding_node_content),
                        left,
                    },
                    (None, Some(right)) => MatchedPair {
                        left: right.new_sibling_padding_node(new_padding_node_content),
                        right,
                    },
                    // if this case is reached then there is a bug in the above fold
                    (None, None) => {
                        panic!("[Bug in tree constructor] Invalid pair (None, None) found")
                    }
                })
                // create parents for the next loop iteration, and add the pairs to the tree
                .map(|pair| {
                    let parent = pair.merge();
                    // STENT TODO not sure if we can get rid of these clones
                    store.insert(pair.left.coord.clone(), pair.left);
                    store.insert(pair.right.coord.clone(), pair.right);
                    parent
                })
                .collect();
        }

        // if the root node is not present then there is a bug in the above code
        let root = nodes
            .pop()
            .expect("[Bug in tree constructor] Unable to find root node");

        assert!(
            nodes.len() == 0,
            "[Bug in tree constructor] Should be no nodes left to process"
        );

        store.insert(root.coord.clone(), root.clone());

        SparseBinaryTree {
            root,
            store,
            height,
        }
    }
}

// STENT TODO maybe put all this inclusion stuff in a different module/file
//   what is the best practice here?

pub struct InclusionProof<C: Clone> {
    leaf: Node<C>,
    siblings: Vec<Node<C>>,
    root: Node<C>,
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

impl<C: Mergeable + Default + Clone> SparseBinaryTree<C> {
    // STENT TODO maybe we can compress by using something smaller than u64 for coords
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
            let x_coord = if current_node.is_right_sibling() {
                current_node.coord.x - 1
            } else {
                current_node.coord.x + 1
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

impl<C: Mergeable + Clone + PartialEq + Debug> InclusionProof<C> {
    fn verify(&self) -> Result<(), InclusionProofError> {
        let mut parent = self.leaf.clone();

        if self.siblings.len() < MIN_HEIGHT {
            return Err(InclusionProofError::TooFewSiblings);
        }

        for node in &self.siblings {
            let pair = if parent.is_right_sibling_of(node) {
                Ok(MatchedPairRef {
                    left: node,
                    right: &parent,
                })
            } else if parent.is_left_sibling_of(node) {
                Ok(MatchedPairRef {
                    left: &parent,
                    right: node,
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

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Default, Clone, Debug, PartialEq)]
    pub struct TestContent {
        value: u32,
        hash: H256,
    }

    #[derive(Default, Clone, Debug, PartialEq, Eq)]
    pub struct H256([u8; 32]);

    impl H256 {
        fn as_bytes(&self) -> &[u8; 32] {
            &self.0
        }
    }

    pub trait H256Convertable {
        fn finalize_as_h256(&self) -> H256;
    }

    impl H256Convertable for blake3::Hasher {
        fn finalize_as_h256(&self) -> H256 {
            H256(self.finalize().as_bytes().clone())
        }
    }

    impl Mergeable for TestContent {
        fn merge(left_sibling: &Self, right_sibling: &Self) -> Self {
            // C(parent) = C(L) + C(R)
            let parent_value = left_sibling.value + right_sibling.value;

            // H(parent) = Hash(C(L) | C(R) | H(L) | H(R))
            let parent_hash = {
                let mut hasher = blake3::Hasher::new();
                hasher.update(&left_sibling.value.to_le_bytes());
                hasher.update(&right_sibling.value.to_le_bytes());
                hasher.update(left_sibling.hash.as_bytes());
                hasher.update(right_sibling.hash.as_bytes());
                hasher.finalize_as_h256() // STENT TODO double check the output of this thing
            };

            TestContent {
                value: parent_value,
                hash: parent_hash,
            }
        }
    }

    fn get_padding_function() -> impl Fn(&Coordinate) -> TestContent {
        |_coord: &Coordinate| -> TestContent {
            TestContent {
                value: 0,
                hash: H256::default(),
            }
        }
    }

    fn check_tree(tree: &SparseBinaryTree<TestContent>, height: u32) {
        assert_eq!(tree.height, height);
    }

    fn check_inclusion_proof(
        tree: &SparseBinaryTree<TestContent>,
        proof: &InclusionProof<TestContent>,
    ) {
        assert_eq!(tree.root, proof.root);
        assert_eq!(proof.siblings.len() as u32, tree.height - 1);
    }

    // STENT TODO get rid of the prints in this test
    #[test]
    fn tree_works_for_full_base_layer() {
        let height = 4;

        let mut leaves = Vec::<InputLeafNode<TestContent>>::new();

        for i in 0..2usize.pow(height - 1) {
            leaves.push(InputLeafNode::<TestContent> {
                x_coord: i as u64,
                content: TestContent {
                    hash: H256::default(),
                    value: i as u32,
                },
            });
        }
        println!("leaves size {}", leaves.len());

        let tree = SparseBinaryTree::new(leaves, height, &get_padding_function());
        check_tree(&tree, height);
        for item in &tree.store {
            println!(
                "coord {:?} value {:?} hash {:?}",
                item.1.coord, item.1.content.value, item.1.content.hash
            );
        }

        println!("\n");

        let proof = tree
            .create_inclusion_proof(0)
            .expect("Inclusion proof generation should have been successful");
        check_inclusion_proof(&tree, &proof);

        println!("num siblings in proof {:?}", proof.siblings.len());
        for item in &proof.siblings {
            println!(
                "coord {:?} value {:?} hash {:?}",
                item.coord, item.content.value, item.content.hash
            );
        }

        println!("\n");
        proof
            .verify()
            .expect("Inclusion proof verification should have been successful");
    }

    #[test]
    fn tree_works_for_single_leaf() {
        let height = 4;

        for i in 0..2usize.pow(height - 1) {
            let leaf = InputLeafNode::<TestContent> {
                x_coord: i as u64,
                content: TestContent {
                    hash: H256::default(),
                    value: 1,
                },
            };

            let tree = SparseBinaryTree::new(vec![leaf], height, &get_padding_function());
            check_tree(&tree, height);

            let proof = tree
                .create_inclusion_proof(i as u64)
                .expect("Inclusion proof generation should have been successful");
            check_inclusion_proof(&tree, &proof);

            proof
                .verify()
                .expect("Inclusion proof verification should have been successful");
        }
    }

    // STENT TODO test to see if too many nodes gives error
    // STENT TODO test all edge cases where the first and last 2 nodes are either all present or all not or partially present
}
