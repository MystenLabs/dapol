//! Examples on how to generate and verify inclusion proofs.

/// An inclusion proof can be generated from only a tree + entity ID.
pub fn simple_inclusion_proof_generation_and_verification(
    ndm_smt: &dapol::NdmSmt,
    entity_id: dapol::EntityId,
) {
    let inclusion_proof = ndm_smt.generate_inclusion_proof(&entity_id).unwrap();
    inclusion_proof.verify(ndm_smt.root_hash()).unwrap();
}

/// The inclusion proof generation algorithm can be customized via some
/// parameters. See [dapol][InclusionProof] for more details.
pub fn advanced_inclusion_proof_generation_and_verification(
    ndm_smt: &dapol::NdmSmt,
    entity_id: dapol::EntityId,
) {
    // Determines how many of the range proofs in the inclusion proof are
    // aggregated together. The ones that are not aggregated are proved
    // individually. The more that are aggregated the faster the proving
    // and verification times.
    let aggregation_percentage = dapol::percentage::ONE_HUNDRED_PERCENT;
    let aggregation_factor = dapol::AggregationFactor::Percent(aggregation_percentage);
    let aggregation_factor = dapol::AggregationFactor::default();

    // 2^upper_bound_bit_length is the upper bound used in the range proof i.e.
    // the secret value is shown to reside in the range [0, 2^upper_bound_bit_length].
    let upper_bound_bit_length = 32u8;
    let upper_bound_bit_length = dapol::DEFAULT_RANGE_PROOF_UPPER_BOUND_BIT_LENGTH;

    let inclusion_proof = ndm_smt
        .generate_inclusion_proof_with(&entity_id, aggregation_factor, upper_bound_bit_length)
        .unwrap();

    inclusion_proof.verify(ndm_smt.root_hash()).unwrap();
}
