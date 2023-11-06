//! Inclusion proof struct and methods.
//!
//! The inclusion proof is very closely related to the node content type, and so
//! the struct was not made generic in the type of node content. If other node
//! contents are to be supported then new inclusion proof structs and methods
//! will need to be written.

use primitive_types::H256;
use serde::{Deserialize, Serialize};

use std::{fmt::Debug, path::PathBuf};

use log::info;

use crate::binary_tree::{Coordinate, Height, Node, Path};
use crate::node_content::{FullNodeContent, HiddenNodeContent};
use crate::{EntityId, read_write_utils};

mod individual_range_proof;
use individual_range_proof::IndividualRangeProof;

mod aggregated_range_proof;
use aggregated_range_proof::AggregatedRangeProof;

mod aggregation_factor;
pub use aggregation_factor::AggregationFactor;

/// Default upper bound for the range proof in the inclusion proof.
/// 64 bits should be more than enough bits to represent liabilities for real
/// world applications such as crypto asset exchange balances.
pub const DEFAULT_UPPER_BOUND_BIT_LENGTH: u8 = 64u8;

/// The file extension used when writing serialized binary files.
const SERIALIZED_PROOF_EXTENSION: &str = "dapolproof";

/// Inclusion proof struct.
///
/// There are 2 parts to an inclusion proof:
/// - the path in the tree
/// - the range proof for the Pedersen commitments
///
/// The tree path is taken to be of a compressed node content type because
/// sharing a full node content type with entities would leak secret information
/// such as other entity's liabilities and the total sum of liabilities.
///
/// The Bulletproofs protocol allows aggregating multiple range proofs into 1
/// proof, which is more efficient to produce & verify than doing them
/// individually. Both aggregated and individual range proofs are supported.
#[derive(Debug, Serialize, Deserialize)]
pub struct InclusionProof {
    path: Path<HiddenNodeContent>,
    individual_range_proofs: Option<Vec<IndividualRangeProof>>,
    aggregated_range_proof: Option<AggregatedRangeProof>,
    aggregation_factor: AggregationFactor,
    upper_bound_bit_length: u8,
}

