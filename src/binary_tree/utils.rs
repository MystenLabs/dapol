//! Ease of use functions to make cleaner code.

// -------------------------------------------------------------------------------------------------
// Test utils for sub-modules.

#[cfg(any(test, fuzzing))]
pub mod test_utils {
    use super::super::*;
    use primitive_types::H256;
    use crate::hasher::Hasher;

    #[derive(Clone, Debug, PartialEq, Serialize)]
    pub struct TestContent {
        pub value: u32,
        pub hash: H256,
    }

    impl Mergeable for TestContent {
        fn merge(left_sibling: &Self, right_sibling: &Self) -> Self {
            // C(parent) = C(L) + C(R)
            let parent_value = left_sibling.value + right_sibling.value;

            // H(parent) = Hash(C(L) | C(R) | H(L) | H(R))
            let parent_hash = {
                let mut hasher = Hasher::new();
                hasher.update(&left_sibling.value.to_le_bytes());
                hasher.update(&right_sibling.value.to_le_bytes());
                hasher.update(left_sibling.hash.as_bytes());
                hasher.update(right_sibling.hash.as_bytes());
                hasher.finalize()
            };

            TestContent {
                value: parent_value,
                hash: parent_hash,
            }
        }
    }

    pub fn generate_padding_closure() -> impl Fn(&Coordinate) -> TestContent {
        |_coord: &Coordinate| -> TestContent {
            TestContent {
                value: 0,
                hash: H256::default(),
            }
        }
    }

    pub fn random_leaf_nodes(num_leaf_nodes: u64, height: &Height, seed: u64) -> Vec<InputLeafNode<TestContent>> {
        use crate::accumulators::RandomXCoordGenerator;

        let mut leaf_nodes = Vec::<InputLeafNode<TestContent>>::new();
        let mut x_coord_generator = RandomXCoordGenerator::from_seed(height, seed);

        for i in 0..num_leaf_nodes {
            leaf_nodes.push(InputLeafNode::<TestContent> {
                x_coord: x_coord_generator.new_unique_x_coord().unwrap(),
                content: TestContent {
                    hash: H256::random(),
                    value: i as u32,
                },
            });
        }

        leaf_nodes
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
        InputLeafNode::<TestContent> {
            x_coord: x_coord_of_leaf,
            content: TestContent {
                hash: H256::random(),
                value: 100000000,
            },
        }
    }

    // A selection of leaves dispersed sparsely along the bottom layer.
    pub fn sparse_leaves(height: &Height) -> Vec<InputLeafNode<TestContent>> {
        // Otherwise we will go over the max x-coord value.
        assert!(height.as_u8() >= 4u8);

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
