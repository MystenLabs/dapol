//! Accumulator config and parser.
//!
//! This is the top-most file in the hierarchy of the dapol crate. An
//! accumulator is required to build a binary tree, and determines how the
//! binary tree is constructed. The are different types of accumulators, which
//! can all be found under this module. Each accumulator has different
//! configuration requirements, which are detailed in each of the sub-modules.
//!
//! Currently only TOML files are supported for config files. The only
//! config requirement at this level (not including the specific accumulator
//! config) is the accumulator type:
//!
//! ```toml,ignore
//! accumulator_type = "ndm-smt"
//! ```
//!
//! The rest of the config details can be found in the submodules:
//! - [ndm_smt][ndm_smt_config]
//!
//! Example how to use the parser:
//! ```
//! use std::path::PathBuf;
//! use dapol::AccumulatorParser;
//!
//! let path = PathBuf::from("./tree_config_example.toml");
//!
//! let accumulator = AccumulatorParser::from_config_fil_path(path)
//!     .parse()
//!     .unwrap();
//! ```

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
