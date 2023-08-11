use ::std::collections::HashMap;
use curve25519_dalek_ng::ristretto::RistrettoPoint;
use digest::Digest;
use std::marker::PhantomData;

// STENT TODO need to have the master secret as input somewhere
// STENT TODO what are the commonalities between all the different accumulator types in the paper?

/// Trait for merging two child nodes to extract the parent node in the SMT.
pub trait Mergeable {
    /// A function to merge two child nodes as the parent node in the SMT.
    fn merge_with_left_sibling(&self, right_child: &Self) -> Self;
}

/// Node content data for the DAPOL+ protocol, consisting of the Pedersen commitment and the hash.
#[derive(Default, Clone, Debug)]
pub struct DapolNodeContent<H> {
    commitment: RistrettoPoint,
    hash: H256,
    _phantom_hash_function: PhantomData<H>,
}

impl<H> PartialEq for DapolNodeContent<H> {
    fn eq(&self, other: &Self) -> bool {
        self.commitment == other.commitment && self.hash == other.hash
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

impl<H: Digest + H256Convertable> Mergeable for DapolNodeContent<H> {
    /// Returns the parent node by merging two child nodes.
    ///
    /// The commitment of the parent is the homomorphic sum of the two children.
    /// The hash of the parent is computed by hashing the concatenated commitments and hashes of two children.
    fn merge_with_left_sibling(&self, right_child: &Self) -> Self {
        // C(parent) = C(L) + C(R)
        let parent_commitment = self.commitment + right_child.commitment;

        // H(parent) = Hash(C(L) | C(R) | H(L) | H(R))
        let parent_hash = {
            let mut hasher = H::new();
            hasher.update(self.commitment.compress().as_bytes());
            hasher.update(right_child.commitment.compress().as_bytes());
            hasher.update(self.hash.as_bytes());
            hasher.update(right_child.hash.as_bytes());
            hasher.finalize_as_h256() // STENT TODO double check the output of this thing
        };

        DapolNodeContent {
            commitment: parent_commitment,
            hash: parent_hash,
            _phantom_hash_function: PhantomData,
        }
    }
}

pub struct SparseSummationMerkleTree<C: Clone> {
    root: Node<C>,
    store: HashMap<Coordinate, Node<C>>,
    height: u64,
}

impl<C: Mergeable + Clone> SparseSummationMerkleTree<C> {
    fn insert_node(&mut self, node: Node<C>) {
        self.store.insert(node.coord.clone(), node);
    }

    // create a padding node as the right child, then merge to get the parent; insert both into the store
    fn create_parent_from_only_left_child_coord<F>(
        &mut self,
        left_child_coord: &Coordinate,
        new_padding_node_content: &F,
    ) -> u64
    where
        F: Fn(&Coordinate) -> C,
    {
        // if this is not in the store then there is a bug, so definitely panic
        let left_child = self
            .store
            .get(&left_child_coord)
            .expect("Left child coordinates given to do match a node in the store");
        let right_child = left_child.new_right_sibling_padding_node(new_padding_node_content);
        let parent_node = right_child.merge_with_left_sibling(left_child);
        let parrent_node_x_coord = parent_node.coord.x;
        self.insert_node(parent_node);
        self.insert_node(right_child);
        parrent_node_x_coord
    }

    // create a padding node as the right child, then merge to get the parent; insert both into the store
    fn create_parent_from_only_left_child<F>(
        &mut self,
        left_child: &Node<C>,
        new_padding_node_content: &F,
    ) -> u64
    where
        F: Fn(&Coordinate) -> C,
    {
        let right_child = left_child.new_right_sibling_padding_node(new_padding_node_content);
        let parent_node = right_child.merge_with_left_sibling(left_child);
        let parrent_node_x_coord = parent_node.coord.x;
        self.insert_node(parent_node);
        self.insert_node(right_child);
        parrent_node_x_coord
    }
}

impl<C: Clone> SparseSummationMerkleTree<C> {
    fn get_node(&self, coord: &Coordinate) -> Option<&Node<C>> {
        self.store.get(coord)
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct H256([u8; 32]);

impl H256 {
    fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

// STENT TODO maybe rename this to TreeIndex
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Coordinate {
    // STENT TODO make these bounded, which depends on tree height
    y: u64, // from 0 to height
    x: u64, // from 0 to 2^y
}

// STENT TODO maybe turn into enum with Internal, Padding, Node as options
#[derive(Clone, Debug)]
pub struct Node<C: Clone> {
    coord: Coordinate,
    content: C,
}

impl<C: Default + Clone> Node<C> {
    fn default_root_node(height: u64) -> Self {
        Node {
            coord: Coordinate { y: height, x: 0 },
            content: C::default(),
        }
    }
}

impl<C: Mergeable + Clone> Node<C> {
    // STENT TODO this closure's implementation of the padding functionality is experimental. Should we rather have a struct-wide generic type? what about the hash function for the merge? it has a similar situation
    //  it seems this is the best way because it can take _multiple_ contexts (coord + whatever context where it is defined)
    fn new_right_sibling_padding_node<F>(&self, new_padding_node_content: &F) -> Self
    where
        F: Fn(&Coordinate) -> C,
    {
        if !self.is_left_sibling() {
            panic!(
                "This node is not a left sibling so a right sibling padding node cannot be created"
            );
        }
        let coord = self.get_right_sibling_coord();
        let content = new_padding_node_content(&coord);
        Node { coord, content }
    }

    fn new_left_sibling_padding_node<F>(&self, new_padding_node_content: &F) -> Self
    where
        F: Fn(&Coordinate) -> C,
    {
        // STENT TODO we can potentially get rid of these panics by having a left and right sibling type (enums)
        if !self.is_right_sibling() {
            panic!(
                "This node is not a right sibling so a left sibling padding node cannot be created"
            );
        }
        let coord = self.get_left_sibling_coord();
        let content = new_padding_node_content(&coord);
        Node { coord, content }
    }

    // create a parent node by merging this node with it's left sibling node
    fn merge_with_left_sibling(&self, left_sibling: &Node<C>) -> Self {
        if !self.is_right_sibling() {
            panic!("This node is not a right sibling");
        }
        Node {
            coord: Coordinate {
                y: self.coord.y + 1,
                x: self.coord.x / 2,
            },
            content: self.content.merge_with_left_sibling(&left_sibling.content),
        }
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
        self.coord.y == other.coord.y && self.coord.x - 1 == other.coord.x
    }

    // return true if the given node lives just to the left of self
    fn is_right_sibling_of(&self, other: &Node<C>) -> bool {
        self.coord.y == other.coord.y && self.coord.x + 1 == other.coord.x
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

// since we are working with a binary tree we can tell if the node is a right sibling of the above layer by checking the x_coord modulus 2
// since x_coord starts from 0 we check if the modulus is equal to 1
fn is_right_sibling(x_coord: u64) -> bool {
    x_coord % 2 == 1
}

// return true if the 2 provided x_coords are consecutive i.e. if the nodes are neighbours/siblings
fn are_siblings(left_x_coord: u64, right_x_coord: u64) -> bool {
    left_x_coord + 1 == right_x_coord
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

struct MaybeUnmatchedPair<C: Clone> {
    left: Option<Node<C>>,
    right: Option<Node<C>>,
}

struct MatchedPair<C: Clone> {
    left: Node<C>,
    right: Node<C>,
}

impl<C: Mergeable + Default + Clone> SparseSummationMerkleTree<C> {
    // STENT TODO make this return a Result instead
    //   do we actually want it to return an option? why not just return the tree straight?
    pub fn new<F>(
        // STENT TODO need to make sure the number of leaves is less than 2^height
        leaves: Vec<InputLeafNode<C>>,
        height: u64,
        new_padding_node_content: &F,
    ) -> Option<SparseSummationMerkleTree<C>>
    where
        F: Fn(&Coordinate) -> C,
    {
        let mut tree = SparseSummationMerkleTree {
            root: Node::default_root_node(height),
            store: HashMap::new(),
            height,
        };
        let mut parent_layer = Vec::<Node<C>>::new();

        // STENT TODO make sure leaves are sorted by x_coord

        // translate InputLeafNode to Node
        let mut nodes: Vec<Node<C>> = leaves.into_iter().map(|leaf| leaf.to_node()).collect();
        while nodes.len() > 1 {
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
                        right: left.new_right_sibling_padding_node(new_padding_node_content),
                        left,
                    },
                    (None, Some(right)) => MatchedPair {
                        left: right.new_left_sibling_padding_node(new_padding_node_content),
                        right,
                    },
                    // if this case is reached then there is a bug in the above fold
                    (None, None) => {
                        panic!("[Bug in tree constructor] Invalid pair (None, None) found")
                    }
                })
                // create parents for the next loop iteration, and add the pairs to the tree
                .map(|pair| {
                    let parent = pair.right.merge_with_left_sibling(&pair.left);
                    tree.insert_node(pair.left);
                    tree.insert_node(pair.right);
                    parent
                })
                .collect();
        }

        let root = nodes.pop().expect("Unable to find root node");
        tree.root = root.clone();
        tree.store.insert(root.coord.clone(), root);
        Some(tree)
    }

    // STENT TODO maybe we can compress by using something smaller than u64 for coords
    fn create_inclusion_proof(&self, leaf_x_coord: u64) -> InclusionProof<C> {
        let coord = Coordinate {
            x: leaf_x_coord,
            y: 0,
        };

        let leaf = self
            .get_node(&coord)
            // STENT TODO better error message
            .expect("Cannot find leaf node in the tree");

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
                    .expect("Sibling node not in the tree")
                    .clone(),
            );
            current_node = &self
                .get_node(&current_node.get_parent_coord())
                // STENT TODO better error message
                .expect("Parent node not in the tree");
        }
        InclusionProof {
            leaf: leaf.clone(),
            siblings,
            root: self.root.clone(),
        }
    }
}

pub struct InclusionProof<C: Clone> {
    leaf: Node<C>,
    siblings: Vec<Node<C>>,
    root: Node<C>,
}

impl<C: Mergeable + Clone + PartialEq> InclusionProof<C> {
    fn verify(&self) {
        let mut parent = self.leaf.clone();

        for node in &self.siblings {
            let (left_child, right_child) = if parent.is_right_sibling() {
                (node, &parent)
            } else {
                (&parent, node)
            };
            parent = right_child.merge_with_left_sibling(left_child);
        }

        if parent.content != self.root.content {
            panic!("Verify failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use bulletproofs::PedersenGens;
    use curve25519_dalek_ng::scalar::Scalar;

    use super::*;

    #[test]
    pub fn stent_tree_test() {
        let height = 4;
        let v_blinding = Scalar::from(8_u32);

        let new_padding_node_content = |coord: &Coordinate| -> DapolNodeContent<blake3::Hasher> {
            DapolNodeContent {
                commitment: PedersenGens::default()
                    .commit(Scalar::from(3_u32), Scalar::from(0_u32)),
                hash: H256::default(),
                _phantom_hash_function: PhantomData,
            }
        };

        let leaf_1 = InputLeafNode::<DapolNodeContent<blake3::Hasher>> {
            x_coord: 0,
            content: DapolNodeContent {
                hash: H256::default(),
                commitment: PedersenGens::default().commit(Scalar::from(0_u32), v_blinding),
                _phantom_hash_function: PhantomData,
            },
        };
        let leaf_2 = InputLeafNode::<DapolNodeContent<blake3::Hasher>> {
            x_coord: 4,
            content: DapolNodeContent {
                hash: H256::default(),
                commitment: PedersenGens::default().commit(Scalar::from(2_u32), v_blinding),
                _phantom_hash_function: PhantomData,
            },
        };
        let leaf_3 = InputLeafNode::<DapolNodeContent<blake3::Hasher>> {
            x_coord: 7,
            content: DapolNodeContent {
                hash: H256::default(),
                commitment: PedersenGens::default().commit(Scalar::from(3_u32), v_blinding),
                _phantom_hash_function: PhantomData,
            },
        };
        let input = vec![leaf_1, leaf_2, leaf_3];
        let tree =
            SparseSummationMerkleTree::new(input, height, &new_padding_node_content).unwrap();
        for item in &tree.store {
            println!("coord {:?} hash {:?}", item.1.coord, item.1.content.hash);
        }

        println!("\n");

        let proof = tree.create_inclusion_proof(0);
        for item in &proof.siblings {
            println!("coord {:?} hash {:?}", item.coord, item.content.hash);
        }

        println!("\n");
        proof.verify();
    }
}
