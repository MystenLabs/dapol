//! Ease of use functions to make cleaner code.

// -------------------------------------------------------------------------------------------------
// Traits for Option.

pub trait ErrOnSome {
    fn err_on_some<E>(&self, err: E) -> Result<(), E>;
}

/// Return an error if `Some(_)`, otherwise do nothing.
impl<T> ErrOnSome for Option<T> {
    fn err_on_some<E>(&self, err: E) -> Result<(), E> {
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
    fn err_unless_true<E>(&self, err: E) -> Result<(), E> {
        match self {
            None => Err(err),
            Some(false) => Err(err),
            Some(true) => Ok(()),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Helper functions.

use super::Height;

/// The maximum number of leaf nodes on the bottom layer of the binary tree.
/// TODO latex `max = 2^(height-1)`
pub fn max_bottom_layer_nodes(height: &Height) -> u64 {
    2u64.pow(height.as_u32() - 1)
}

// -------------------------------------------------------------------------------------------------
// Test utils for sub-modules.

#[cfg(test)]
pub mod test_utils {
    use super::super::*;
    use primitive_types::H256;

    #[derive(Clone, Debug, PartialEq, Serialize)]
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
                hasher.finalize_as_h256()
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

    // If the tree has a full bottom layer then all other layers will also be
    // full (if construction is correct).
    pub fn full_bottom_layer(height: &Height) -> Vec<InputLeafNode<TestContent>> {
        let mut leaf_nodes = Vec::<InputLeafNode<TestContent>>::new();

        // note we don't use the helper function max_bottom_layer_nodes
        for i in 0..2usize.pow(height.as_u32() - 1) {
            leaf_nodes.push(InputLeafNode::<TestContent> {
                x_coord: i as u64,
                content: TestContent {
                    hash: H256::random(),
                    value: i as u32,
                },
            });
        }

        leaf_nodes
    }

    pub fn single_leaf(x_coord_of_leaf: u64) -> InputLeafNode<TestContent> {
        let leaf = InputLeafNode::<TestContent> {
            x_coord: x_coord_of_leaf,
            content: TestContent {
                hash: H256::random(),
                value: 100000000,
            },
        };
        leaf
    }

    // A selection of leaves dispersed sparsely along the bottom layer.
    pub fn sparse_leaves(height: &Height) -> Vec<InputLeafNode<TestContent>> {
        // Otherwise we will go over the max x-coord value.
        assert!(height.as_raw_int() >= 4u8);

        // Note the nodes are not in order here (wrt x-coord) so this test also
        // somewhat covers the sorting code in the constructor.
        let leaf_0 = InputLeafNode::<TestContent> {
            x_coord: 6,
            content: TestContent {
                hash: H256::random(),
                value: 1,
            },
        };
        let leaf_1 = InputLeafNode::<TestContent> {
            x_coord: 1,
            content: TestContent {
                hash: H256::random(),
                value: 2,
            },
        };
        let leaf_2 = InputLeafNode::<TestContent> {
            x_coord: 0,
            content: TestContent {
                hash: H256::random(),
                value: 3,
            },
        };
        let leaf_3 = InputLeafNode::<TestContent> {
            x_coord: 5,
            content: TestContent {
                hash: H256::random(),
                value: 4,
            },
        };

        vec![leaf_0, leaf_1, leaf_2, leaf_3]
    }
}