impl InclusionProof {
    /// Generate an inclusion proof from a tree path.
    ///
    /// `aggregation_factor` is used to determine how many of the range proofs
    /// are aggregated. Those that do not form part of the aggregated proof
    /// are just proved individually. The aggregation is a feature of the
    /// Bulletproofs protocol that improves efficiency.
    ///
    /// `upper_bound_bit_length` is used to determine the upper bound for the
    /// range proof, which is set to `2^upper_bound_bit_length` i.e. the
    /// range proof shows `0 <= liability <= 2^upper_bound_bit_length` for
    /// some liability. The type is set to `u8` because we are not expected
    /// to require bounds higher than $2^256$. Note that if the value is set
    /// to anything other than 8, 16, 32 or 64 the Bulletproofs code will return
    /// an Err.
    pub fn generate(
        path: Path<FullNodeContent>,
        aggregation_factor: AggregationFactor,
        upper_bound_bit_length: u8,
    ) -> Result<Self, InclusionProofError> {
        // Is this cast safe? Yes because the tree height (which is the same as the
        // length of the input) is also stored as a u8, and so there would never
        // be more siblings than max(u8). TODO might be worth using a bounded
        // vector for siblings. If the tree height changes type for some
        // reason then this code would fail silently.
        let tree_height = Height::from_y_coord(path.siblings.len() as u8);
        let aggregation_index = aggregation_factor.apply_to(&tree_height);

        let mut nodes_for_aggregation = path.nodes_from_bottom_to_top()?;
        let nodes_for_individual_proofs =
            nodes_for_aggregation.split_off(aggregation_index as usize);

        let aggregated_range_proof = match aggregation_factor.is_zero(&tree_height) {
            false => {
                let aggregation_tuples = nodes_for_aggregation
                    .into_iter()
                    .map(|node| (node.content.liability, node.content.blinding_factor))
                    .collect();
                Some(AggregatedRangeProof::generate(
                    &aggregation_tuples,
                    upper_bound_bit_length,
                )?)
            }
            true => None,
        };

        let individual_range_proofs = match aggregation_factor.is_max(&tree_height) {
            false => Some(
                nodes_for_individual_proofs
                    .into_iter()
                    .map(|node| {
                        IndividualRangeProof::generate(
                            node.content.liability,
                            &node.content.blinding_factor,
                            upper_bound_bit_length,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            true => None,
        };

        Ok(InclusionProof {
            path: path.convert(),
            individual_range_proofs,
            aggregated_range_proof,
            aggregation_factor,
            upper_bound_bit_length,
        })
    }

    /// Verify that an inclusion proof matches a given root hash.
    pub fn verify(&self, root_hash: H256) -> Result<(), InclusionProofError> {
        use curve25519_dalek_ng::ristretto::CompressedRistretto;

        info!("Verifying inclusion proof..");

        // Is this cast safe? Yes because the tree height (which is the same as the
        // length of the input) is also stored as a u8, and so there would never
        // be more siblings than max(u8).
        let tree_height = Height::from_y_coord(self.path.siblings.len() as u8);

        {
            // Merkle tree path verification

            use bulletproofs::PedersenGens;
            use curve25519_dalek_ng::scalar::Scalar;

            // PartialEq for HiddenNodeContent does not depend on the commitment so we can
            // make this whatever we like
            let dummy_commitment =
                PedersenGens::default().commit(Scalar::from(0u8), Scalar::from(0u8));
            let root = Node {
                content: HiddenNodeContent::new(dummy_commitment, root_hash),
                coord: Coordinate {
                    x: 0,
                    y: tree_height.as_y_coord(),
                },
            };

            self.path.verify(&root)?;
        }

        {
            // Range proof verification

            let aggregation_index = self.aggregation_factor.apply_to(&tree_height) as usize;

            let mut commitments_for_aggregated_proofs: Vec<CompressedRistretto> = self
                .path
                .nodes_from_bottom_to_top()?
                .iter()
                .map(|node| node.content.commitment.compress())
                .collect();

            let commitments_for_individual_proofs =
                commitments_for_aggregated_proofs.split_off(aggregation_index);

            if let Some(proofs) = &self.individual_range_proofs {
                commitments_for_individual_proofs
                    .iter()
                    .zip(proofs.iter())
                    .map(|(com, proof)| proof.verify(com, self.upper_bound_bit_length))
                    .collect::<Result<Vec<_>, _>>()?;
            }

            if let Some(proof) = &self.aggregated_range_proof {
                proof.verify(
                    &commitments_for_aggregated_proofs,
                    self.upper_bound_bit_length,
                )?;
            }
        }

        info!("Succesfully verified proof");

        Ok(())
    }

    /// Serialize the [InclusionProof] structure to a binary file.
    ///
    /// An error is returned if
    /// 1. [bincode] fails to serialize the file.
    /// 2. There is an issue opening or writing the file.
    // STENT TODO don't need entity_id as param, need to change Path to accept another generic type
    pub fn serialize(&self, entity_id: &EntityId, dir: PathBuf) -> Result<(), InclusionProofError> {
        let mut file_name = entity_id.to_string();
        file_name.push_str(".");
        file_name.push_str(SERIALIZED_PROOF_EXTENSION);

        let path = dir.join(file_name);
        info!("Serializing inclusion proof to path {:?}", path);

        read_write_utils::serialize_to_bin_file(&self, path)?;

        Ok(())
    }

    /// Deserialize the [InclusionProof] structure from a binary file.
    ///
    /// The file is assumed to be in [bincode] format.
    ///
    /// An error is logged and returned if
    /// 1. The file cannot be opened.
    /// 2. The [bincode] deserializer fails.
    pub fn deserialize(file_path: PathBuf) -> Result<InclusionProof, InclusionProofError> {
        info!("Deserializing inclusion proof to path {:?}", file_path);
        let proof: InclusionProof = read_write_utils::deserialize_from_bin_file(file_path)?;
        Ok(proof)
    }
}

// -------------------------------------------------------------------------------------------------
// Errors

#[derive(thiserror::Error, Debug)]
pub enum InclusionProofError {
    #[error("Siblings path verification failed")]
    TreePathError(#[from] crate::binary_tree::PathError),
    #[error("Issues with range proof")]
    RangeProofError(#[from] RangeProofError),
    #[error("Error serializing/deserializing file")]
    SerdeError(#[from] crate::read_write_utils::ReadWriteError),
}

#[derive(thiserror::Error, Debug)]
pub enum RangeProofError {
    #[error("Bulletproofs generation failed")]
    BulletproofGenerationError(bulletproofs::ProofError),
    #[error("Bulletproofs verification failed")]
    BulletproofVerificationError(bulletproofs::ProofError),
    #[error("The length of the Pedersen commitments vector did not match the length of the input used to generate the proof")]
    InputVectorLengthMismatch,
}

// -------------------------------------------------------------------------------------------------
// Unit tests

// TODO should we mock out the inclusion proof layer for these tests?

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_tree::Coordinate;
    use crate::utils::H256Finalizable;

    use bulletproofs::PedersenGens;
    use curve25519_dalek_ng::{ristretto::RistrettoPoint, scalar::Scalar};
    use primitive_types::H256;

    // STENT TODO grab this from a central place
    type Hash = blake3::Hasher;

    // tree that is built, with path highlighted
    ///////////////////////////////////////////////////////
    //    |                   [root]                     //
    //  3 |                     224                      //
    //    |                    //\                       //
    //    |                   //  \                      //
    //    |                  //    \                     //
    //    |                 //      \                    //
    //    |                //        \                   //
    //    |               //          \                  //
    //    |              //            \                 //
    //    |             //              \                //
    //    |            //                \               //
    //    |           //                  \              //
    //    |          //                    \             //
    //  2 |         80                      144          //
    //    |         /\\                     /\           //
    //    |        /  \\                   /  \          //
    //    |       /    \\                 /    \         //
    //    |      /      \\               /      \        //
    //    |     /        \\             /        \       //
    //  1 |   30          50          84          60     //
    //    |   /\         //\          /\          /\     //
    //    |  /  \       //  \        /  \        /  \    //
    //  0 |13    17    27    23    41    43    07    53  //
    //  _            [leaf]                              //
    //  y  --------------------------------------------  //
    //  x| 0     1     2     3     4     5     6     7   //
    //                                                   //
    ///////////////////////////////////////////////////////
    fn build_test_path() -> (Path<FullNodeContent>, RistrettoPoint, H256) {
        // leaf at (2,0)
        let liability = 27u64;
        let blinding_factor = Scalar::from_bytes_mod_order(*b"11112222333344445555666677778888");
        let commitment = PedersenGens::default().commit(Scalar::from(liability), blinding_factor);
        let mut hasher = Hash::new();
        hasher.update("leaf".as_bytes());
        let hash = hasher.finalize_as_h256();
        let leaf = Node {
            coord: Coordinate { x: 2u64, y: 0u8 },
            content: FullNodeContent::new(liability, blinding_factor, commitment, hash),
        };

        // sibling at (3,0)
        let liability = 23u64;
        let blinding_factor = Scalar::from_bytes_mod_order(*b"22223333444455556666777788881111");
        let commitment = PedersenGens::default().commit(Scalar::from(liability), blinding_factor);
        let mut hasher = Hash::new();
        hasher.update("sibling1".as_bytes());
        let hash = hasher.finalize_as_h256();
        let sibling1 = Node {
            coord: Coordinate { x: 3u64, y: 0u8 },
            content: FullNodeContent::new(liability, blinding_factor, commitment, hash),
        };

        // we need to construct the root hash & commitment for verification testing
        let (parent_hash, parent_commitment) = build_parent(
            leaf.content.commitment,
            sibling1.content.commitment,
            leaf.content.hash,
            sibling1.content.hash,
        );

        // sibling at (0,1)
        let liability = 30u64;
        let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
        let commitment = PedersenGens::default().commit(Scalar::from(liability), blinding_factor);
        let mut hasher = Hash::new();
        hasher.update("sibling2".as_bytes());
        let hash = hasher.finalize_as_h256();
        let sibling2 = Node {
            coord: Coordinate { x: 0u64, y: 1u8 },
            content: FullNodeContent::new(liability, blinding_factor, commitment, hash),
        };

        // we need to construct the root hash & commitment for verification testing
        let (parent_hash, parent_commitment) = build_parent(
            sibling2.content.commitment,
            parent_commitment,
            sibling2.content.hash,
            parent_hash,
        );

        // sibling at (1,2)
        let liability = 144u64;
        let blinding_factor = Scalar::from_bytes_mod_order(*b"44445555666677778888111122223333");
        let commitment = PedersenGens::default().commit(Scalar::from(liability), blinding_factor);
        let mut hasher = Hash::new();
        hasher.update("sibling3".as_bytes());
        let hash = hasher.finalize_as_h256();
        let sibling3 = Node {
            coord: Coordinate { x: 1u64, y: 2u8 },
            content: FullNodeContent::new(liability, blinding_factor, commitment, hash),
        };

        // we need to construct the root hash & commitment for verification testing
        let (root_hash, root_commitment) = build_parent(
            parent_commitment,
            sibling3.content.commitment,
            parent_hash,
            sibling3.content.hash,
        );

        (
            Path {
                siblings: vec![sibling1, sibling2, sibling3],
                leaf,
            },
            root_commitment,
            root_hash,
        )
    }

    fn build_parent(
        left_commitment: RistrettoPoint,
        right_commitment: RistrettoPoint,
        left_hash: H256,
        right_hash: H256,
    ) -> (H256, RistrettoPoint) {
        let parent_commitment = left_commitment + right_commitment;

        // `H(parent) = Hash(C(L) | C(R) | H(L) | H(R))`
        let parent_hash = {
            let mut hasher = Hash::new();
            hasher.update(left_commitment.compress().as_bytes());
            hasher.update(right_commitment.compress().as_bytes());
            hasher.update(left_hash.as_bytes());
            hasher.update(right_hash.as_bytes());
            hasher.finalize_as_h256()
        };

        (parent_hash, parent_commitment)
    }

    // TODO fuzz on the aggregation factor
    #[test]
    fn generate_works() {
        let aggregation_factor = AggregationFactor::Divisor(2u8);
        let upper_bound_bit_length = 64u8;

        let (path, _, _) = build_test_path();
        InclusionProof::generate(path, aggregation_factor, upper_bound_bit_length).unwrap();
    }

    #[test]
    fn verify_works() {
        let aggregation_factor = AggregationFactor::Divisor(2u8);
        let upper_bound_bit_length = 64u8;

        let (path, _root_commitment, root_hash) = build_test_path();

        let proof =
            InclusionProof::generate(path, aggregation_factor, upper_bound_bit_length).unwrap();
        proof.verify(root_hash).unwrap();
    }

    // Ensures Blake 3 library produces correct hashed output.
    // Comparison hash derived through the following urls:
    // https://toolkitbay.com/tkb/tool/BLAKE3
    // https://connor4312.github.io/blake3/index.html
    // https://asecuritysite.com/hash/blake3
    #[test]
    fn verify_hasher() {
        use std::str::FromStr;

        let mut hasher = Hash::new();
        hasher.update("dapol-PoR".as_bytes());
        let hash = hasher.finalize_as_h256();
        assert_eq!(
            hash,
            H256::from_str("e4bf4e238e74eb8d253191a56b594565514201a71373c86e304628ed623c4850")
                .unwrap()
        );
    }

    // TODO test correct error translation from lower layers (probably should
    // mock the error responses rather than triggering them from the code in the
    // lower layers)
}
