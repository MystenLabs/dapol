//! An implementation of the generic content type required for [crate][binary_tree][`Node<C>`].
//!
//! This implementation contains the values in the [super][hidden_node] implementation
//! (Pedersen commitment & hash) plus the additional private values (blinding factor and plain text
//! liability). The private values are included so that the total blinding factor & liability sum
//! can be accessed after tree construction. This node type should ideally not be used in
//! the serialization process since it will increase the final byte size and expose the secret
//! values.
//!
//! All the logic related to how to construct the content of a node is held in this file.

use crate::binary_tree::{Coordinate, Mergeable};
use crate::secret::Secret;
use crate::entity::EntityId;

use bulletproofs::PedersenGens;
use curve25519_dalek_ng::{ristretto::RistrettoPoint, scalar::Scalar};
use digest::Digest;
use primitive_types::H256;
use std::marker::PhantomData;

use crate::primitives::H256Finalizable;

use super::HiddenNodeContent;

/// Main struct containing:
/// - Raw liability value
/// - Blinding factor
/// - Pedersen commitment
/// - Hash
///
/// The hash function needs to be a generic parameter because when implementing
/// [crate][binary_tree][`Mergeable`] one needs to define the merge function, which is not generic,
/// and the merge function in the case of FullNodeContent needs to use a generic hash function.
/// One way to solve this problem is to have a generic parameter on this struct and a phantom field.
#[derive(Clone, Debug)]
pub struct FullNodeContent<H> {
    pub liability: u64,
    pub blinding_factor: Scalar,
    pub commitment: RistrettoPoint,
    pub hash: H256,
    _phantom_hash_function: PhantomData<H>,
}

impl<H> PartialEq for FullNodeContent<H> {
    fn eq(&self, other: &Self) -> bool {
        self.liability == other.liability
            && self.blinding_factor == other.blinding_factor
            && self.commitment == other.commitment
            && self.hash == other.hash
    }
}

// -------------------------------------------------------------------------------------------------
// Constructors

impl<H: Digest + H256Finalizable> FullNodeContent<H> {
    /// Simple constructor
    pub fn new(
        liability: u64,
        blinding_factor: Scalar,
        commitment: RistrettoPoint,
        hash: H256,
    ) -> Self {
        FullNodeContent {
            liability,
            blinding_factor,
            commitment,
            hash,
            _phantom_hash_function: PhantomData,
        }
    }

    /// Constructor.
    ///
    /// The secret `liability` realistically does not need more space than 64 bits because it is
    /// generally used for monetary value or head count, also the Bulletproofs library requires
    /// the value to be u64.
    /// The `blinding_factor` needs to have a larger sized storage space (256 bits) ensure promised
    /// n-bit security of the commitments; it can be enlarged to 512 bits if need be as this size
    /// is supported by the underlying `Scalar` constructors.
    #[allow(dead_code)]
    pub fn new_leaf(
        liability: u64,
        blinding_factor: Secret,
        entity_id: EntityId,
        entity_salt: Secret,
    ) -> FullNodeContent<H> {
        // Scalar expects bytes to be in little-endian
        let blinding_factor_scalar = Scalar::from_bytes_mod_order(blinding_factor.into());

        // Compute the Pedersen commitment to the liability `P = g_1^liability * g_2^blinding_factor`
        let commitment =
            PedersenGens::default().commit(Scalar::from(liability), blinding_factor_scalar);

        let entity_id_bytes: Vec<u8> = entity_id.into();
        let entity_salt_bytes: [u8; 32] = entity_salt.into();

        // Compute the hash: `H("leaf" | entity_id | entity_salt)`
        let mut hasher = H::new();
        hasher.update("leaf".as_bytes());
        hasher.update(entity_id_bytes);
        hasher.update(entity_salt_bytes);
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
    #[allow(dead_code)]
    pub fn new_pad(blinding_factor: Secret, coord: &Coordinate, salt: Secret) -> FullNodeContent<H> {
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

// -------------------------------------------------------------------------------------------------
// Conversions

impl<H: Digest + H256Finalizable> FullNodeContent<H> {
    pub fn compress(self) -> HiddenNodeContent<H> {
        HiddenNodeContent::new(self.commitment, self.hash)
    }
}

// -------------------------------------------------------------------------------------------------
// Implement Mergeable trait

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
            commitment: parent_commitment,
            hash: parent_hash,
            _phantom_hash_function: PhantomData,
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Unit tests

// TODO should fuzz the values instead of hard-coding
// TODO we need to unit test the new "new" constructor method
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn new_leaf_works() {
        let liability = 11u64;
        let blinding_factor = 7u64.into();
        let entity_id = EntityId::from_str("some entity").unwrap();
        let entity_salt = 13u64.into();

        FullNodeContent::<blake3::Hasher>::new_leaf(liability, blinding_factor, entity_id, entity_salt);
    }

    #[test]
    fn new_pad_works() {
        let blinding_factor = 7u64.into();
        let coord = Coordinate { x: 1u64, y: 2u8 };
        let entity_salt = 13u64.into();

        FullNodeContent::<blake3::Hasher>::new_pad(blinding_factor, &coord, entity_salt);
    }

    #[test]
    fn merge_works() {
        let liability_1 = 11u64;
        let blinding_factor_1 = 7u64.into();
        let entity_id_1 = EntityId::from_str("some entity 1").unwrap();
        let entity_salt_1 = 13u64.into();
        let node_1 = FullNodeContent::<blake3::Hasher>::new_leaf(
            liability_1,
            blinding_factor_1,
            entity_id_1,
            entity_salt_1,
        );

        let liability_2 = 21u64;
        let blinding_factor_2 = 27u64.into();
        let entity_id_2 = EntityId::from_str("some entity 2").unwrap();
        let entity_salt_2 = 23u64.into();
        let node_2 = FullNodeContent::<blake3::Hasher>::new_leaf(
            liability_2,
            blinding_factor_2,
            entity_id_2,
            entity_salt_2,
        );

        FullNodeContent::merge(&node_1, &node_2);
    }
}
