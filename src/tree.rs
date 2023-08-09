use ::std::collections::HashMap;

pub struct SparseSummationMerkleTree {
    root: Node,
    store: HashMap<Coordinate, Node>,
    height: u64,
}

impl SparseSummationMerkleTree {
    fn insert_node(&mut self, node: Node) {
        self.store.insert(node.coord.clone(), node);
    }

    // create a padding node as the right child, then merge to get the parent; insert both into the store
    fn create_parent_from_only_left_child_coord(&mut self, left_child_coord: &Coordinate) -> u64 {
        // if this is not in the store then there is a bug, so definitely panic
        let left_child = self
            .store
            .get(&left_child_coord)
            .expect("Left child coordinates given to do match a node in the store");
        let right_child = left_child.new_right_sibling_padding_node();
        let parent_node = right_child.merge_with_left_sibling(left_child);
        let parrent_node_x_coord = parent_node.coord.x;
        self.insert_node(parent_node);
        self.insert_node(right_child);
        parrent_node_x_coord
    }

    // create a padding node as the right child, then merge to get the parent; insert both into the store
    fn create_parent_from_only_left_child(&mut self, left_child: &Node) -> u64 {
        let right_child = left_child.new_right_sibling_padding_node();
        let parent_node = right_child.merge_with_left_sibling(left_child);
        let parrent_node_x_coord = parent_node.coord.x;
        self.insert_node(parent_node);
        self.insert_node(right_child);
        parrent_node_x_coord
    }
}

impl SparseSummationMerkleTree {
    fn get_node(&self, coord: &Coordinate) -> Option<&Node> {
        self.store.get(coord)
    }
}

// STENT why have an array of u8 as apposed to an array of u64?
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct H256([u8; 32]);

// STENT TODO do we really need default?
#[derive(Default, PartialEq, Eq, Hash, Debug, Clone)]
pub struct Coordinate {
    // STENT TODO make these bounded, which depends on tree height
    y: u64, // from 0 to height
    x: u64, // from 0 to 2^y
}

// STENT TODO maybe turn into enum with Internal, Padding, Node as options
// STENT TODO we should not have default here
#[derive(Default, Clone, Debug)]
pub struct Node {
    hash: H256, // STENT do we really need this? Is this the best type?
    value: u64, // STENT change to Pedersen commitment
    coord: Coordinate,
}

impl Node {
    fn get_hash_bytes(&self) -> &[u8] {
        &self.hash.0
    }

    // STENT TODO doesn't workout with the type system because the data is not stored as bytes,
    //   so the ref is created for data that immediately goes out of scope
    // fn get_value_bytes(&self) -> &[u8] {
    //     // STENT TODO is little endian correct here?
    //     &self.value.to_le_bytes()
    // }
}

impl Node {
    fn new_right_sibling_padding_node(&self) -> Self {
        Node {
            hash: H256::default(), // STENT TODO make this match the paper
            value: 0,
            coord: self.get_right_sibling_coord(),
        }
    }

    fn new_left_sibling_padding_node(&self) -> Self {
        Node {
            hash: H256::default(), // STENT TODO make this match the paper
            value: 0,
            coord: self.get_left_sibling_coord(),
        }
    }

    // create a parent node by merging this node with it's left sibling node
    fn merge_with_left_sibling(&self, left_sibling: &Node) -> Self {
        if !self.is_right_sibling() {
            panic!("This node is not a right sibling");
        }
        let h = {
            let mut hasher = blake3::Hasher::new();
            // STENT TODO is little endian correct here?
            hasher.update(&left_sibling.value.to_le_bytes());
            hasher.update(&self.value.to_le_bytes());
            hasher.update(left_sibling.get_hash_bytes());
            hasher.update(self.get_hash_bytes());
            *hasher.finalize().as_bytes() // STENT TODO double check the output of this thing
        };
        Node {
            hash: H256(h),
            value: left_sibling.value + self.value,
            coord: Coordinate {
                y: self.coord.y + 1,
                x: self.coord.x / 2,
            },
        }
    }
}

impl Node {
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
    fn is_left_sibling_of(&self, other: &Node) -> bool {
        self.coord.y == other.coord.y && self.coord.x - 1 == other.coord.x
    }

    // return true if the given node lives just to the left of self
    fn is_right_sibling_of(&self, other: &Node) -> bool {
        self.coord.y == other.coord.y && self.coord.x + 1 == other.coord.x
    }

    // self must be a right sibling, otherwise will panic
    fn get_left_sibling_coord(&self) -> Coordinate {
        if !self.is_right_sibling() {
            panic!("Cannot call this function on a left node");
        }
        Coordinate {
            y: self.coord.y,
            x: self.coord.x - 1,
        }
    }

