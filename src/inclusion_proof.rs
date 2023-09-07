//! Inclusion proof struct and methods.
//!
//! The inclusion proof is very closely related to the node content type, and so the struct was not
//! made generic in the type of node content. If other node contents are to be supported then new
//! inclusion proof structs and methods will need to be written.

use crate::binary_tree::{Node, Path, PathError};
use crate::node_content::{CompressedNodeContent, FullNodeContent};
use crate::primitives::H256Finalizable;

use ::std::fmt::Debug;
use bulletproofs::ProofError;
use digest::Digest;
use percentage::PercentageInteger;
use thiserror::Error;

mod individual_range_proof;
use individual_range_proof::IndividualRangeProof;

mod aggregated_range_proof;
use aggregated_range_proof::AggregatedRangeProof;

/// Inclusion proof struct.
///
/// There are 2 parts to an inclusion proof:
/// - the path in the tree
/// - the range proof for the Pedersen commitments
/// The tree path is taken to be of a compressed node content type because sharing a full node
/// content type with users would leak secret information such as other user's liabilities and the
/// total sum of liabilities.
#[derive(Debug)]
pub struct InclusionProof<H: Clone> {
    path: Path<CompressedNodeContent<H>>,
    individual_range_proofs: Vec<IndividualRangeProof>,
    aggregated_range_proof: AggregatedRangeProof,
}

