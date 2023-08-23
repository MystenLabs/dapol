//! An implementation of the generic content type required for [crate][binary_tree][`Node<C>`].
//!
//! This implementation contains the values in the [super][compressed_node] implementation
//! (Pedersen commitment & hash) plus the additional private values (blinding factor and plain text
//! liability). The private values are included so that the total blinding factor & liability sum
//! can be accessed after tree construction. This node type should ideally not be used in
//! the serialization process since it will increase the final byte size and expose the secret
//! values.
//!
//! All the logic related to how to construct the content of a node is held in this file.

use crate::binary_tree::{Coordinate, Mergeable};
use crate::primitives::D256;
use crate::user::UserId;

use bulletproofs::PedersenGens;
use curve25519_dalek_ng::{ristretto::RistrettoPoint, scalar::Scalar};
use digest::Digest;
use primitive_types::H256;
use std::marker::PhantomData;

use crate::primitives::H256Finalizable;

/// Main struct containing:
/// - Raw liability value
/// - Blinding factor
/// - Pedersen commitment
/// - Hash
///
/// The hash function needs to be a generic parameter because when implementing
/// [crate][binary_tree][`Mergeable`] one needs to define the merge function, is not generic
/// and the merge function in this case needs to use a generic hash function. One way to
/// solve this is to have a generic parameter on this struct and a phantom field.
#[derive(Default, Clone, Debug)]
pub struct FullNodeContent<H> {
    liability: u64,
    blinding_factor: Scalar,
    commitment: RistrettoPoint,
    hash: H256,
    _phantom_hash_function: PhantomData<H>,
}

impl<H: Digest + H256Finalizable> FullNodeContent<H> {
    /// Constructor.
    ///
    /// The secret `liability` realistically does not need more space than 64 bits because it is
    /// generally used for monetary value or head count, also the Bulletproofs library requires
    /// the value to be u64.
    /// The `blinding_factor` needs to have a larger sized storage space (256 bits) ensure promised
    /// n-bit security of the commitments; it can be enlarged to 512 bits if need be as this size
    /// is supported by the underlying `Scalar` constructors.
    pub fn new_leaf(
        liability: u64,
        blinding_factor: D256,
        user_id: UserId,
        user_salt: D256,
    ) -> FullNodeContent<H> {
        // Scalar expects bytes to be in little-endian
        let blinding_factor_scalar = Scalar::from_bytes_mod_order(blinding_factor.into());

        // Compute the Pedersen commitment to the liability `P = g_1^liability * g_2^blinding_factor`
        let commitment =
            PedersenGens::default().commit(Scalar::from(liability), blinding_factor_scalar);

        let user_id_bytes: [u8; 32] = user_id.into();
        let user_salt_bytes: [u8; 32] = user_salt.into();

        // Compute the hash: `H("leaf" | user_id | user_salt)`
        let mut hasher = H::new();
        hasher.update("leaf".as_bytes());
        hasher.update(user_id_bytes);
        hasher.update(user_salt_bytes);
        let hash = hasher.finalize_as_h256();

        FullNodeContent {
            liability,
            blinding_factor: blinding_factor_scalar,
            commitment,
            hash,
            _phantom_hash_function: PhantomData,
        }
    }

    /// Create the content for a new padding node.
    ///
    /// The hash requires the node's coordinate as well as a salt. Since the liability of a
    /// padding node is 0 only the blinding factor is required for the Pedersen commitment.
    pub fn new_pad(blinding_factor: D256, coord: &Coordinate, salt: D256) -> FullNodeContent<H> {
        let liability = 0u64;
        let blinding_factor_scalar = Scalar::from_bytes_mod_order(blinding_factor.into());

        // Compute the Pedersen commitment to the liability `P = g_1^liability * g_2^blinding_factor`
        let commitment =
            PedersenGens::default().commit(Scalar::from(liability), blinding_factor_scalar);

        let coord_bytes = coord.as_bytes();
        let salt_bytes: [u8; 32] = salt.into();

        // Compute the hash: `H("pad" | coordinate | salt)`
        let mut hasher = H::new();
        hasher.update("pad".as_bytes());
        hasher.update(coord_bytes);
        hasher.update(salt_bytes);
        let hash = hasher.finalize_as_h256();

        FullNodeContent {
            liability,
            blinding_factor: blinding_factor_scalar,
            commitment,
            hash,
            _phantom_hash_function: PhantomData,
        }
    }
}