    fn get_right_sibling_coord(&self) -> Coordinate {
        if !self.is_left_sibling() {
            panic!("Cannot call this function on a left sibling");
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

impl SparseSummationMerkleTree {
    // STENT TODO make this return a Result instead
    //   do we actually want it to return an option? why not just return the tree straight?
    pub fn new(leaves: &Vec<Node>, height: u64) -> Option<SparseSummationMerkleTree> {
        // STENT TODO check all leaves have the same y coord of 0
        let mut tree = SparseSummationMerkleTree {
            root: Node::default(), // STENT TODO get the correct node
            store: HashMap::new(),
            height,
        };
        let mut parent_layer = Vec::<u64>::new();

        // put all the leaves in the tree
        for i in 0..leaves.len() {
            let current_leaf_node = &leaves[i];

            if current_leaf_node.is_right_sibling() {
                // this leaf is a right sibling (x starts at 0)

                let right_child = current_leaf_node;

                let parent_node = if i > 0 && right_child.is_left_sibling_of(&leaves[i - 1]) {
                    // if the sibling leaf is available then use that..
                    right_child.merge_with_left_sibling(&leaves[i - 1])
                } else {
                    // ...otherwise create a padding node
                    let left_child = right_child.new_left_sibling_padding_node();
                    let parent_node = right_child.merge_with_left_sibling(&left_child);
                    tree.insert_node(left_child);
                    parent_node
                };

                parent_layer.push(parent_node.coord.x);
                tree.insert_node(parent_node);
            } else if i == leaves.len() - 1
                || !current_leaf_node.is_right_sibling_of(&leaves[i + 1])
            {
                // this leaf is a left sibling AND needs a padding node
                // if this leaf is a left sibling and does not need a padding node (i.e. right sibling is in leaves) then it will be caught by the previous if-statement in the next loop iteration

                let parent_node_x_coord =
                    tree.create_parent_from_only_left_child(&current_leaf_node);
                parent_layer.push(parent_node_x_coord);
            }

            tree.insert_node(current_leaf_node.clone());
        }

        // calculate nodes in the upper layers and put them in the tree
        // loop over all the layers (except the bottom layer), bottom to top
        for y in 1..height - 1 {
            let current_layer = parent_layer;
            parent_layer = Vec::<u64>::new();

            // loop over all the nodes
            for i in 0..current_layer.len() {
                let current_coord = Coordinate {
                    y,
                    x: current_layer[i],
                };

                if is_right_sibling(current_coord.x) {
                    // this node is a right sibling

                    // if this is not in the tree then there is a bug in the above loop, so definitely panic
                    let right_child = tree
                        .get_node(&current_coord)
                        .expect("Right sibling node should have been in the store");

                    let parent_node = if let Some(left_child) =
                        tree.get_node(&right_child.get_left_sibling_coord())
                    {
                        // if the sibling node exists in the tree then use that..
                        right_child.merge_with_left_sibling(&left_child)
                    } else {
                        // ...otherwise create a padding node
                        let left_child = right_child.new_left_sibling_padding_node();
                        let node = right_child.merge_with_left_sibling(&left_child);
                        tree.insert_node(left_child);
                        node
                    };

                    parent_layer.push(parent_node.coord.x);
                    tree.insert_node(parent_node);
                } else if i == current_layer.len() - 1
                    || !are_siblings(current_coord.x, current_layer[i + 1])
                {
                    // this node is a left sibling AND needs a padding node
                    // if this leaf is a left sibling and does not need a padding node (i.e. right sibling is in leaves) then it will be caught by the previous if-statement in the next loop iteration

                    let parent_node_x_coord =
                        tree.create_parent_from_only_left_child_coord(&current_coord);
                    parent_layer.push(parent_node_x_coord);
                }
            }
        }

        // STENT TODO we need to make sure the height of the tree is at least 3 for this to work
        let left_child = tree
            .get_node(&Coordinate {
                y: height - 2,
                x: 0,
            })
            .expect("Left child for root node not in tree");
        let right_child = tree
            .get_node(&Coordinate {
                y: height - 2,
                x: 1,
            })
            .expect("Right child for root node not in tree");
        let root_node = right_child.merge_with_left_sibling(left_child);
        tree.root = root_node.clone();
        tree.store.insert(root_node.coord.clone(), root_node);
        Some(tree)
    }

    fn create_inclusion_proof(&self, leaf: &Node) -> InclusionProof {
        self.get_node(&leaf.coord)
            .expect("Provided leaf node is not part of the tree");
        let mut siblings = Vec::<Node>::new();
        let mut current_node = leaf;

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
                .expect("Parent node not in the tree");
        }
        InclusionProof {
            leaf: leaf.clone(),
            siblings,
            root: self.root.clone(),
        }
    }
}

pub struct InclusionProof {
    leaf: Node,
    siblings: Vec<Node>,
    root: Node,
}

impl InclusionProof {
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

        if parent.hash != self.root.hash {
            panic!("Verify failed");
        }
    }
}

#[test]
pub fn stent_tree_test() {
    let height = 4;
    let leaf_1 = Node {
        hash: H256::default(),
        value: 1,
        coord: Coordinate { y: 0, x: 0 },
    };
    let leaf_2 = Node {
        hash: H256::default(),
        value: 2,
        coord: Coordinate { y: 0, x: 4 },
    };
    let leaf_3 = Node {
        hash: H256::default(),
        value: 3,
        coord: Coordinate { y: 0, x: 7 },
    };
    let input: Vec<Node> = vec![leaf_1, leaf_2, leaf_3];
    let tree = SparseSummationMerkleTree::new(&input, height).unwrap();
    for item in &tree.store {
        println!("{:?}", item);
    }

    println!("\n");

    let proof = tree.create_inclusion_proof(&input[0]);
    for item in &proof.siblings {
        println!("{:?}", item);
    }

    println!("\n");
    proof.verify();
}
