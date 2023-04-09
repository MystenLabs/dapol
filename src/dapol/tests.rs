use super::{Dapol, DapolOptions, Liability, LiabilityId};
use crate::{utils::get_secret, RangeProofPadding};

use smtree::{index::TreeIndex, traits::Serializable};

// TREE CONSTRUCTION
// ================================================================================================

#[test]
fn build_leaf_nodes() {
    let liabilities = build_test_liabilities();
    let (nodes, _) =
        super::build_leaf_nodes::<blake2::Blake2s>(liabilities, "test".as_bytes(), 4).unwrap();
    assert_eq!(4, nodes.len());
}

#[test]
fn build_dapol_tree() {
    let options = build_test_options(4, 2);
    let liabilities = build_test_liabilities();
    let tree = Dapol::<blake2::Blake2s, RangeProofPadding>::new(liabilities, options).unwrap();

    let root_node = tree.root_raw();
    assert_eq!(root_node.get_value(), 26);
}

// PROOF GENERATION
// ================================================================================================

#[test]
fn generate_proof_for_id() {
    // build a test tree
    let tree_height = 4;
    let options = build_test_options(tree_height, 2);
    let liabilities = build_test_liabilities();
    let tree = Dapol::<blake2::Blake2s, RangeProofPadding>::new(liabilities, options).unwrap();

    // ID "a" should map to index 7
    let proof_a = tree
        .generate_proof_for_id(&LiabilityId::from_str("a"))
        .unwrap();
    let proof_7 = tree
        .generate_proof(&TreeIndex::from_u64(tree_height, 7))
        .unwrap();
    assert_eq!(
        proof_7.get_merkle_path().serialize(),
        proof_a.get_merkle_path().serialize()
    );

    // ID "b" should map to index 12
    let proof_b = tree
        .generate_proof_for_id(&LiabilityId::from_str("b"))
        .unwrap();
    let proof_12 = tree
        .generate_proof(&TreeIndex::from_u64(tree_height, 12))
        .unwrap();
    assert_eq!(
        proof_12.get_merkle_path().serialize(),
        proof_b.get_merkle_path().serialize()
    );

    // ID "c" should map to index 2
    let proof_c = tree
        .generate_proof_for_id(&LiabilityId::from_str("c"))
        .unwrap();
    let proof_2 = tree
        .generate_proof(&TreeIndex::from_u64(tree_height, 2))
        .unwrap();
    assert_eq!(
        proof_2.get_merkle_path().serialize(),
        proof_c.get_merkle_path().serialize()
    );

    // ID "d" should map to index 4
    let proof_d = tree
        .generate_proof_for_id(&LiabilityId::from_str("d"))
        .unwrap();
    let proof_4 = tree
        .generate_proof(&TreeIndex::from_u64(tree_height, 4))
        .unwrap();
    assert_eq!(
        proof_4.get_merkle_path().serialize(),
        proof_d.get_merkle_path().serialize()
    );
}

#[test]
fn generate_proof_batch_for_ids() {
    // build a test tree
    let tree_height = 4;
    let options = build_test_options(tree_height, 2);
    let liabilities = build_test_liabilities();
    let tree = Dapol::<blake2::Blake2s, RangeProofPadding>::new(liabilities, options).unwrap();

    // IDs ["a", "b"] should map to indexes [7, 12]
    let ids = [LiabilityId::from_str("a"), LiabilityId::from_str("b")];
    let idx = [
        TreeIndex::from_u64(tree_height, 7),
        TreeIndex::from_u64(tree_height, 12),
    ];

    let actual = tree.generate_proof_batch_for_ids(&ids).unwrap();
    let expected = tree.generate_proof_batch(&idx).unwrap();
    assert_eq!(
        expected.get_merkle_path().serialize(),
        actual.get_merkle_path().serialize()
    );
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_test_options(tree_height: usize, aggregation_factor: usize) -> DapolOptions {
    DapolOptions {
        audit_seed: "test".as_bytes().to_vec(),
        tree_height,
        aggregation_factor,
        secret: get_secret(),
    }
}

fn build_test_liabilities() -> Vec<Liability> {
    let internal_ids = vec!["a", "b", "c", "d"];
    let external_ids = vec!["w", "x", "y", "z"];
    let values = vec![3u64, 5, 7, 11];

    internal_ids
        .into_iter()
        .zip(external_ids)
        .zip(values)
        .map(|((i_id, e_id), value)| Liability {
            internal_id: LiabilityId::from_str(i_id),
            external_id: LiabilityId::from_str(e_id),
            value,
        })
        .collect()
}
