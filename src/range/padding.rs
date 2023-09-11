//! Legacy

use bulletproofs::{PedersenGens, RangeProof};
use curve25519_dalek_ng::{ristretto::CompressedRistretto, scalar::Scalar};
use smtree::{
    error::DecodingError,
    traits::{Serializable, TypeName},
    utils::usize_to_bytes,
};
use std::cmp::Ordering;

use super::{
    deserialize_aggregated_proof, deserialize_individual_proofs, generate_aggregated_range_proof,
    generate_single_range_proof, verify_aggregated_range_proof, verify_single_range_proof,
    RangeProvable, RangeVerifiable, INDIVIDUAL_NUM_BYTE_NUM, PROOF_SIZE_BYTE_NUM,
};

// RANGE PROOF PADDING
// ================================================================================================

#[derive(Debug, Clone)]
pub struct RangeProofPadding {
    aggregated: Vec<RangeProof>,
    individual: Vec<RangeProof>,
}

impl RangeProofPadding {
    pub fn get_aggregated(&self) -> &RangeProof {
        if self.aggregated.is_empty() {
            panic!(); // TODO
        }
        &self.aggregated[0]
    }

    pub fn get_individual(&self) -> &Vec<RangeProof> {
        &self.individual
    }
}

impl Serializable for RangeProofPadding {
    /// (aggregated_size || aggregated_proof) || (individual_num || proof_1 || ...)
    fn serialize(&self) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();
        let mut bytes = self.get_aggregated().to_bytes();
        result.append(&mut usize_to_bytes(bytes.len(), PROOF_SIZE_BYTE_NUM));
        result.append(&mut bytes);
        result.append(&mut usize_to_bytes(
            self.get_individual().len(),
            INDIVIDUAL_NUM_BYTE_NUM,
        ));
        for proof in self.get_individual() {
            result.append(&mut proof.to_bytes());
        }
        result
    }

    fn deserialize_as_a_unit(bytes: &[u8], begin: &mut usize) -> Result<Self, DecodingError> {
        let aggregated = deserialize_aggregated_proof(&bytes, begin)?;
        let individual = deserialize_individual_proofs(bytes, begin)?;
        Ok(RangeProofPadding {
            aggregated: vec![aggregated],
            individual,
        })
    }

    /// (aggregated_size || aggregated_proof) || (individual_num || proof_1 || ...)
    fn deserialize(bytes: &[u8]) -> Result<Self, DecodingError> {
        let mut begin = 0;
        Self::deserialize_as_a_unit(bytes, &mut begin)
    }
}

impl TypeName for RangeProofPadding {
    fn get_name() -> String {
        "Rang Proof by Padding".to_owned()
    }
}

impl RangeProvable for RangeProofPadding {
    fn new(aggregated: &[RangeProof], individual: &[RangeProof]) -> Self {
        if aggregated.len() > 1 {
            panic!(); //TODO
        }
        RangeProofPadding {
            aggregated: aggregated.to_vec(),
            individual: individual.to_vec(),
        }
    }

    fn generate_proof(
        _secrets: &[u64],
        _blindings: &[Scalar],
        aggregated: usize,
    ) -> RangeProofPadding {
        let mut secrets = Vec::<u64>::new();
        let mut blindings = Vec::<Scalar>::new();
        for _i in 0..aggregated {
            secrets.push(_secrets[_i]);
            blindings.push(_blindings[_i]);
        }
        let power = aggregated.next_power_of_two();
        for _i in aggregated..power {
            secrets.push(0);
            blindings.push(Scalar::one());
        }
        let aggregated_proof =
            generate_aggregated_range_proof(&secrets[0..power], &blindings[0..power]);

        let mut individual_proofs: Vec<RangeProof> = Vec::new();
        let mut pos = aggregated;
        while pos < _secrets.len() {
            individual_proofs.push(generate_single_range_proof(_secrets[pos], &_blindings[pos]));
            pos += 1;
        }

        RangeProofPadding {
            aggregated: vec![aggregated_proof],
            individual: individual_proofs,
        }
    }

    fn generate_proof_by_new_com(
        &mut self,
        secrets: &[u64],
        blindings: &[Scalar],
        aggregation_factor: usize,
    ) {
        let len = secrets.len();
        match len.cmp(&aggregation_factor) {
            Ordering::Greater => {
                self.individual.push(generate_single_range_proof(
                    secrets[len - 1],
                    &blindings[len - 1],
                ));
            }
            Ordering::Equal => {
                let base = aggregation_factor.next_power_of_two();
                let mut _secrets = Vec::<u64>::new();
                let mut _blindings = Vec::<Scalar>::new();
                for _i in 0..len {
                    _secrets.push(secrets[_i]);
                    _blindings.push(blindings[_i]);
                }
                for _i in len..base {
                    _secrets.push(0);
                    _blindings.push(Scalar::one());
                }
                self.aggregated.push(generate_aggregated_range_proof(
                    &_secrets[..],
                    &_blindings[..],
                ));
            }
            _ => {}
        }
    }

    fn remove_proof_by_last_com(&mut self, len: usize, aggregation_factor: usize) {
        match len.cmp(&aggregation_factor) {
            Ordering::Greater => {
                self.individual.pop();
            }
            Ordering::Equal => {
                self.aggregated.pop();
            }
            _ => {}
        }
    }
}

impl RangeVerifiable for RangeProofPadding {
    fn verify(&self, _commitments: &[CompressedRistretto]) -> bool {
        let mut commitments = Vec::<CompressedRistretto>::new();
        let aggregated = _commitments.len() - self.individual.len();
        for item in _commitments.iter().take(aggregated) {
            commitments.push(*item);
        }
        let power = aggregated.next_power_of_two();
        let pc_gens = PedersenGens::default();
        let com_padding = pc_gens.commit(Scalar::from(0u64), Scalar::one()).compress();
        for _i in aggregated..power {
            commitments.push(com_padding);
        }
        if !verify_aggregated_range_proof(&self.get_aggregated(), &commitments[0..power]) {
            return false;
        }

        let mut idx = 0;
        let mut pos = aggregated;
        while pos < _commitments.len() {
            if !verify_single_range_proof(&self.individual[idx], &_commitments[pos]) {
                return false;
            }
            idx += 1;
            pos += 1;
        }

        true
    }
}
