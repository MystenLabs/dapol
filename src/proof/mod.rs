//! Legacy

use crate::{DapolNode, RangeVerifiable};
use digest::Digest;
use smtree::{error::DecodingError, proof::MerkleProof, traits::Serializable};

mod node;
pub use node::DapolProofNode;

#[cfg(test)]
mod tests;

// DAPOL PROOF
// ================================================================================================

#[derive(Default, Debug)]
pub struct DapolProof<D, R>
where
    D: Digest + Default + Clone + std::fmt::Debug,
    R: Clone + RangeVerifiable + Serializable,
{
    merkle: MerkleProof<DapolNode<D>>,
    range_proofs: R,
}

impl<D, R> DapolProof<D, R>
where
    D: Digest + Default + Clone + std::fmt::Debug,
    R: Clone + RangeVerifiable + Serializable,
{
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    pub fn new(merkle_proof: MerkleProof<DapolNode<D>>, range_proofs: R) -> Self {
        DapolProof {
            merkle: merkle_proof,
            range_proofs,
        }
    }

    // PUBLIC METHODS
    // --------------------------------------------------------------------------------------------

    pub fn verify(&self, root: &DapolProofNode<D>, leaf: &DapolProofNode<D>) -> bool {
        // TODO: check if the proof was created for a single leaf
        if !self.merkle.verify(leaf, root) {
            return false;
        }
        self.verify_proof()
    }

    pub fn verify_batch(&self, root: &DapolProofNode<D>, leaves: &[DapolProofNode<D>]) -> bool {
        if !self.merkle.verify_batch(leaves, root) {
            return false;
        }
        self.verify_proof()
    }

    pub fn get_range_proofs(&self) -> &R {
        &self.range_proofs
    }

    pub fn get_merkle_path(&self) -> &MerkleProof<DapolNode<D>> {
        &self.merkle
    }

    // SERIALIZATION / DESERIALIZATION
    // --------------------------------------------------------------------------------------------

    /// range_proof || merkle_path
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.append(&mut self.range_proofs.serialize());
        bytes.append(&mut self.merkle.serialize());
        bytes
    }

    /// range_proof || merkle_path
    pub fn deserialize(bytes: &[u8]) -> Result<Self, DecodingError> {
        let mut begin = 0;
        let range_proofs = R::deserialize_as_a_unit(bytes, &mut begin)?;
        let merkle = MerkleProof::<DapolNode<D>>::deserialize_as_a_unit(bytes, &mut begin)?;
        Ok(DapolProof {
            merkle,
            range_proofs,
        })
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    fn verify_proof(&self) -> bool {
        let mut commitments = Vec::new();
        for i in 0..self.merkle.get_siblings_num() {
            commitments.push((*self.merkle.get_sibling_at_idx(i)).get_com().compress());
        }
        self.range_proofs.verify(&commitments[..])
    }
}
