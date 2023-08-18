//! This is legacy and will be removed

use digest::Digest;
use std::fmt::Debug;
use std::marker::PhantomData;

use smtree::{
    index::TreeIndex,
    traits::{ProofExtractable, Serializable, TypeName},
    utils::generate_sorted_index_value_pairs,
};

use crate::{
    utils::get_secret, Dapol, DapolNode, DapolProof, RangeProofPadding, RangeProofSplitting,
    RangeProvable, RangeVerifiable,
};

pub struct TesterDapol<D, R> {
    _phantom_d: PhantomData<D>,
    _phantom_r: PhantomData<R>,
}

impl<
        D: Digest + Default + Clone + TypeName + Debug,
        R: Clone + Serializable + RangeProvable + RangeVerifiable + TypeName,
    > TesterDapol<D, R>
{
    pub fn test() {
        const LEAF_NUM: usize = 100;
        const TREE_HEIGHT: usize = 10;
        for _iter in 0..5 {
            for aggregation_factor in 1..TREE_HEIGHT + 1 {
                println!("Test #{} for SMT({}) with {} leaves of {} ({}) with aggregation factor {} starts",
                         _iter, TREE_HEIGHT, LEAF_NUM, DapolNode::<D>::get_name(), R::get_name(), aggregation_factor);

                let list: Vec<(TreeIndex, DapolNode<D>)> =
                    generate_sorted_index_value_pairs(TREE_HEIGHT, LEAF_NUM);

                let secret = get_secret();
                let mut build_dapol = Dapol::<D, R>::new_blank(TREE_HEIGHT, aggregation_factor);
                build_dapol.build(&list, &secret);
                build_dapol.generate_all_proofs();

                let mut update_dapol = Dapol::<D, R>::new_blank(TREE_HEIGHT, aggregation_factor);
                for item in list.iter() {
                    update_dapol.update(&item.0, item.1.clone(), &secret);
                }
                update_dapol.generate_all_proofs();

                assert_eq!(build_dapol.root_raw(), update_dapol.root_raw());

                let batch_size = 10usize;
                for i in 0..LEAF_NUM / batch_size {
                    let mut proof_list = Vec::new();
                    let mut leaves = Vec::new();
                    for j in 0..batch_size {
                        let item = &list[i * batch_size + j];
                        proof_list.push(item.0);
                        leaves.push(item.1.get_proof_node());
                    }
                    let proof = build_dapol.generate_proof_batch(&proof_list);
                    match proof {
                        None => unreachable!(),
                        Some(proof) => {
                            let serialized_proof = proof.serialize();
                            let deserialized_proof =
                                DapolProof::<D, R>::deserialize(&serialized_proof).unwrap();
                            assert!(deserialized_proof.verify_batch(&build_dapol.root(), &leaves));
                        }
                    }
                }

                for item in list.iter() {
                    let proof = build_dapol.generate_proof(&item.0);
                    let proof_node = item.1.get_proof_node();
                    match proof {
                        None => unreachable!(),
                        Some(proof) => {
                            let serialized_proof = proof.serialize();
                            let deserialized_proof =
                                DapolProof::<D, R>::deserialize(&serialized_proof).unwrap();
                            assert!(deserialized_proof.verify(&build_dapol.root(), &proof_node));
                        }
                    }
                    let proof = update_dapol.generate_proof(&item.0);
                    match proof {
                        None => unreachable!(),
                        Some(proof) => {
                            let serialized_proof = proof.serialize();
                            let deserialized_proof =
                                DapolProof::<D, R>::deserialize(&serialized_proof).unwrap();
                            assert!(deserialized_proof.verify(&update_dapol.root(), &proof_node));
                        }
                    }
                }

                println!("Succeed!");
            }
        }
    }
}
#[test]
#[ignore] // test takes long
fn test_dapol() {
    TesterDapol::<blake3::Hasher, RangeProofSplitting>::test();
    TesterDapol::<blake3::Hasher, RangeProofPadding>::test();
    TesterDapol::<blake2::Blake2b, RangeProofPadding>::test();
    TesterDapol::<blake2::Blake2b, RangeProofSplitting>::test();
}

#[test]
#[ignore] // test takes long
fn prove_n_verify() {
    let tree_height = 5;
    let aggregation_factor = 1;

    // build a list of random nodes
    let nodes = generate_sorted_index_value_pairs::<DapolNode<blake3::Hasher>>(tree_height, 10);

    // create an instance of DAPOL for the generated nodes
    let secret = get_secret();
    let mut dapol =
        Dapol::<blake3::Hasher, RangeProofSplitting>::new_blank(tree_height, aggregation_factor);
    dapol.build(&nodes, &secret);

    for node in nodes.iter() {
        let proof = dapol.generate_proof(&node.0).unwrap();
        let result = proof.verify(&dapol.root(), &node.1.get_proof_node());
        assert_eq!(true, result);
    }
}
