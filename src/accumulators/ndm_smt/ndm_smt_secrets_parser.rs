//! Parser for files containing NDM-SMT-related secrets.
//!
//! Supported file types: toml
//! Note that the file type is inferred from its path extension.
//!
//! TOML format:
//! ```toml,ignore
//! # None of these values should be shared. They should be kept with the tree
//! # creator.
//!
//! # Used for generating secrets for each entity.
//! master_secret = "master_secret"
//!
//! # Used for generating blinding factors for Pedersen commitments.
//! salt_b = "salt_b"
//!
//! # Used as an input to the hash function when merging nodes.
//! salt_s = "salt_s"
//! ```
//!
//! See [super][secrets] for more details about the secret values.

use log::{info, warn};
use std::{ffi::OsString, fs::File, io::Read, path::PathBuf, str::FromStr};

use super::ndm_smt_secrets::{Secrets, SecretsInput};
use crate::secret::SecretParseError;

/// Parser requires a valid path to a file.
pub struct NdmSmtSecretsParser {
    path: Option<PathBuf>,
}

impl NdmSmtSecretsParser {
    /// Constructor.
    ///
    /// `Option` is used to wrap the parameter to make the code work more
    /// seamlessly with the config builders in [super][super][accumulators].
    pub fn from_path(path: Option<PathBuf>) -> Self {
        NdmSmtSecretsParser { path }
    }

    /// Open and parse the file, returning a [Secrets] struct.
    ///
    /// An error is returned if:
    /// 1. The path is None (i.e. was not set).
    /// 2. The file cannot be opened.
    /// 3. The file cannot be read.
    /// 4. The file type is not supported.
    /// 5. Deserialization of any of the records in the file fails.
    pub fn parse(self) -> Result<Secrets, NdmSmtSecretsParserError> {
        info!(
            "Attempting to parse {:?} as a file containing NDM-SMT secrets",
            &self.path
        );

        let path = self.path.ok_or(NdmSmtSecretsParserError::PathNotSet)?;

        let ext = path.extension().and_then(|s| s.to_str()).ok_or(
            NdmSmtSecretsParserError::UnknownFileType(path.clone().into_os_string()),
        )?;

        let secrets = match FileType::from_str(ext)? {
            FileType::Toml => {
                let mut buf = String::new();
                File::open(path)?.read_to_string(&mut buf)?;
                let secrets: SecretsInput = toml::from_str(&buf)?;
                Secrets::try_from(secrets)?
            }
        };

        Ok(secrets)
    }

    pub fn parse_or_generate_random(self) -> Result<Secrets, NdmSmtSecretsParserError> {
        match &self.path {
            Some(_) => self.parse(),
            None => {
                warn!(
                    "Could not determine path for secrets file, defaulting to randomized secrets"
                );
                Ok(Secrets::generate_random())
            }
        }
    }
}

/// Supported file types for the parser.
enum FileType {
    Toml,
}

impl FromStr for FileType {
    type Err = NdmSmtSecretsParserError;

    fn from_str(ext: &str) -> Result<FileType, Self::Err> {
        match ext {
            "toml" => Ok(FileType::Toml),
            _ => Err(NdmSmtSecretsParserError::UnsupportedFileType { ext: ext.into() }),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum NdmSmtSecretsParserError {
    #[error("Expected path to be set but found none")]
    PathNotSet,
    #[error("Unable to find file extension for path {0:?}")]
    UnknownFileType(OsString),
    #[error("The file type with extension {ext:?} is not supported")]
    UnsupportedFileType { ext: String },
    #[error("Error converting string found in file to Secret")]
    StringConversionError(#[from] SecretParseError),
    #[error("Error reading the file")]
    FileReadError(#[from] std::io::Error),
    #[error("Deserialization process failed")]
    DeserializationError(#[from] toml::de::Error),
}
