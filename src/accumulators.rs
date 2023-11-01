//! Accumulators.
//!
//! This is the top-most file in the hierarchy of the dapol crate. An
//! accumulator is required to build a binary tree, and determines how the
//! binary tree is constructed. The are different types of accumulators, which
//! can all be found under this module. Each accumulator has different
//! configuration requirements, which are detailed in each of the sub-modules.
//! The currently supported accumulator types are:
//! - [Non-Deterministic Mapping Sparse Merkle Tree]
//!
//! Each accumulator can be constructed via the configuration structs:
//! - [config][AccumulatorConfig] is used to deserialize config from a file. The
//! specific type of accumulator is determined from the config file.
//! - [ndm_smt][ndm_smt_config][NdmSmtConfigBuilder] is used to construct the
//! NDM-SMT accumulator type using the builder pattern.
//!
//! [Non-Deterministic Mapping Sparse Merkle Tree]: ndm_smt

use log::info;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{ndm_smt::NdmSmtError, utils::LogOnErr, AggregationFactor, EntityId, InclusionProof};

pub mod config;
pub mod ndm_smt;

/// Various accumulator types.
#[derive(Serialize, Deserialize)]
pub enum Accumulator {
    NdmSmt(ndm_smt::NdmSmt),
    // TODO other accumulators..
}

impl Accumulator {
    /// Try deserialize an accumulator from the given file path.
    ///
    /// The file is assumed to be in [bincode] format.
    ///
    /// An error is logged and returned if
    /// 1. The file cannot be opened.
    /// 2. The [bincode] deserializer fails.
    pub fn deserialize(path: PathBuf) -> Result<Accumulator, AccumulatorError> {
        use crate::read_write_utils::deserialize_from_bin_file;

        let accumulator: Accumulator = deserialize_from_bin_file(path).log_on_err()?;
        Ok(accumulator)
    }

    /// Serialize to a file.
    ///
    /// Serialization is done using [bincode]
    ///
    /// An error is returned if
    /// 1. [bincode] fails to serialize the file.
    /// 2. There is an issue opening or writing the file.
    pub fn serialize(&self, path: PathBuf) -> Result<(), AccumulatorError> {
        use crate::read_write_utils::serialize_to_bin_file;

        info!(
            "Serializing accumulator to file {:?}",
            path.clone().into_os_string()
        );

        serialize_to_bin_file(self, path).log_on_err()?;
        Ok(())
    }

    /// Generate an inclusion proof for the given `entity_id`.
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
    pub fn generate_inclusion_proof_with(
        &self,
        entity_id: &EntityId,
        aggregation_factor: AggregationFactor,
        upper_bound_bit_length: u8,
    ) -> Result<InclusionProof, NdmSmtError> {
        match self {
            Accumulator::NdmSmt(ndm_smt) => ndm_smt.generate_inclusion_proof_with(
                entity_id,
                aggregation_factor,
                upper_bound_bit_length,
            ),
        }
    }

    /// Generate an inclusion proof for the given `entity_id`.
    pub fn generate_inclusion_proof(
        &self,
        entity_id: &EntityId,
    ) -> Result<InclusionProof, NdmSmtError> {
        match self {
            Accumulator::NdmSmt(ndm_smt) => ndm_smt.generate_inclusion_proof(entity_id),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AccumulatorError {
    #[error("Error deserializing file")]
    DeserializationError(#[from] crate::read_write_utils::ReadWriteError),
}
