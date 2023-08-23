//! An implementation of the content generic type required for [crate][binary_tree][`Node<C>`].
//!
//! This implementation contains only the Pedersen commitment and the hash as fields in the struct.

use bulletproofs::PedersenGens;
use curve25519_dalek_ng::{ristretto::RistrettoPoint, scalar::Scalar};
use digest::Digest;
use primitive_types::H256;
use std::marker::PhantomData;

use crate::binary_tree::{Coordinate, Mergeable};
use crate::primitives::D256;
use crate::user::UserId;
use crate::primitives::H256Finalizable;

/// Main struct containing the Pedersen commitment & hash.
///
/// The hash function needs to be a generic parameter because when implementing
/// [crate][binary_tree][`Mergeable`] one needs to define the merge function, is not generic
/// and the merge function in this case needs to use a generic hash function. One way to
/// solve this is to have a generic parameter on this struct and a phantom field.
#[derive(Default, Clone, Debug)]
pub struct CompressedNodeContent<H> {
    commitment: RistrettoPoint,
    hash: H256,
    _phantom_hash_function: PhantomData<H>,
}

impl<H: Digest + H256Finalizable> CompressedNodeContent<H> {
    /// Constructor.
    ///
    /// The secret `value` realistically does not need more space than 64 bits because it is
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
    ) -> CompressedNodeContent<H> {
        use bulletproofs::PedersenGens;

        // Compute the Pedersen commitment to the value `P = g_1^value * g_2^blinding_factor`
        let commitment = PedersenGens::default().commit(
            Scalar::from(liability),
            Scalar::from_bytes_mod_order(blinding_factor.into()),
        );

        let user_id_bytes: [u8; 32] = user_id.into();
        let user_salt_bytes: [u8; 32] = user_salt.into();

        // Compute the hash: `H("leaf" | user_id | user_salt)`
        let mut hasher = H::new();
        hasher.update("leaf".as_bytes());
        hasher.update(user_id_bytes);
        hasher.update(user_salt_bytes);
        let hash = hasher.finalize_as_h256();

        CompressedNodeContent {
            commitment,
            hash,
            _phantom_hash_function: PhantomData,
        }
    }

    /// Create the content for a new padding node.
    ///
    /// The hash requires the node's coordinate as well as a salt. Since the liability of a
    /// padding node is 0 only the blinding factor is required for the Pedersen commitment.
    pub fn new_pad(
        blinding_factor: D256,
        coord: &Coordinate,
        salt: D256,
    ) -> CompressedNodeContent<H> {
        // Compute the Pedersen commitment to 0 `P = g_1^0 * g_2^blinding_factor`
        let commitment = PedersenGens::default().commit(
            Scalar::from(0u64),
            Scalar::from_bytes_mod_order(blinding_factor.into()),
        );

        let salt_bytes: [u8; 32] = salt.into();

        // Compute the hash: `H("pad" | coordinate | salt)`
        let mut hasher = H::new();
        hasher.update("pad".as_bytes());
        hasher.update(coord.as_bytes());
        hasher.update(salt_bytes);
        let hash = hasher.finalize_as_h256();

        CompressedNodeContent {
            commitment,
            hash,
            _phantom_hash_function: PhantomData,
        }
    }
}

impl<H: Digest + H256Finalizable> Mergeable for CompressedNodeContent<H> {
    /// Returns the parent node content by merging two child node contents.
    ///
    /// The commitment of the parent is the homomorphic sum of the two children.
    /// The hash of the parent is computed by hashing the concatenated commitments and hashes of two children.
    fn merge(left_sibling: &Self, right_sibling: &Self) -> Self {
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

        CompressedNodeContent {
            commitment: parent_commitment,
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
    fn constructor_works() {
        let liability = 11u64;
        let blinding_factor = 7u64.into();
        let user_id = UserId::from_str("some user").unwrap();
        let user_salt = 13u64.into();

        CompressedNodeContent::<blake3::Hasher>::new_leaf(
            liability,
            blinding_factor,
            user_id,
            user_salt,
        );
    }

    #[test]
    fn new_pad_works() {
        let blinding_factor = 7u64.into();
        let coord = Coordinate::new(1u64, 2u8);
        let user_salt = 13u64.into();

        CompressedNodeContent::<blake3::Hasher>::new_pad(blinding_factor, &coord, user_salt);
    }

    #[test]
    fn merge_works() {
        let liability_1 = 11u64;
        let blinding_factor_1 = 7u64.into();
        let user_id_1 = UserId::from_str("some user 1").unwrap();
        let user_salt_1 = 13u64.into();
        let node_1 = CompressedNodeContent::<blake3::Hasher>::new_leaf(
            liability_1,
            blinding_factor_1,
            user_id_1,
            user_salt_1,
        );

        let liability_2 = 21u64;
        let blinding_factor_2 = 27u64.into();
        let user_id_2 = UserId::from_str("some user 2").unwrap();
        let user_salt_2 = 23u64.into();
        let node_2 = CompressedNodeContent::<blake3::Hasher>::new_leaf(
            liability_2,
            blinding_factor_2,
            user_id_2,
            user_salt_2,
        );

        CompressedNodeContent::merge(&node_1, &node_2);
    }
}