impl<H: Digest + H256Finalizable> Mergeable for FullNodeContent<H> {
    /// Returns the parent node content by merging two child node contents.
    ///
    /// The value and blinding factor of the parent are the sums of the two children respectively.
    /// The commitment of the parent is the homomorphic sum of the two children.
    /// The hash of the parent is computed by hashing the concatenated commitments and hashes of two children.
    fn merge(left_sibling: &Self, right_sibling: &Self) -> Self {
        let parent_liability = left_sibling.liability + right_sibling.liability;
        let parent_blinding_factor = left_sibling.blinding_factor + right_sibling.blinding_factor;
        let parent_commitment = left_sibling.commitment + right_sibling.commitment;

        // `H(parent) = Hash(C(L) | C(R) | H(L) | H(R))`
        let parent_hash = {
            let mut hasher = H::new();
            hasher.update(left_sibling.commitment.compress().as_bytes());
            hasher.update(right_sibling.commitment.compress().as_bytes());
            hasher.update(left_sibling.hash.as_bytes());
            hasher.update(right_sibling.hash.as_bytes());
            hasher.finalize_as_h256() // TODO do a unit test that compares the output of this to a different piece of code
        };

        FullNodeContent {
            liability: parent_liability,
            blinding_factor: parent_blinding_factor,
            commitment: left_sibling.commitment + right_sibling.commitment,
            hash: parent_hash,
            _phantom_hash_function: PhantomData,
        }
    }
}

// TODO should fuzz the values instead of hard-coding
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn new_leaf_works() {
        let liability = 11u64;
        let blinding_factor = 7u64.into();
        let user_id = UserId::from_str("some user").unwrap();
        let user_salt = 13u64.into();

        FullNodeContent::<blake3::Hasher>::new_leaf(liability, blinding_factor, user_id, user_salt);
    }

    #[test]
    fn new_pad_works() {
        let blinding_factor = 7u64.into();
        let coord = Coordinate::new(1u64, 2u8);
        let user_salt = 13u64.into();

        FullNodeContent::<blake3::Hasher>::new_pad(blinding_factor, &coord, user_salt);
    }

    #[test]
    fn merge_works() {
        let liability_1 = 11u64;
        let blinding_factor_1 = 7u64.into();
        let user_id_1 = UserId::from_str("some user 1").unwrap();
        let user_salt_1 = 13u64.into();
        let node_1 = FullNodeContent::<blake3::Hasher>::new_leaf(
            liability_1,
            blinding_factor_1,
            user_id_1,
            user_salt_1,
        );

        let liability_2 = 21u64;
        let blinding_factor_2 = 27u64.into();
        let user_id_2 = UserId::from_str("some user 2").unwrap();
        let user_salt_2 = 23u64.into();
        let node_2 = FullNodeContent::<blake3::Hasher>::new_leaf(
            liability_2,
            blinding_factor_2,
            user_id_2,
            user_salt_2,
        );

        FullNodeContent::merge(&node_1, &node_2);
    }
}

// =================================================
// can probably remove all this

// use smtree::{
//     index::TreeIndex,
//     pad_secret::Secret,
//     traits::{Paddable, ProofExtractable, Rand, TypeName},
// };
// use rand::{thread_rng, Rng};

// STENT TODO this is not needed anymore, the padding function definition should live somewhere else
// impl<H: Digest> Paddable for FullNodeContent<H> {
//     /// Returns a padding node with value 0 and a random blinding factor.
//     /// TODO: check with Kostas if this padding is ok.
//     fn padding(_idx: &TreeIndex, _secret: &Secret) -> FullNodeContent<H> {
//         FullNodeContent::<H>::new(0, Scalar::random(&mut thread_rng()))
//     }
// }

// STENT TODO this conversion does need to happen but not sure how I want to do it yet
//   most likely the tree code will have a conversion function that takes a generic C' type
//   that implements the convert_node trait or something, then can define convert_node here
// impl<H> ProofExtractable for FullNodeContent<H> {
//     type ProofNode = DapolProofNode<H>;
//     fn get_proof_node(&self) -> Self::ProofNode {
//         DapolProofNode::new(self.commitment, self.hash.clone())
//     }
// }

// STENT TODO not sure we need this anymore, seems to only be used for testing
// impl<H: Digest> Rand for FullNodeContent<H> {
//     /// Randomly generates a DAPOL node with random value and random blinding factor.
//     fn randomize(&mut self) {
//         // The value shouldn't be generated as u64 to prevent overflow of sums.
//         let tmp: u32 = thread_rng().gen();
//         *self = FullNodeContent::<H>::new(tmp as u64, Scalar::random(&mut thread_rng()));
//     }
// }

// // STENT TODO why do we need this?
// impl<H: TypeName> TypeName for FullNodeContent<H> {
//     /// Returns the type name of DAPOL nodes with corresponding hash function (for logging purpose).
//     fn get_name() -> String {
//         format!("DAPOL Node ({})", D::get_name())
//     }
// }

// // STENT TODO why partial eq defined like this? what is partial eq actually supposed to do?
// impl<H> PartialEq for FullNodeContent<H> {
//     /// Two DAPOL nodes are considered equal iff the values are equal.
//     fn eq(&self, other: &Self) -> bool {
//         self.liability == other.liability
//     }
// }
