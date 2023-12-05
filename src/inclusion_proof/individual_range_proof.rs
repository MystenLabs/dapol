//! Single range proof generation and verification using the Bulletproofs
//! protocol.
//!
//! See also [super][aggregated_range_proof] which is used for batching range
//! proofs with an efficiency gain.
//!
//! Note `upper_bound_bit_length` parameter is in u8 because it is not expected
//! to require bounds higher than $2^256$.

// TODO more docs, on Pedersen gens maybe, on bulletproof gens maybe, on
// transcript maybe

use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use curve25519_dalek_ng::{ristretto::CompressedRistretto, scalar::Scalar};
use merlin::Transcript;
use serde::{Deserialize, Serialize};

use super::RangeProofError;

#[derive(Debug, Serialize, Deserialize)]
pub struct IndividualRangeProof(RangeProof);

/// Maximum number of parties that can produce an aggregated proof.
///
/// It is required to produce the generator group elements for Bulletproofs
/// protocol. The value is set to 1 because only 1 proof is generated at a time
/// and so no aggregation is required.
const PARTY_CAPACITY: usize = 1;

/// The transcript initial state must be the same for proof generation and
/// verification.
fn new_transcript() -> Transcript {
    Transcript::new(b"IndividualRangeProof")
}

impl IndividualRangeProof {
    /// Generate a range proof using the Bulletproofs protocol.
    ///
    /// The proof will convince a verifier that $0 <= secret <=
    /// 2^upper_bound_bit_length$.
    ///
    /// `upper_bound_bit_length` is in u8 because it is not expected to require
    /// bounds higher than $2^256$.
    pub fn generate(
        secret: u64,
        blinding_factor: &Scalar,
        upper_bound_bit_length: u8,
    ) -> Result<IndividualRangeProof, RangeProofError> {
        let pc_gens = PedersenGens::default();
        let bp_gens = BulletproofGens::new(upper_bound_bit_length as usize, PARTY_CAPACITY);

        match RangeProof::prove_single(
            &bp_gens,
            &pc_gens,
            &mut new_transcript(),
            secret,
            blinding_factor,
            upper_bound_bit_length as usize,
        ) {
            Err(underlying_err) => Err(RangeProofError::BulletproofGenerationError(underlying_err)),
            Ok((proof, _commitment)) => Ok(IndividualRangeProof(proof)),
        }
    }

