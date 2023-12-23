//! Aggregated range proof generation & verification using Bulletproofs
//! protocol.
//!
//! See also [super][individual_range_proof] which is used for single range
//! proofs.
//!
//! Bulletproofs allows multiple ranges to be grouped together (aggregated) into
//! a single proof that is more efficient to compute than producing individual
//! range proofs.
//!
//! The Bulletproofs library used only supports aggregation if the number of
//! ranges to prove is a power of 2. There are 2 different ways of getting
//! around this limitation for arbitrary number of ranges $n$:
//! 1. Increase $n$ to the next power of 2 by padding with extra superfluous
//! values 2. Perform multiple range proofs, one for each of the on-bits in the
//! $n$'s base-2 representation
//!
//! Padding example:
//! Suppose $n=5$, `ranges = [range_1, range_2, range_3, range_4, range_5]`. We
//! find $m=8$ to be the next power of 2, so we add 3 values to our array to get
//! the size of the array to equal $m$: `ranges_extended = [range_1, range_2,
//! range_3, range_4, range_5, 0, 0, 0]` We default to 0 here but any value can
//! be used.
//!
//! Splitting example:
//! Suppose $n=5$ as above. $n$'s base-2 representation is 101 which has 2
//! on-bits. We create 2 aggregated range proofs by splitting the array into the
//! following 2 pieces:
//! - `ranges_a = [range_2, range_3, range_4, range_5]`
//! - `ranges_b = [range_1]`
//! We gave the tail of the array the highest power of 2 but one can also do it
//! instead by associating the highest with the top of the array.
//!
//! When padding? When splitting?
//! Each is more efficient in different cases. For $n=127$ splitting would be
//! more efficient, but for $n=255$ padding would win.

use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use curve25519_dalek_ng::{ristretto::CompressedRistretto, scalar::Scalar};
use merlin::Transcript;
use serde::{Deserialize, Serialize};

use super::RangeProofError;

/// `input_size` is u8 because it will be directly related to the length of a
/// tree path, which is equal to the height of the tree, which is also stored as
/// u8.
#[derive(Debug, Serialize, Deserialize)]
pub enum AggregatedRangeProof {
    Padding {
        proof: RangeProof,
        input_size: u8,
    },
    Splitting {
        proofs: Vec<(RangeProof, usize)>, /* the 2nd value is the number of values in the
                                           * aggregated proof */
        input_size: u8,
    },
}

/// Used to pad the inputs to proof generation so that the length can be made a
/// power of 2, a requirement for the [bulletproofs] library.
// TODO are these the best option for the pad? Maybe there is another option
// that gives efficiency guarantees
fn padding_tuple() -> (u64, Scalar) {
    (0, Scalar::one())
}

/// The transcript initial state must be the same for proof generation and
/// verification.
// TODO we may want to make this different for padding & splitting because it
// may help with deserialization
fn new_transcript() -> Transcript {
    Transcript::new(b"AggregatedRangeProof")
}

impl AggregatedRangeProof {
    /// Generate an aggregated proof.
    ///
    /// Whether the padding method or splitting method is used will be
    /// determined by the input size, so that the most efficient method is
    /// used. The code currently just naively checks whether the size lies
    /// in the first or second half of the gap between the 2 powers of 2 on
    /// either side if the size value.
    pub fn generate(
        secrets_blindings_tuples: &Vec<(u64, Scalar)>,
        upper_bound_bit_length: u8,
    ) -> Result<AggregatedRangeProof, RangeProofError> {
        let size = secrets_blindings_tuples.len();
        let next_pow_2 = size.next_power_of_two();
        let prev_pow_2 = next_pow_2 / 2;

        // TODO this choice of split is fairly arbitrary, one should run the numbers and
        // figure out where the best split is
        if size < (next_pow_2 - prev_pow_2) / 2 {
            Self::generate_with_splitting(secrets_blindings_tuples, upper_bound_bit_length)
        } else {
            Self::generate_with_padding(secrets_blindings_tuples, upper_bound_bit_length)
        }
    }

