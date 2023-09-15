//! Ease of use functions to make cleaner code.

// -------------------------------------------------------------------------------------------------
// Traits for Option.

pub trait ErrOnSome {
    fn err_on_some<E>(&self, err: E) -> Result<(), E>;
}

/// Return an error if `Some(_)`, otherwise do nothing.
impl<T> ErrOnSome for Option<T> {
    fn err_on_some<E>(&self, err: E) -> Result<(), E>
    {
        match self {
            None => Ok(()),
            Some(_) => Err(err),
        }
    }
}

pub trait ErrUnlessTrue {
    fn err_unless_true<E>(&self, err: E) -> Result<(), E>;
}

/// Return an error if `None` or `Some(false)`, otherwise do nothing.
impl ErrUnlessTrue for Option<bool> {
    fn err_unless_true<E>(&self, err: E) -> Result<(), E>
    {
        match self {
            None => Err(err),
            Some(false) => Err(err),
            Some(true) => Ok(()),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Helper functions.

/// The maximum number of leaf nodes on the bottom layer of the binary tree.
/// TODO latex `max = 2^(height-1)`
// TODO change name to 'max'
pub fn num_bottom_layer_nodes(height: u8) -> u64 {
    2u64.pow(height as u32 - 1)
}

// -------------------------------------------------------------------------------------------------
// Test utils for sub-modules.

#[cfg(test)]
mod test_utils {
    use super::super::*;
    use primitive_types::H256;

    #[derive(Default, Clone, Debug, PartialEq)]
    pub struct TestContent {
        pub value: u32,
        pub hash: H256,
    }

    pub trait H256Finalizable {
        fn finalize_as_h256(&self) -> H256;
    }

    impl H256Finalizable for blake3::Hasher {
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
                hasher.finalize_as_h256() // TODO double check the output of
                                          // this thing
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
    pub fn full_tree() -> (BinaryTree<TestContent>, u8) {
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

        let tree = BinaryTree::new(leaves, height, &get_padding_function())
            .expect("Tree construction should not have produced an error");

        (tree, height)
    }

    // only 1 bottom-layer leaf node is present in the whole tree
    pub fn tree_with_single_leaf(x_coord_of_leaf: u64, height: u8) -> BinaryTree<TestContent> {
        let leaf = InputLeafNode::<TestContent> {
            x_coord: x_coord_of_leaf,
            content: TestContent {
                hash: H256::default(),
                value: 1,
            },
        };

        let tree = BinaryTree::new(vec![leaf], height, &get_padding_function())
            .expect("Tree construction should not have produced an error");

        tree
    }

    // a selection of leaves dispersed sparsely along the bottom layer
    pub fn tree_with_sparse_leaves() -> (BinaryTree<TestContent>, u8) {
        let height = 5u8;

        // note the nodes are not in order here (wrt x-coord) so this test also somewhat
        // covers the sorting code in the constructor
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

        let tree = BinaryTree::new(
            vec![leaf_0, leaf_1, leaf_2, leaf_3],
            height,
            &get_padding_function(),
        )
        .expect("Tree construction should not have produced an error");

        (tree, height)
    }
}