    /// Verify the Bulletproof.
    ///
    /// `commitment` - the Pedersen commitment, in compressed form.
    ///
    /// `upper_bound_bit_length` - $2^upper_bound_bit_length$ is the value that
    /// the commitment should be less than.
    ///
    /// Both `commitment` & `upper_bound_bit_length` should be the same as the
    /// values that were was used to generate the proof.
    pub fn verify(
        &self,
        commitment: &CompressedRistretto,
        upper_bound_bit_length: u8,
    ) -> Result<(), RangeProofError> {
        let pc_gens = PedersenGens::default();
        let bp_gens = BulletproofGens::new(upper_bound_bit_length as usize, PARTY_CAPACITY);

        match self.0.verify_single(
            &bp_gens,
            &pc_gens,
            &mut new_transcript(),
            commitment,
            upper_bound_bit_length as usize,
        ) {
            Err(underlying_err) => Err(RangeProofError::BulletproofVerificationError(
                underlying_err,
            )),
            Ok(_) => Ok(()),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Unit tests
//
// Note that the tests here penetrate into the Bulletproofs library, as opposed
// to mocking that library. This is intentional because we cannot write our own
// unit tests for that library so we rather use the unit tests here as
// integration tests.

// TODO how to get the generation to emit an error from the underlying
// bulletproof library? TODO need to test that bound bit lengths not equal to
// one of 8, 16, 32, 64 produce errors in generation/verification
#[cfg(test)]
mod tests {
    use bulletproofs::ProofError;

    use super::*;
    use crate::utils::test_utils::assert_err;

    #[test]
    fn generate_works() {
        let secret = 7u64;
        let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
        let upper_bound_bit_length = 32u8;

        IndividualRangeProof::generate(secret, &blinding_factor, upper_bound_bit_length).unwrap();
    }

    // this is unexpected but verification will definitely fail so it's not a
    // problem
    #[test]
    fn generation_works_when_secret_out_of_bounds() {
        // secret = 2^32 > 2^8 = upper_bound
        let invalid_upper_bound = 8u8;

        let secret = 2u64.pow(10u32);
        let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");

        let _ =
            IndividualRangeProof::generate(secret, &blinding_factor, invalid_upper_bound).unwrap();
    }

    #[test]
    fn verify_works() {
        let secret = 7u64;
        let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
        let commitment = PedersenGens::default().commit(Scalar::from(secret), blinding_factor);
        let upper_bound_bit_length = 32u8;

        let proof =
            IndividualRangeProof::generate(secret, &blinding_factor, upper_bound_bit_length)
                .unwrap();

        proof
            .verify(&commitment.compress(), upper_bound_bit_length)
            .unwrap();
    }

    #[test]
    fn verification_error_when_secret_out_of_bounds_with_different_bounds() {
        // secret = 2^32 > 2^8 = upper_bound
        let valid_upper_bound = 64u8;
        let invalid_upper_bound = 8u8;
        let secret = 2u64.pow(10u32);

        let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
        let commitment = PedersenGens::default().commit(Scalar::from(secret), blinding_factor);

        let proof =
            IndividualRangeProof::generate(secret, &blinding_factor, valid_upper_bound).unwrap();

        let res = proof.verify(&commitment.compress(), invalid_upper_bound);

        assert_err!(
            res,
            Err(RangeProofError::BulletproofVerificationError(
                ProofError::VerificationError
            ))
        );
    }

    #[test]
    fn verification_error_when_secret_out_of_bounds_with_different_bounds_reverse() {
        // secret = 2^32 > 2^8 = upper_bound
        let valid_upper_bound = 64u8;
        let invalid_upper_bound = 8u8;
        let secret = 2u64.pow(10u32);

        let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
        let commitment = PedersenGens::default().commit(Scalar::from(secret), blinding_factor);

        let proof =
            IndividualRangeProof::generate(secret, &blinding_factor, invalid_upper_bound).unwrap();

        let res = proof.verify(&commitment.compress(), valid_upper_bound);

        assert_err!(
            res,
            Err(RangeProofError::BulletproofVerificationError(
                ProofError::VerificationError
            ))
        );
    }

    #[test]
    fn verification_error_when_secret_out_of_bounds_with_same_bounds() {
        // secret = 2^32 > 2^8 = upper_bound
        let upper_bound_bit_length = 8u8;
        let secret = 2u64.pow(10u32);

        let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
        let commitment = PedersenGens::default().commit(Scalar::from(secret), blinding_factor);

        // NOTE the proof generation succeeds even though the secret value is greater
        // than the bound
        let proof =
            IndividualRangeProof::generate(secret, &blinding_factor, upper_bound_bit_length)
                .unwrap();

        let res = proof.verify(&commitment.compress(), upper_bound_bit_length);

        assert_err!(
            res,
            Err(RangeProofError::BulletproofVerificationError(
                ProofError::VerificationError
            ))
        );
    }

    #[test]
    fn verification_error_when_commitment_not_same_as_secret_used_for_generation() {
        let secret = 7u64; // for generation
        let other_secret = 8u64; // for the commitment, for verification

        let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
        let commitment =
            PedersenGens::default().commit(Scalar::from(other_secret), blinding_factor);

        let upper_bound_bit_length = 32u8;

        let proof =
            IndividualRangeProof::generate(secret, &blinding_factor, upper_bound_bit_length)
                .unwrap();

        let res = proof.verify(&commitment.compress(), upper_bound_bit_length);

        assert_err!(
            res,
            Err(RangeProofError::BulletproofVerificationError(
                ProofError::VerificationError
            ))
        );
    }

    #[test]
    fn verification_error_when_commitment_not_same_as_blinding_used_for_generation() {
        let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
        let other_blinding_factor =
            Scalar::from_bytes_mod_order(*b"44445555666677778888111122223333");

        let secret = 7u64;
        let commitment =
            PedersenGens::default().commit(Scalar::from(secret), other_blinding_factor);

        let upper_bound_bit_length = 32u8;

        let proof =
            IndividualRangeProof::generate(secret, &blinding_factor, upper_bound_bit_length)
                .unwrap();

        let res = proof.verify(&commitment.compress(), upper_bound_bit_length);

        assert_err!(
            res,
            Err(RangeProofError::BulletproofVerificationError(
                ProofError::VerificationError
            ))
        );
    }
}