    /// Generate aggregated proof using the padding method.
    ///
    /// `secrets_blindings_tuples` is a vector of secret & blinding_factor
    /// tuples. `upper_bound_bit_length` is the power of 2 that the range
    /// proof will show the secret value to be less than i.e. `secret <
    /// 2^upper_bound_bit_length`.
    pub fn generate_with_padding(
        secrets_blindings_tuples: &Vec<(u64, Scalar)>,
        upper_bound_bit_length: u8,
    ) -> Result<AggregatedRangeProof, RangeProofError> {
        // We want a mutable vector so that we can add padding to it.
        // Since proofs will be for paths in a binary tree the length of the input
        // should be the the same as the height of the tree, which can
        // reasonably be assumed to be less than 256, small enough for the copy
        // not to affect performance too much.
        let mut secrets_blindings_tuples_clone = secrets_blindings_tuples.clone();

        // Is this cast safe? Yes because the tree height (which is the same as the
        // length of the input) is also stored as a u8.
        let input_size = secrets_blindings_tuples.len() as u8;
        let next_pow_2 = input_size.next_power_of_two();

        for _i in input_size..next_pow_2 {
            secrets_blindings_tuples_clone.push(padding_tuple());
        }

        let pc_gens = PedersenGens::default();
        let bp_gens = BulletproofGens::new(upper_bound_bit_length as usize, next_pow_2 as usize);

        let (secrets, blinding_factors): (Vec<u64>, Vec<Scalar>) =
            secrets_blindings_tuples_clone.into_iter().unzip();

        match RangeProof::prove_multiple(
            &bp_gens,
            &pc_gens,
            &mut new_transcript(),
            &secrets,
            &blinding_factors,
            upper_bound_bit_length as usize,
        ) {
            Err(underlying_err) => Err(RangeProofError::BulletproofGenerationError(underlying_err)),
            Ok((proof, _commitments)) => Ok(AggregatedRangeProof::Padding { proof, input_size }),
        }
    }

    /// Generate aggregated proof using the splitting method.
    ///
    /// `secrets_blindings_tuples` is a vector of secret & blinding_factor
    /// tuples. `upper_bound_bit_length` is the power of 2 that the range
    /// proof will show the secret value to be less than i.e. `secret <
    /// 2^upper_bound_bit_length`.
    pub fn generate_with_splitting(
        secrets_blindings_tuples: &Vec<(u64, Scalar)>,
        upper_bound_bit_length: u8,
    ) -> Result<AggregatedRangeProof, RangeProofError> {
        let pc_gens = PedersenGens::default();

        let mut prover_transcript = new_transcript();

        // Is this cast safe? Yes because the tree height (which is the same as the
        // length of the input) is also stored as a u8.
        let input_size = secrets_blindings_tuples.len() as u8;
        let mut next_pow_2 = input_size.next_power_of_two();

        // We want mutable vectors to make the code easier to read.
        // Since proofs will be for paths in a binary tree the length of the input
        // should be the the same as the height of the tree, which can
        // reasonably be assumed to be less than 256, small enough for the copy
        // not to affect performance too much.
        let (mut secrets, mut blinding_factors): (Vec<u64>, Vec<Scalar>) =
            secrets_blindings_tuples.iter().cloned().unzip();

        // We avoid initializing with capacity because it's not easy to get the exact
        // capacity and the vectors should be relatively short so performance
        // should not be impacted.
        let mut proofs: Vec<(RangeProof, usize)> = Vec::new();

        // We slowly shave off parts of the 2 vectors (from the tail) till there is
        // nothing left.
        while !secrets.is_empty() {
            if input_size & next_pow_2 > 0 {
                let bp_gens =
                    BulletproofGens::new(upper_bound_bit_length as usize, next_pow_2 as usize);
                let index = secrets.len() - next_pow_2 as usize;

                let (proof, _commitments) = RangeProof::prove_multiple(
                    &bp_gens,
                    &pc_gens,
                    &mut prover_transcript,
                    &secrets.split_off(index),
                    &blinding_factors.split_off(index),
                    upper_bound_bit_length as usize,
                )
                .map_err(RangeProofError::BulletproofGenerationError)?;

                proofs.push((proof, next_pow_2 as usize));
            }
            next_pow_2 >>= 1;
        }

        Ok(AggregatedRangeProof::Splitting { proofs, input_size })
    }

