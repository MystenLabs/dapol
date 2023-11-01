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

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::utils::LogOnErr;

pub mod config;
pub mod ndm_smt;

/// Various accumulator types.
#[derive(Serialize, Deserialize)]
// STENT TODO create serialization function in the impl
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

    pub fn serialize(&self, path: PathBuf) -> Result<(), AccumulatorError>{
        use crate::read_write_utils::serialize_to_bin_file;

        serialize_to_bin_file(self, path).log_on_err()?;
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AccumulatorError {
    #[error("Error deserializing file")]
    DeserializationError(#[from] crate::read_write_utils::ReadWriteError),
}