impl<H: Clone + Debug + Digest + H256Finalizable> InclusionProof<H> {
    /// Generate an inclusion proof from a tree path.
    ///
    /// The aggregation factor is used to determine how many of the range proofs are aggregated.
    /// Those that do not form part of the aggregated proof are just proved individually. The
    /// aggregation is a feature of the Bulletproofs protocol that improves efficiency.
    pub fn generate(
        path: Path<FullNodeContent<H>>,
        aggregation_factor: AggregationFactor,
    ) -> Result<Self, InclusionProofError> {
        // Is this cast safe? Yes because the tree height (which is the same as the length of the
        // input) is also stored as a u8, and so there would never be more siblings than max(u8).
        // TODO might be worth using a bounded vector for siblings. If the tree height changes
        //   type for some reason then this code would fail silently.
        let tree_height = path.siblings.len() as u8 + 1;
        let aggregation_index = aggregation_factor.apply_to(tree_height);

        let mut nodes_for_aggregation = path.nodes_from_bottom_to_top()?;
        let mut nodes_for_individual_proofs =
            nodes_for_aggregation.split_off(aggregation_index as usize);

        let upper_bound_bit_length = 64; // TODO parameter

        let aggregation_tuples = nodes_for_aggregation
            .into_iter()
            .map(|node| (node.content.liability, node.content.blinding_factor))
            .collect();
        let aggregated_range_proof =
            AggregatedRangeProof::generate(&aggregation_tuples, upper_bound_bit_length)?;

        let individual_range_proofs = nodes_for_individual_proofs
            .into_iter()
            .map(|node| {
                IndividualRangeProof::generate(
                    node.content.liability,
                    &node.content.blinding_factor,
                    upper_bound_bit_length,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(InclusionProof {
            path: path.convert(),
            individual_range_proofs,
            aggregated_range_proof,
        })
    }

    /// Verify that an inclusion proof matches a given root hash.
    // TODO have the upper bound as an input here to make it clear what is needed to verify
    // TODO only require the hash, not the whole node
    pub fn verify(&self, root: &Node<CompressedNodeContent<H>>) -> Result<(), InclusionProofError> {
        use curve25519_dalek_ng::ristretto::CompressedRistretto;

        self.path.verify(root)?;

        let upper_bound_bit_length = 64; // TODO parameter
        let tree_height = self.path.siblings.len() + 1;
        let aggregation_index = tree_height - self.individual_range_proofs.len();

        let mut commitments_for_aggregated_proofs: Vec<CompressedRistretto> = self
            .path
            .nodes_from_bottom_to_top()?
            .iter()
            .map(|node| node.content.commitment.compress())
            .collect();

        let commitments_for_individual_proofs =
            commitments_for_aggregated_proofs.split_off(aggregation_index);

        commitments_for_individual_proofs
            .iter()
            .zip(self.individual_range_proofs.iter())
            .map(|(com, proof)| proof.verify(&com, upper_bound_bit_length))
            .collect::<Result<Vec<_>, _>>()?;

        // STENT TODO do not perform the verification if there were no aggregated proof, same for individual
        self.aggregated_range_proof
            .verify(&commitments_for_aggregated_proofs, upper_bound_bit_length);

        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------
// Aggregation factor

/// Method used to determine how many of the range proofs are aggregated. Those that do not
/// form part of the aggregated proof are just proved individually.
///
/// Divisor: divide the number of nodes by this number to get the ratio of the nodes to be used in
/// the aggregated proof i.e. `number_of_ranges_for_aggregation = tree_height / divisor` (any
/// decimals are truncated, not rounded). Note:
/// - if this number is 0 it means that none of the proofs should be aggregated
/// - if this number is 1 it means that all of the proofs should be aggregated
/// - if this number is `tree_height` it means that only the leaf node should be aggregated
/// - if this number is `> tree_height` it means that none of the proofs should be aggregated
///
/// Percent: multiply the `tree_height` by this percentage to get the number of nodes to be used
/// in the aggregated proof i.e. `number_of_ranges_for_aggregation = tree_height * percentage`.
///
/// Number: the exact number of nodes to be used in the aggregated proof. Note that if this number
/// is `> tree_height` it is treated as if it was equal to `tree_height`.
pub enum AggregationFactor {
    Divisor(u8),
    Percent(PercentageInteger),
    Number(u8),
}

impl AggregationFactor {
    fn apply_to(self, tree_height: u8) -> u8 {
        match self {
            Self::Divisor(div) => {
                if div == 0 || div > tree_height {
                    0
                } else {
                    tree_height / div
                }
            }
            Self::Percent(per) => per.apply_to(tree_height),
            Self::Number(num) => num.min(tree_height),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Errors

#[derive(Error, Debug)]
pub enum InclusionProofError {
    #[error("Siblings path verification failed")]
    TreePathError(#[from] PathError),
    #[error("Issues with range proof")]
    RangeProofError(#[from] RangeProofError),
}

#[derive(Error, Debug)]
pub enum RangeProofError {
    #[error("Bulletproofs generation failed")]
    BulletproofGenerationError(ProofError),
    #[error("Bulletproofs verification failed")]
    BulletproofVerificationError(ProofError),
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

    use bulletproofs::PedersenGens;
    use curve25519_dalek_ng::{ristretto::RistrettoPoint, scalar::Scalar};
    use primitive_types::H256;

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
    fn build_test_path() -> (Path<FullNodeContent<Hash>>, RistrettoPoint, H256) {
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
        let (path, _, _) = build_test_path();
        InclusionProof::generate(path, aggregation_factor).unwrap();
    }

    #[test]
    fn verify_works() {
        let aggregation_factor = AggregationFactor::Divisor(2u8);
        let (path, root_commitment, root_hash) = build_test_path();
        let root = Node {
            coord: Coordinate { x: 0u64, y: 3u8 },
            content: CompressedNodeContent::new(root_commitment, root_hash),
        };

        let proof = InclusionProof::generate(path, aggregation_factor).unwrap();
        proof.verify(&root).unwrap();
    }

    // TODO test correct error translation from lower layers (probably should mock the error responses rather than triggering them from the code in the lower layers)
}

// -------------------------------------------------------------------------------------------------
// This was an attempt at making this struct more generic but it's actually just over-complicating the code for no reason

// impl<C: Mergeable + Clone + PartialEq + Debug> InclusionProof<C> {
//     pub fn generate<B, F, G>(
//         path: Path<B>,
//         secret_extractor: F,
//         blinding_extractor: G,
//     ) -> Result<Self, InclusionProofError>
//     where
//         C: From<B>,
//         B: Mergeable + Clone + PartialEq + Debug,
//         F: FnMut(&Node<B>) -> u64,
//         G: FnMut(&Node<B>) -> Scalar,
//     {
//         let aggregation_factor = 2usize;

//         let nodes = path.get_nodes()?;
//         let secrets: Vec<u64> = nodes.iter().map(secret_extractor).collect();
//         let blindings: Vec<Scalar> = nodes.iter().map(blinding_extractor).collect();
//         let range_proof =
//             RangeProofPadding::generate_proof(&secrets, &blindings, aggregation_factor);

//         Ok(InclusionProof {
//             path: path.convert::<C>(),
//             range_proof,
//         })
//     }
// }