    pub fn verify(
        &self,
        commitments: &Vec<CompressedRistretto>,
        upper_bound_bit_length: u8,
    ) -> Result<(), RangeProofError> {
        if commitments.len() != self.input_size() as usize {
            return Err(RangeProofError::InputVectorLengthMismatch);
        }

        let pc_gens = PedersenGens::default();
        let mut prover_transcript = new_transcript();

        // We want a mutable vector.
        // Since proofs will be for paths in a binary tree the length of the input
        // should be the the same as the height of the tree, which can
        // reasonably be assumed to be less than 256, small enough for the copy
        // not to affect performance too much.
        let mut commitments_clone = commitments.clone();

        match self {
            AggregatedRangeProof::Padding { proof, input_size } => {
                let next_pow_2 = input_size.next_power_of_two();
                let bp_gens =
                    BulletproofGens::new(upper_bound_bit_length as usize, next_pow_2 as usize);
                let commitment_pad = pc_gens
                    .commit(Scalar::from(padding_tuple().0), padding_tuple().1)
                    .compress();

                for _i in *input_size..next_pow_2 {
                    commitments_clone.push(commitment_pad);
                }

                proof.verify_multiple(
                    &bp_gens,
                    &pc_gens,
                    &mut prover_transcript,
                    commitments,
                    upper_bound_bit_length as usize,
                )
            }
            AggregatedRangeProof::Splitting {
                proofs,
                input_size: _,
            } => proofs.iter().try_for_each(|(proof, length)| {
                let bp_gens = BulletproofGens::new(upper_bound_bit_length as usize, *length);
                let commitments_slice = commitments_clone.split_off(commitments.len() - length);

                proof.verify_multiple(
                    &bp_gens,
                    &pc_gens,
                    &mut prover_transcript,
                    &commitments_slice,
                    upper_bound_bit_length as usize,
                )
            }),
        }
        .map_err(RangeProofError::BulletproofVerificationError)
    }

    fn input_size(&self) -> u8 {
        match self {
            AggregatedRangeProof::Padding {
                proof: _,
                input_size: input_length,
            } => *input_length,
            AggregatedRangeProof::Splitting {
                proofs: _,
                input_size: input_length,
            } => *input_length,
        }
    }
}

// TODO need to test the generate function once we have decided on the best
// split point
#[cfg(test)]
mod tests {
    use bulletproofs::ProofError;

    use super::*;
    use crate::utils::test_utils::assert_err;

    // This test does not call any of the above code but it just checks to make sure
    // that there is no drop in efficiency with the `next_power_of_two`
    // function. Basically need to check that the function does not give a
    // greater value if the input is already a power of 2.
    #[test]
    // TODO fuzz the power of 2 here
    fn next_power_of_2_works_for_boundary_value() {
        let length = 8u64;
        let power = length.next_power_of_two();
        assert_eq!(power, length);
    }

    fn build_secrets_blindings_tuples() -> Vec<(u64, Scalar)> {
        let mut result = Vec::new();

        let secret = 7u64;
        let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
        result.push((secret, blinding_factor));

        let secret = 11u64;
        let blinding_factor = Scalar::from_bytes_mod_order(*b"44445555666677778888111122223333");
        result.push((secret, blinding_factor));

        result
    }

    mod padding {
        use super::*;

        #[test]
        fn generate_works() {
            let upper_bound_bit_length = 32u8;
            AggregatedRangeProof::generate_with_padding(
                &build_secrets_blindings_tuples(),
                upper_bound_bit_length,
            )
            .unwrap();
        }

        #[test]
        fn verify_works_for_padding() {
            let upper_bound_bit_length = 32u8;
            let values = build_secrets_blindings_tuples();
            let commitments: Vec<CompressedRistretto> = values
                .clone()
                .into_iter()
                .map(|(secret, blinding_factor)| {
                    PedersenGens::default()
                        .commit(Scalar::from(secret), blinding_factor)
                        .compress()
                })
                .collect();

            let proof =
                AggregatedRangeProof::generate_with_padding(&values, upper_bound_bit_length)
                    .unwrap();

            proof.verify(&commitments, upper_bound_bit_length).unwrap();
        }

