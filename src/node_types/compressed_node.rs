//! This file is not legit yet

use curve25519_dalek_ng::ristretto::RistrettoPoint;
use digest::Digest;
use std::marker::PhantomData;
use crate::binary_tree::Mergeable;

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
    fn merge(left_sibling: &Self, right_sibling: &Self) -> Self {
        // C(parent) = C(L) + C(R)
        let parent_commitment = left_sibling.commitment + right_sibling.commitment;

        // H(parent) = Hash(C(L) | C(R) | H(L) | H(R))
        let parent_hash = {
            let mut hasher = H::new();
            hasher.update(left_sibling.commitment.compress().as_bytes());
            hasher.update(right_sibling.commitment.compress().as_bytes());
            hasher.update(left_sibling.hash.as_bytes());
            hasher.update(right_sibling.hash.as_bytes());
            hasher.finalize_as_h256() // STENT TODO double check the output of this thing
        };

        DapolNodeContent {
            commitment: parent_commitment,
            hash: parent_hash,
            _phantom_hash_function: PhantomData,
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct H256([u8; 32]);

impl H256 {
    fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

// #[cfg(test)]
// mod tests {
//     use bulletproofs::PedersenGens;
//     use curve25519_dalek_ng::scalar::Scalar;

//     use super::*;

//     #[test]
//     pub fn stent_tree_test() {
//         let height = 4;
//         let v_blinding = Scalar::from(8_u32);

//         let new_padding_node_content = |coord: &Coordinate| -> DapolNodeContent<blake3::Hasher> {
//             DapolNodeContent {
//                 commitment: PedersenGens::default()
//                     .commit(Scalar::from(3_u32), Scalar::from(0_u32)),
//                 hash: H256::default(),
//                 _phantom_hash_function: PhantomData,
//             }
//         };

//         let leaf_1 = InputLeafNode::<DapolNodeContent<blake3::Hasher>> {
//             x_coord: 0,
//             content: DapolNodeContent {
//                 hash: H256::default(),
//                 commitment: PedersenGens::default().commit(Scalar::from(0_u32), v_blinding),
//                 _phantom_hash_function: PhantomData,
//             },
//         };
//         let leaf_2 = InputLeafNode::<DapolNodeContent<blake3::Hasher>> {
//             x_coord: 4,
//             content: DapolNodeContent {
//                 hash: H256::default(),
//                 commitment: PedersenGens::default().commit(Scalar::from(2_u32), v_blinding),
//                 _phantom_hash_function: PhantomData,
//             },
//         };
//         let leaf_3 = InputLeafNode::<DapolNodeContent<blake3::Hasher>> {
//             x_coord: 7,
//             content: DapolNodeContent {
//                 hash: H256::default(),
//                 commitment: PedersenGens::default().commit(Scalar::from(3_u32), v_blinding),
//                 _phantom_hash_function: PhantomData,
//             },
//         };
//         let input = vec![leaf_1, leaf_2, leaf_3];
//         let tree = SparseSummationMerkleTree::new(input, height, &new_padding_node_content);
//         for item in &tree.store {
//             println!("coord {:?} hash {:?}", item.1.coord, item.1.content.hash);
//         }

//         println!("\n");

//         let proof = tree.create_inclusion_proof(0);
//         for item in &proof.siblings {
//             println!(
//                 "coord {:?} value {:?} hash {:?}",
//                 item.coord, item.content.commitment, item.content.hash
//             );
//         }

//         println!("\n");
//         proof.verify();
//     }
// }
