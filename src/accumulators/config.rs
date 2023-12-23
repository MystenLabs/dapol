use log::debug;
use serde::Deserialize;
use std::{ffi::OsString, fs::File, io::Read, path::PathBuf, str::FromStr};

use super::{ndm_smt, Accumulator};

/// Configuration required for building various accumulator types.
///
/// Currently only TOML files are supported for config files. The only
/// config requirement at this level (not including the specific accumulator
/// config) is the accumulator type:
///
/// ```toml,ignore
/// accumulator_type = "ndm-smt"
/// ```
///
/// The rest of the config details can be found in the sub-modules:
/// - [crate][accumulators][NdmSmtConfig]
///
/// Config deserialization example:
/// ```
/// use std::path::PathBuf;
/// use dapol::AccumulatorConfig;
///
/// let file_path = PathBuf::from("./examples/tree_config_example.toml");
/// let config = AccumulatorConfig::deserialize(file_path).unwrap();
/// ```
#[derive(Deserialize, Debug)]
#[serde(tag = "accumulator_type", rename_all = "kebab-case")]
pub enum AccumulatorConfig {
    NdmSmt(ndm_smt::NdmSmtConfig),
    // TODO other accumulators..
}

// STENT TODO rename all other builder methods that are 'new' to 'default' since
// this is what derive_default uses STENT TODO also maybe get rid of the 'with'
// in the setters

impl AccumulatorConfig {
    /// Open the config file, then try to create an accumulator object.
    ///
    /// An error is returned if:
    /// 1. The file cannot be opened.
    /// 2. The file cannot be read.
    /// 3. The file type is not supported.
    pub fn deserialize(config_file_path: PathBuf) -> Result<Self, AccumulatorConfigError> {
        debug!(
            "Attempting to parse {:?} as a file containing accumulator config",
            config_file_path.clone().into_os_string()
        );

        let ext = config_file_path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or(AccumulatorConfigError::UnknownFileType(
                config_file_path.clone().into_os_string(),
            ))?;

        let config = match FileType::from_str(ext)? {
            FileType::Toml => {
                let mut buf = String::new();
                File::open(config_file_path)?.read_to_string(&mut buf)?;
                let config: AccumulatorConfig = toml::from_str(&buf)?;
                config
            }
        };

        debug!("Successfully parsed accumulator config file");

        Ok(config)
    }

    /// Parse the config, attempting to create an accumulator object.
    ///
    /// An error is returned if the parser for the specific accumulator type
    /// fails.
    pub fn parse(self) -> Result<Accumulator, AccumulatorParserError> {
        let accumulator = match self {
            AccumulatorConfig::NdmSmt(config) => Accumulator::NdmSmt(config.parse()?),
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

/// Errors encountered when handling [AccumulatorConfig].
#[derive(thiserror::Error, Debug)]
pub enum AccumulatorConfigError {
    #[error("Unable to find file extension for path {0:?}")]
    UnknownFileType(OsString),
    #[error("The file type with extension {ext:?} is not supported")]
    UnsupportedFileType { ext: String },
    #[error("Error reading the file")]
    FileReadError(#[from] std::io::Error),
    #[error("Deserialization process failed")]
    DeserializationError(#[from] toml::de::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum AccumulatorParserError {
    #[error("Error parsing NDM-SMT config")]
    NdmSmtError(#[from] ndm_smt::NdmSmtConfigParserError),
}
