mod sparse_binary_tree;
use sparse_binary_tree::{Node, Mergeable};

mod binary_tree_path;
mod dapol_node;

// STENT TODO need to have the master secret as input somewhere
// STENT TODO what are the commonalities between all the different accumulator types in the paper?

// ===========================================
// Supporting structs, types and functions.
// All objects are used in multiple sub-modules.
// Not available outside the scope of this module.

/// Used to organise nodes into left/right siblings.
// STENT TODO this should not be public but is needed to be used in paths file
pub enum NodeOrientation {
    Left,
    Right,
}

/// Used to orient nodes inside a sibling pair so that the compiler can guarantee a left node is actually a left node.
enum Sibling<C: Clone> {
    Left(LeftSibling<C>),
    Right(RightSibling<C>),
}

/// Simply holds a Node under the designated 'LeftSibling' name.
struct LeftSibling<C: Clone>(Node<C>);

/// Simply holds a Node under the designated 'RightSibling' name.
struct RightSibling<C: Clone>(Node<C>);

/// A pair of sibling nodes, but one might be absent.
struct MaybeUnmatchedPair<C: Mergeable + Clone> {
    left: Option<LeftSibling<C>>,
    right: Option<RightSibling<C>>,
}

/// A pair of sibling nodes where both are present.
struct MatchedPair<C: Mergeable + Clone> {
    left: LeftSibling<C>,
    right: RightSibling<C>,
}

// ===========================================
// Test utils for submodules.

#[cfg(test)]
mod test_utils {
    use super::sparse_binary_tree::{SparseBinaryTree, InputLeafNode, Coordinate, Mergeable};

    #[derive(Default, Clone, Debug, PartialEq)]
    pub struct TestContent {
        pub value: u32,
        pub hash: H256,
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

    pub fn get_padding_function() -> impl Fn(&Coordinate) -> TestContent {
        |_coord: &Coordinate| -> TestContent {
            TestContent {
                value: 0,
                hash: H256::default(),
            }
        }
    }

    // tree has a full bottom layer, and, subsequently, all other layers
    pub fn full_tree() -> (SparseBinaryTree<TestContent>, u8) {
        let height = 4u8;
        let mut leaves = Vec::<InputLeafNode<TestContent>>::new();

        for i in 0..2usize.pow(height as u32 - 1) {
            leaves.push(InputLeafNode::<TestContent> {
                x_coord: i as u64,
                content: TestContent {
                    hash: H256::default(),
                    value: i as u32,
                },
            });
        }

        let tree = SparseBinaryTree::new(leaves, height, &get_padding_function())
            .expect("Tree construction should not have produced an error");

        (tree, height)
    }

    // only 1 bottom-layer leaf node is present in the whole tree
    pub fn tree_with_single_leaf(x_coord_of_leaf: u64, height: u8) -> SparseBinaryTree<TestContent> {
        let leaf = InputLeafNode::<TestContent> {
            x_coord: x_coord_of_leaf,
            content: TestContent {
                hash: H256::default(),
                value: 1,
            },
        };

        let tree = SparseBinaryTree::new(vec![leaf], height, &get_padding_function())
            .expect("Tree construction should not have produced an error");

        tree
    }

    // a selection of leaves dispersed sparsely along the bottom layer
    pub fn tree_with_sparse_leaves() -> (SparseBinaryTree<TestContent>, u8) {
        let height = 5u8;

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

        let tree = SparseBinaryTree::new(
            vec![leaf_0, leaf_1, leaf_2, leaf_3],
            height,
            &get_padding_function(),
        )
        .expect("Tree construction should not have produced an error");

        (tree, height)
    }
}
