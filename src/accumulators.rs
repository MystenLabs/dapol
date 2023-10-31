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
use std::{fs::File, io::Read, path::PathBuf, str::FromStr};

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
        use crate::utils::LogOnErr;

        let accumulator: Accumulator = deserialize_from_bin_file(path).log_on_err()?;
        Ok(accumulator)
    }
}

/// Configuration for the various accumulator types.
///
/// Currently only TOML files are supported for config files. The only
/// config requirement at this level (not including the specific accumulator
/// config) is the accumulator type:
///
/// ```toml,ignore
/// accumulator_type = "ndm-smt"
/// ```
///
/// The rest of the config details can be found in the submodules:
/// - [ndm_smt][ndm_smt_config]
#[derive(Deserialize, Debug)]
#[serde(tag = "accumulator_type", rename_all = "kebab-case")]
// STENT TODO move to own file
pub enum AccumulatorConfig {
    NdmSmt(ndm_smt::NdmSmtConfig),
    // TODO other accumulators..
}

// STENT TODO rename all other builder methods that are 'new' to 'default' since this is what derive_default uses
// STENT TODO also maybe get rid of the 'with' in the setters
// STENT TODO move this to its own file

impl AccumulatorConfig {
    /// Open the config file, then try to create an accumulator object.
    ///
    /// An error is returned if:
    /// 1. The file cannot be opened.
    /// 2. The file cannot be read.
    /// 3. The file type is not supported.
    pub fn deserialize(config_file_path: PathBuf) -> Result<Self, AccumulatorConfigError> {
        let ext = config_file_path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or(AccumulatorConfigError::UnknownFileType)?;

        let config = match FileType::from_str(ext)? {
            FileType::Toml => {
                let mut buf = String::new();
                File::open(config_file_path)?.read_to_string(&mut buf)?;
                let config: AccumulatorConfig = toml::from_str(&buf)?;
                config
            }
        };

        Ok(config)
    }

    /// Parse the config, attempting to create an accumulator object.
    ///
    /// An error is returned if:
    /// 1. TODO need to change the ndm-smt parse function to return an error first
    pub fn parse(self) -> Result<Accumulator, AccumulatorError> {
        let accumulator = match self {
            AccumulatorConfig::NdmSmt(config) => Accumulator::NdmSmt(config.parse()),
            // TODO add more accumulators..
        };

        Ok(accumulator)
    }
}

/// Supported file types for deserialization.
enum FileType {
    Toml,
}

impl FromStr for FileType {
    type Err = AccumulatorConfigError;

    fn from_str(ext: &str) -> Result<FileType, Self::Err> {
        match ext {
            "toml" => Ok(FileType::Toml),
            _ => Err(AccumulatorConfigError::UnsupportedFileType { ext: ext.into() }),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AccumulatorError {
    #[error("Error deserializing file")]
    DeserializationError(#[from] crate::read_write_utils::ReadWriteError),
}

#[derive(thiserror::Error, Debug)]
pub enum AccumulatorConfigError {
    #[error("Unable to find file extension")]
    // STENT TODO add path variable here to help user diagnose
    UnknownFileType,
    #[error("The file type with extension {ext:?} is not supported")]
    UnsupportedFileType { ext: String },
    #[error("Error reading the file")]
    FileReadError(#[from] std::io::Error),
    #[error("Deserialization process failed")]
    DeserializationError(#[from] toml::de::Error),
}
