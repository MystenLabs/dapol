//! Legacy

use crate::{utils::get_secret, Dapol, DapolNode, DapolProof, RangeProofSplitting};
use smtree::{
    index::TreeIndex, traits::ProofExtractable, utils::generate_sorted_index_value_pairs,
};

#[test]
#[ignore] // test takes long
fn test_serialization() {
    let tree_height = 8;
    let num_leaves = 20;
    let batch_size = 10;

    // build random DAPOL tree
    let list: Vec<(TreeIndex, DapolNode<blake3::Hasher>)> =
        generate_sorted_index_value_pairs(tree_height, num_leaves);
    let secret = get_secret();
    let mut dapol = Dapol::<blake3::Hasher, RangeProofSplitting>::new_blank(tree_height, 1);
    dapol.build(&list, &secret);

    // build a batch proof
    let mut batch_indecies = Vec::new();
    let mut batch_leaves = Vec::new();
    for item in list.iter().take(batch_size) {
        batch_indecies.push(item.0);
        batch_leaves.push(item.1.get_proof_node());
    }
    let proof = dapol.generate_proof_batch(&batch_indecies).unwrap();

    // serialize and deserialize the proof
    let serialized_proof = proof.serialize();
    let deserialized_proof =
        DapolProof::<blake3::Hasher, RangeProofSplitting>::deserialize(&serialized_proof).unwrap();
    // TODO: ideally, this should be assert_eq!(proof, deserialized_proof); but DapolProof doesn't
    // implement PartialEq, and there doesn't seem to be an easy way to implement it.
    assert!(deserialized_proof.verify_batch(&dapol.root(), &batch_leaves));
}
