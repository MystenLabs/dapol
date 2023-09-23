use super::Mergeable;
use super::{Coordinate, InputLeafNode};

// ===========================================
// Test utils for submodules.

// ===========================================
// Types

trait H256Convertible {
    fn finalize_as_h256(&self) -> H256;
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct H256(pub [u8; 32]);

#[derive(Default, Clone, Debug, PartialEq)]
pub struct TestContent {
    pub value: u32,
    pub hash: H256,
}

// ===========================================
// Implementations

impl H256 {
    fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl H256Convertible for blake3::Hasher {
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

// tree has a full bottom layer, and, subsequently, all other layers
pub fn full_tree() -> Vec<InputLeafNode<TestContent>> {
    let height: u32 = 4;

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

    leaves
}

// only 1 bottom-layer leaf node is present in the whole tree
pub fn tree_with_single_leaf(x_coord_of_leaf: u64) -> InputLeafNode<TestContent> {
    let leaf = InputLeafNode::<TestContent> {
        x_coord: x_coord_of_leaf,
        content: TestContent {
            hash: H256::default(),
            value: 1,
        },
    };

    leaf
}

// a selection of leaves dispersed sparsely along the bottom layer
pub fn tree_with_sparse_leaves() -> Vec<InputLeafNode<TestContent>> {
    // note the nodes are not in order here (wrt x-coord) so this test also somewhat covers the sorting code in the constructor
    let leaf_0 = InputLeafNode::<TestContent> {
        x_coord: 6,
        content: TestContent {
            hash: H256::default(),
            value: 1,
        },
    };
    let leaf_1 = InputLeafNode::<TestContent> {
        x_coord: 1,
        content: TestContent {
            hash: H256::default(),
            value: 2,
        },
    };
    let leaf_2 = InputLeafNode::<TestContent> {
        x_coord: 0,
        content: TestContent {
            hash: H256::default(),
            value: 3,
        },
    };
    let leaf_3 = InputLeafNode::<TestContent> {
        x_coord: 5,
        content: TestContent {
            hash: H256::default(),
            value: 4,
        },
    };

    vec![leaf_0, leaf_1, leaf_2, leaf_3]
}

pub fn get_padding_function() -> impl Fn(&Coordinate) -> TestContent {
    |_coord: &Coordinate| -> TestContent {
        TestContent {
            value: 0,
            hash: H256::default(),
        }
    }
}
