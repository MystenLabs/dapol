//! An implementation of the content generic type required for [crate][binary_tree][`Node<C>`].
//!
//! This implementation contains only the Pedersen commitment and the hash as fields in the struct.

use bulletproofs::PedersenGens;
use curve25519_dalek_ng::{ristretto::RistrettoPoint, scalar::Scalar};
use primitive_types::H256;
use serde::{Deserialize, Serialize};

use crate::binary_tree::{Coordinate, Mergeable};
use crate::entity::EntityId;
use crate::hasher::Hasher;
use crate::secret::Secret;

use super::FullNodeContent;

/// Main struct containing the Pedersen commitment & hash.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HiddenNodeContent {
    pub commitment: RistrettoPoint,
    pub hash: H256,
}

impl PartialEq for HiddenNodeContent {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

// -------------------------------------------------------------------------------------------------
// Constructors

impl HiddenNodeContent {
    /// Simple constructor
    pub fn new(commitment: RistrettoPoint, hash: H256) -> Self {
        HiddenNodeContent { commitment, hash }
    }

    /// Create the content for a leaf node.
    ///
    /// The secret `value` realistically does not need more space than 64 bits because it is
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
    ) -> HiddenNodeContent {
        // Compute the Pedersen commitment to the value `P = g_1^value * g_2^blinding_factor`
        let commitment = PedersenGens::default().commit(
            Scalar::from(liability),
            Scalar::from_bytes_mod_order(blinding_factor.into()),
        );

        let entity_id_bytes: Vec<u8> = entity_id.into();
        let entity_salt_bytes: [u8; 32] = entity_salt.into();

        // Compute the hash: `H("leaf" | entity_id | entity_salt)`
        let mut hasher = Hasher::new();
        hasher.update("leaf".as_bytes());
        hasher.update(&entity_id_bytes);
        hasher.update(&entity_salt_bytes);
        let hash = hasher.finalize();

        HiddenNodeContent { commitment, hash }
    }

    /// Create the content for a new padding node.
    ///
    /// The hash requires the node's coordinate as well as a salt. Since the liability of a
    /// padding node is 0 only the blinding factor is required for the Pedersen commitment.
    #[allow(dead_code)]
    pub fn new_pad(blinding_factor: Secret, coord: &Coordinate, salt: Secret) -> HiddenNodeContent {
        // Compute the Pedersen commitment to 0 `P = g_1^0 * g_2^blinding_factor`
        let commitment = PedersenGens::default().commit(
            Scalar::from(0u64),
            Scalar::from_bytes_mod_order(blinding_factor.into()),
        );

        let salt_bytes: [u8; 32] = salt.into();

        // Compute the hash: `H("pad" | coordinate | salt)`
        let mut hasher = Hasher::new();
        hasher.update("pad".as_bytes());
        hasher.update(&coord.to_bytes());
        hasher.update(&salt_bytes);
        let hash = hasher.finalize();

        HiddenNodeContent { commitment, hash }
    }
}

// -------------------------------------------------------------------------------------------------
// Conversion

impl From<FullNodeContent> for HiddenNodeContent {
    fn from(full_node: FullNodeContent) -> Self {
        full_node.compress()
    }
}

// -------------------------------------------------------------------------------------------------
// Implement merge trait

impl Mergeable for HiddenNodeContent {
    /// Returns the parent node content by merging two child node contents.
    ///
    /// The commitment of the parent is the homomorphic sum of the two children.
    /// The hash of the parent is computed by hashing the concatenated commitments and hashes of two children.
    fn merge(left_sibling: &Self, right_sibling: &Self) -> Self {
        let parent_commitment = left_sibling.commitment + right_sibling.commitment;

        // `hash = H(left.com | right.com | left.hash | right.hash`
        let parent_hash = {
            let mut hasher = Hasher::new();
            hasher.update(left_sibling.commitment.compress().as_bytes());
            hasher.update(right_sibling.commitment.compress().as_bytes());
            hasher.update(left_sibling.hash.as_bytes());
            hasher.update(right_sibling.hash.as_bytes());
            hasher.finalize()
        };

        HiddenNodeContent {
            commitment: parent_commitment,
            hash: parent_hash,
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

        HiddenNodeContent::new_leaf(liability, blinding_factor, entity_id, entity_salt);
    }

    #[test]
    fn new_pad_works() {
        let blinding_factor = 7u64.into();
        let coord = Coordinate { x: 1u64, y: 2u8 };
        let entity_salt = 13u64.into();

        HiddenNodeContent::new_pad(blinding_factor, &coord, entity_salt);
    }

    #[test]
    fn merge_works() {
        let liability_1 = 11u64;
        let blinding_factor_1 = 7u64.into();
        let entity_id_1 = EntityId::from_str("some entity 1").unwrap();
        let entity_salt_1 = 13u64.into();
        let node_1 =
            HiddenNodeContent::new_leaf(liability_1, blinding_factor_1, entity_id_1, entity_salt_1);

        let liability_2 = 21u64;
        let blinding_factor_2 = 27u64.into();
        let entity_id_2 = EntityId::from_str("some entity 2").unwrap();
        let entity_salt_2 = 23u64.into();
        let node_2 =
            HiddenNodeContent::new_leaf(liability_2, blinding_factor_2, entity_id_2, entity_salt_2);

        HiddenNodeContent::merge(&node_1, &node_2);
    }
}
