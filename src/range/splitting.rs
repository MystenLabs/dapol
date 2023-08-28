//! Legacy

use crate::utils::bytes_to_usize_with_error;
use bulletproofs::RangeProof;
use curve25519_dalek_ng::{ristretto::CompressedRistretto, scalar::Scalar};
use smtree::{
    error::DecodingError,
    traits::{Serializable, TypeName},
    utils::usize_to_bytes,
};

use super::{
    deserialize_aggregated_proof, deserialize_individual_proofs, generate_aggregated_range_proof,
    generate_single_range_proof, verify_aggregated_range_proof, verify_single_range_proof,
    RangeProvable, RangeVerifiable, AGGREGATED_NUM_BYTE_NUM, INDIVIDUAL_NUM_BYTE_NUM,
    PROOF_SIZE_BYTE_NUM,
};

// RANGE PROOF SPLITTING
// ================================================================================================

#[derive(Debug, Clone)]
pub struct RangeProofSplitting {
    aggregated: Vec<RangeProof>,
    individual: Vec<RangeProof>,
}

impl RangeProofSplitting {
    pub fn get_aggregated(&self) -> &Vec<RangeProof> {
        &self.aggregated
    }

    pub fn get_individual(&self) -> &Vec<RangeProof> {
        &self.individual
    }
}

impl Serializable for RangeProofSplitting {
    /// (aggregated_num || (size_1 || proof_1) || ...) || (individual_num || proof_1 || ...)
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        // append aggregated proofs to the result
        result.append(&mut usize_to_bytes(
            self.get_aggregated().len(),
            AGGREGATED_NUM_BYTE_NUM,
        ));
        for proof in self.get_aggregated() {
            let mut bytes = proof.to_bytes();
            result.append(&mut usize_to_bytes(bytes.len(), PROOF_SIZE_BYTE_NUM));
            result.append(&mut bytes);
        }
        // append individual proofs to the result
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
        // parse aggregated proofs
        let mut aggregated: Vec<RangeProof> = Vec::new();
        let aggregated_num = bytes_to_usize_with_error(bytes, AGGREGATED_NUM_BYTE_NUM, begin)?;
        for _ in 0..aggregated_num {
            let proof = deserialize_aggregated_proof(bytes, begin)?;
            aggregated.push(proof);
        }

        // parse individual proofs
        let individual = deserialize_individual_proofs(bytes, begin)?;

        Ok(RangeProofSplitting {
            aggregated,
            individual,
        })
    }

    /// (aggregated_num || (size_1 || proof_1) || ...) || (individual_num || proof_1 || ...)
    fn deserialize(bytes: &[u8]) -> Result<Self, DecodingError> {
        let mut begin = 0;
        Self::deserialize_as_a_unit(bytes, &mut begin)
    }
}

impl TypeName for RangeProofSplitting {
    fn get_name() -> String {
        "Rang Proof by Splitting".to_owned()
    }
}

impl RangeProvable for RangeProofSplitting {
    fn new(aggregated: &[RangeProof], individual: &[RangeProof]) -> Self {
        RangeProofSplitting {
            aggregated: aggregated.to_vec(),
            individual: individual.to_vec(),
        }
    }

    fn generate_proof(
        secrets: &[u64],
        blindings: &[Scalar],
        aggregated: usize,
    ) -> RangeProofSplitting {
        let mut aggregated_proofs: Vec<RangeProof> = Vec::new();
        let mut base = aggregated.next_power_of_two();
        let mut pos = 0usize;
        while pos < aggregated {
            if aggregated & base > 0 {
                aggregated_proofs.push(generate_aggregated_range_proof(
                    &secrets[pos..pos + base],
                    &blindings[pos..pos + base],
                ));
                pos += base;
            }
            base >>= 1;
        }

        let mut individual_proofs: Vec<RangeProof> = Vec::new();
        while pos < secrets.len() {
            individual_proofs.push(generate_single_range_proof(secrets[pos], &blindings[pos]));
            pos += 1;
        }

        RangeProofSplitting {
            aggregated: aggregated_proofs,
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
        if len > aggregation_factor {
            self.individual.push(generate_single_range_proof(
                secrets[len - 1],
                &blindings[len - 1],
            ));
        } else {
            let mut base = aggregation_factor.next_power_of_two();
            let mut pos = 0usize;
            while pos < len {
                if base & aggregation_factor > 0 {
                    if pos + base == len {
                        self.aggregated.push(generate_aggregated_range_proof(
                            &secrets[pos..pos + base],
                            &blindings[pos..pos + base],
                        ));
                    }
                    pos += base;
                }
                base >>= 1;
            }
        }
    }

    fn remove_proof_by_last_com(&mut self, len: usize, aggregation_factor: usize) {
        if len > aggregation_factor {
            self.individual.pop();
        } else {
            let mut base = aggregation_factor.next_power_of_two();
            let mut pos = 0usize;
            while pos < len {
                if base & aggregation_factor > 0 {
                    if pos + base == len {
                        self.aggregated.pop();
                    }
                    pos += base;
                }
                base >>= 1;
            }
        }
    }
}

impl RangeVerifiable for RangeProofSplitting {
    fn verify(&self, commitments: &[CompressedRistretto]) -> bool {
        let aggregated = commitments.len() - self.individual.len();
        let mut base = aggregated.next_power_of_two();
        let mut pos = 0usize;
        let mut idx = 0usize;
        while pos < aggregated {
            if aggregated & base > 0 {
                if !verify_aggregated_range_proof(
                    &self.aggregated[idx],
                    &commitments[pos..pos + base],
                ) {
                    return false;
                }
                idx += 1;
                pos += base;
            }
            base >>= 1;
        }

        idx = 0;
        while pos < commitments.len() {
            if !verify_single_range_proof(&self.individual[idx], &commitments[pos]) {
                return false;
            }
            idx += 1;
            pos += 1;
        }

        true
    }
}
