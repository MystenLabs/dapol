use std::{fs::File, io::Read, path::PathBuf, str::FromStr};

use serde::Deserialize;
use thiserror::Error;

pub mod ndm_smt;

#[derive(Deserialize, Debug)]
#[serde(tag = "accumulator_type", rename_all = "kebab-case")]
pub enum AccumulatorConfig {
    NdmSmt(ndm_smt::NdmSmtConfig),
    // TODO other accumulators..
}

/// Parser requires a valid path to a file.
pub struct AccumulatorParser {
    config_file_path: Option<PathBuf>,
}

impl AccumulatorParser {
    /// Constructor.
    ///
    /// `Option` is used to wrap the parameter to make the code work more
    /// seamlessly with the config builders in [super][super][accumulators].
    pub fn from_config_fil_path(path: Option<PathBuf>) -> Self {
        AccumulatorParser {
            config_file_path: path,
        }
    }

    /// Open and parse the config file, then try to create an accumulator
    /// object from the config.
    ///
    /// An error is returned if:
    /// 1. The path is None (i.e. was not set).
    /// 2. The file cannot be opened.
    /// 3. The file cannot be read.
    /// 4. The file type is not supported.
    /// 5. Deserialization of any of the records in the file fails.
    pub fn parse(self) -> Result<ndm_smt::NdmSmt, AccumulatorParserError> {
        let config_file_path = self
            .config_file_path
            .ok_or(AccumulatorParserError::PathNotSet)?;

        let ext = config_file_path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or(AccumulatorParserError::UnknownFileType)?;

        let config = match FileType::from_str(ext)? {
            FileType::Toml => {
                let mut buf = String::new();
                File::open(config_file_path)?.read_to_string(&mut buf)?;
                // STENT TODO why unwrap here?
                let config: AccumulatorConfig = toml::from_str(&buf).unwrap();
                config
            }
        };

        let accumulator = match config {
            AccumulatorConfig::NdmSmt(config) => config.parse(),
            // TODO add more accumulators..
        };

        Ok(accumulator)
    }
}

/// Supported file types for the parser.
enum FileType {
    Toml,
}

impl FromStr for FileType {
    type Err = AccumulatorParserError;

    fn from_str(ext: &str) -> Result<FileType, Self::Err> {
        match ext {
            "toml" => Ok(FileType::Toml),
            _ => Err(AccumulatorParserError::UnsupportedFileType { ext: ext.into() }),
        }
    }
}

#[derive(Error, Debug)]
pub enum AccumulatorParserError {
    #[error("Expected path to be set but found none")]
    PathNotSet,
    #[error("Unable to find file extension")]
    UnknownFileType,
    #[error("The file type with extension {ext:?} is not supported")]
    UnsupportedFileType { ext: String },
    // #[error("Error converting string found in file to Secret")]
    // StringConversionError(#[from] SecretParseError),
    #[error("Error reading the file")]
    FileReadError(#[from] std::io::Error),
}