        #[test]
        fn verification_error_when_secret_out_of_bounds_with_different_bounds() {
            // secret = 2^32 > 2^8 = upper_bound
            let valid_upper_bound = 64u8;
            let invalid_upper_bound = 8u8;
            let secret = 2u64.pow(10u32);

            let blinding_factor =
                Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
            let commitment = vec![PedersenGens::default()
                .commit(Scalar::from(secret), blinding_factor)
                .compress()];
            let input = vec![(secret, blinding_factor)];

            let proof =
                AggregatedRangeProof::generate_with_padding(&input, valid_upper_bound).unwrap();

            let res = proof.verify(&commitment, invalid_upper_bound);

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

            let blinding_factor =
                Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
            let commitment = vec![PedersenGens::default()
                .commit(Scalar::from(secret), blinding_factor)
                .compress()];
            let input = vec![(secret, blinding_factor)];

            // NOTE the proof generation succeeds even though the secret value is greater
            // than the bound
            let proof = AggregatedRangeProof::generate_with_padding(&input, upper_bound_bit_length)
                .unwrap();

            let res = proof.verify(&commitment, upper_bound_bit_length);

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

            let blinding_factor =
                Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
            let commitment = vec![PedersenGens::default()
                .commit(Scalar::from(other_secret), blinding_factor)
                .compress()];
            let input = vec![(secret, blinding_factor)];

            let upper_bound_bit_length = 32u8;

            let proof = AggregatedRangeProof::generate_with_padding(&input, upper_bound_bit_length)
                .unwrap();

            let res = proof.verify(&commitment, upper_bound_bit_length);

            assert_err!(
                res,
                Err(RangeProofError::BulletproofVerificationError(
                    ProofError::VerificationError
                ))
            );
        }
    }

    mod splitting {
        use super::*;

        #[test]
        fn generate_works_for_splitting() {
            let upper_bound_bit_length = 32u8;
            AggregatedRangeProof::generate_with_splitting(
                &build_secrets_blindings_tuples(),
                upper_bound_bit_length,
            )
            .unwrap();
        }

        #[test]
        fn verify_works_for_splitting() {
            let upper_bound_bit_length = 32u8;
            let values = build_secrets_blindings_tuples();
            let commitments = values
                .clone()
                .into_iter()
                .map(|(secret, blinding_factor)| {
                    PedersenGens::default()
                        .commit(Scalar::from(secret), blinding_factor)
                        .compress()
                })
                .collect();

            let proof =
                AggregatedRangeProof::generate_with_splitting(&values, upper_bound_bit_length)
                    .unwrap();

            proof.verify(&commitments, upper_bound_bit_length).unwrap();
        }

        #[test]
        fn verification_error_when_secret_out_of_bounds_with_different_bounds() {
            // secret = 2^32 > 2^8 = upper_bound
            let upper_bound_bit_length = 64u8;
            let other_upper_bound_bit_length = 8u8;

            let secret = 2u64.pow(10u32);
            let blinding_factor =
                Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
            let commitment = vec![PedersenGens::default()
                .commit(Scalar::from(secret), blinding_factor)
                .compress()];
            let input = vec![(secret, blinding_factor)];

            let proof =
                AggregatedRangeProof::generate_with_splitting(&input, upper_bound_bit_length)
                    .unwrap();

            let res = proof.verify(&commitment, other_upper_bound_bit_length);

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

            let blinding_factor =
                Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
            let commitment = vec![PedersenGens::default()
                .commit(Scalar::from(secret), blinding_factor)
                .compress()];
            let input = vec![(secret, blinding_factor)];

            // NOTE the proof generation succeeds even though the secret value is greater
            // than the bound
            let proof =
                AggregatedRangeProof::generate_with_splitting(&input, upper_bound_bit_length)
                    .unwrap();

            let res = proof.verify(&commitment, upper_bound_bit_length);

            assert_err!(
                res,
                Err(RangeProofError::BulletproofVerificationError(
                    ProofError::VerificationError
                ))
            );
        }
    }

    #[test]
    fn verification_error_when_commitment_not_same_as_secret_used_for_generation() {
        let secret = 7u64; // for generation
        let other_secret = 8u64; // for the commitment, for verification

        let blinding_factor = Scalar::from_bytes_mod_order(*b"33334444555566667777888811112222");
        let commitment = vec![PedersenGens::default()
            .commit(Scalar::from(other_secret), blinding_factor)
            .compress()];
        let input = vec![(secret, blinding_factor)];

        let upper_bound_bit_length = 32u8;

        let proof =
            AggregatedRangeProof::generate_with_splitting(&input, upper_bound_bit_length).unwrap();

        let res = proof.verify(&commitment, upper_bound_bit_length);

        assert_err!(
            res,
            Err(RangeProofError::BulletproofVerificationError(
                ProofError::VerificationError
            ))
        );
    }
}
