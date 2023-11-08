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

use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    read_write_utils::{self, ReadWriteError},
    utils::LogOnErr,
    AggregationFactor, EntityId, InclusionProof, NdmSmtError,
};

pub mod config;
pub mod ndm_smt;

const SERIALIZED_ACCUMULATOR_EXTENSION: &str = "dapoltree";
const SERIALIZED_ACCUMULATOR_FILE_PREFIX: &str = "accumulator_";

/// Various accumulator types.
#[derive(Serialize, Deserialize)]
pub enum Accumulator {
    NdmSmt(ndm_smt::NdmSmt),
    // TODO other accumulators..
}

// STENT TODO change name here, accumulator is not super intuitive
impl Accumulator {
    /// Try deserialize an accumulator from the given file path.
    ///
    /// The file is assumed to be in [bincode] format.
    ///
    /// An error is logged and returned if
    /// 1. The file cannot be opened.
    /// 2. The [bincode] deserializer fails.
    pub fn deserialize(path: PathBuf) -> Result<Accumulator, AccumulatorError> {
        debug!(
            "Deserializing accumulator from file {:?}",
            path.clone().into_os_string()
        );

        match path.extension() {
            Some(ext) => {
                if ext != SERIALIZED_ACCUMULATOR_EXTENSION {
                    Err(ReadWriteError::UnsupportedFileExtension {
                        expected: SERIALIZED_ACCUMULATOR_EXTENSION.to_owned(),
                        actual: ext.to_os_string(),
                    })?;
                }
            }
            None => Err(ReadWriteError::NotAFile(path.clone().into_os_string()))?,
        }

        let accumulator: Accumulator =
            read_write_utils::deserialize_from_bin_file(path.clone()).log_on_err()?;

        let root_hash = match &accumulator {
            Accumulator::NdmSmt(ndm_smt) => ndm_smt.root_hash(),
        };

        info!(
            "Successfully deserialized accumulator from file {:?} with root hash {:?}",
            path.clone().into_os_string(),
            root_hash
        );

        Ok(accumulator)
    }

    /// Parse `path` as one that points to a serialized dapol tree file.
    ///
    /// `path` can be either of the following:
    /// 1. Existing directory: in this case a default file name is appended to `path`.
    /// 2. Non-existing directory: in this case all dirs in the path are created,
    /// and a default file name is appended.
    /// 3. File in existing dir: in this case the extension is checked to be
    /// [SERIALIZED_ACCUMULATOR_EXTENSION], then `path` is returned.
    /// 4. File in non-existing dir: dirs in the path are created and the file
    /// extension is checked.
    ///
    /// The file prefix is [SERIALIZED_ACCUMULATOR_FILE_PREFIX].
    pub fn parse_accumulator_serialization_path(
        path: PathBuf,
    ) -> Result<PathBuf, ReadWriteError> {
        read_write_utils::parse_serialization_path(
            path,
            SERIALIZED_ACCUMULATOR_EXTENSION,
            SERIALIZED_ACCUMULATOR_FILE_PREFIX,
        )
    }

    /// Serialize to a file.
    ///
    /// Serialization is done using [bincode]
    ///
    /// An error is returned if
    /// 1. [bincode] fails to serialize the file.
    /// 2. There is an issue opening or writing the file.
    pub fn serialize(&self, path: PathBuf) -> Result<(), AccumulatorError> {
        info!(
            "Serializing accumulator to file {:?}",
            path.clone().into_os_string()
        );

        read_write_utils::serialize_to_bin_file(self, path).log_on_err()?;
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
    #[error("Error serializing/deserializing file")]
    SerdeError(#[from] ReadWriteError),
}
