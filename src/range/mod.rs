use crate::utils::bytes_to_usize_with_error;
use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use curve25519_dalek_ng::{ristretto::CompressedRistretto, scalar::Scalar};
use merlin::Transcript;
use smtree::error::DecodingError;

mod padding;
pub use padding::RangeProofPadding;

mod splitting;
pub use splitting::RangeProofSplitting;

// The bit size of Bulletproofs,
// i.e., the range proof proves the value in DAPOL is within [0, 2^BIT_SIZE).
// Must be a power of 2 as limited by the Bulletproofs lib.
const BIT_SIZE: usize = 64;

const SINGLE_PROOF_BYTE_NUM: usize = 672;
const PROOF_SIZE_BYTE_NUM: usize = 8;
const AGGREGATED_NUM_BYTE_NUM: usize = 2;
const INDIVIDUAL_NUM_BYTE_NUM: usize = 8;

// TRAITS
// ================================================================================================

pub trait RangeProvable {
    fn new(aggregated: &[RangeProof], individual: &[RangeProof]) -> Self;

    fn generate_proof(secrets: &[u64], blindings: &[Scalar], aggregation_factor: usize) -> Self;

    fn generate_proof_by_new_com(
        &mut self,
        secrets: &[u64],
        blindings: &[Scalar],
        aggregation_factor: usize,
    );

    fn remove_proof_by_last_com(&mut self, len: usize, aggregation_factor: usize);
}

pub trait RangeVerifiable {
    fn verify(&self, commitments: &[CompressedRistretto]) -> bool;
}

// PROOF GENERATION
// ================================================================================================

fn generate_single_range_proof(secret: u64, blinding: &Scalar) -> RangeProof {
    let pc_gens = PedersenGens::default();
    let bp_gens = BulletproofGens::new(BIT_SIZE, 1);
    let mut prover_transcript = Transcript::new(&[]);
    let (proof, _commitments) = RangeProof::prove_single(
        &bp_gens,
        &pc_gens,
        &mut prover_transcript,
        secret,
        blinding,
        BIT_SIZE,
    )
    .expect("Error in generating aggregated range proof");
    proof
}

fn generate_aggregated_range_proof(secrets: &[u64], blindings: &[Scalar]) -> RangeProof {
    let pc_gens = PedersenGens::default();
    let bp_gens = BulletproofGens::new(BIT_SIZE, secrets.len());
    let mut prover_transcript = Transcript::new(&[]);
    let (proof, _commitments) = RangeProof::prove_multiple(
        &bp_gens,
        &pc_gens,
        &mut prover_transcript,
        &secrets,
        &blindings,
        BIT_SIZE,
    )
    .expect("Error in generating aggregated range proof");
    proof
}

// PROOF VERIFICATION
// ================================================================================================

fn verify_single_range_proof(proof: &RangeProof, commitment: &CompressedRistretto) -> bool {
    let pc_gens = PedersenGens::default();
    let bp_gens = BulletproofGens::new(BIT_SIZE, 1);
    let mut verifier_transcript = Transcript::new(&[]);
    if proof
        .verify_single(
            &bp_gens,
            &pc_gens,
            &mut verifier_transcript,
            commitment,
            BIT_SIZE,
        )
        .is_err()
    {
        return false;
    }
    true
}

fn verify_aggregated_range_proof(proof: &RangeProof, commitments: &[CompressedRistretto]) -> bool {
    let pc_gens = PedersenGens::default();
    let bp_gens = BulletproofGens::new(BIT_SIZE, commitments.len());
    let mut verifier_transcript = Transcript::new(&[]);
    if proof
        .verify_multiple(
            &bp_gens,
            &pc_gens,
            &mut verifier_transcript,
            commitments,
            BIT_SIZE,
        )
        .is_err()
    {
        return false;
    }
    true
}

// PROOF DESERIALIZATION
// ================================================================================================

fn deserialize_range_proof(
    bytes: &[u8],
    byte_num: usize,
    begin: &mut usize,
) -> Result<RangeProof, DecodingError> {
    if bytes.len() - *begin < byte_num {
        return Err(DecodingError::BytesNotEnough);
    }
    let proof = RangeProof::from_bytes(&bytes[*begin..*begin + byte_num]).map_err(|e| {
        DecodingError::ValueDecodingError {
            msg: format!("{}", e),
        }
    })?;
    *begin += byte_num;
    Ok(proof)
}

fn deserialize_aggregated_proof(
    bytes: &[u8],
    begin: &mut usize,
) -> Result<RangeProof, DecodingError> {
    let size = bytes_to_usize_with_error(bytes, PROOF_SIZE_BYTE_NUM, begin)?;
    let proof = deserialize_range_proof(bytes, size, begin)?;
    Ok(proof)
}

fn deserialize_individual_proofs(
    bytes: &[u8],
    begin: &mut usize,
) -> Result<Vec<RangeProof>, DecodingError> {
    let mut individual: Vec<RangeProof> = Vec::new();
    let individual_num = bytes_to_usize_with_error(bytes, INDIVIDUAL_NUM_BYTE_NUM, begin)?;
    for _ in 0..individual_num {
        let proof = deserialize_range_proof(bytes, SINGLE_PROOF_BYTE_NUM, begin)?;
        individual.push(proof);
    }
    Ok(individual)
}
