use std::{convert::TryFrom, fs::File, io::Read, path::PathBuf, str::FromStr};

use log::warn;
use thiserror::Error;

use super::secrets::{Secrets, SecretsInput};
use crate::secret::SecretParseError;

pub struct SecretsParser {
    path: Option<PathBuf>,
}

impl SecretsParser {
    // STENT TODO is the name misleading here because the parameter is optional?
    pub fn from_path(path: Option<PathBuf>) -> Self {
        SecretsParser { path }
    }

    /// Open and parse the file, returning a [Secrets] struct.
    ///
    /// An error is returned if:
    /// 1. The path is None (i.e. was not set).
    /// 2. The file cannot be opened.
    /// 3. The file cannot be read.
    /// 4. The file type is not supported.
    /// 5. Deserialization of any of the records in the file fails.
    pub fn parse(self) -> Result<Secrets, SecretsParseError> {
        let path = self.path.ok_or(SecretsParseError::PathNotSet)?;

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or(SecretsParseError::UnknownFileType)?;

        let secrets = match FileType::from_str(ext)? {
            FileType::Toml => {
                let mut buf = String::new();
                File::open(path)?.read_to_string(&mut buf)?;
                let secrets: SecretsInput = toml::from_str(&buf).unwrap();
                Secrets::try_from(secrets)?
            }
        };

        Ok(secrets)
    }

    pub fn parse_or_generate_random(self) -> Result<Secrets, SecretsParseError> {
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
    type Err = SecretsParseError;

    fn from_str(ext: &str) -> Result<FileType, Self::Err> {
        match ext {
            "toml" => Ok(FileType::Toml),
            _ => Err(SecretsParseError::UnsupportedFileType { ext: ext.into() }),
        }
    }
}

#[derive(Error, Debug)]
pub enum SecretsParseError {
    #[error("Expected path to be set but found none")]
    PathNotSet,
    #[error("Unable to find file extension")]
    UnknownFileType,
    #[error("The file type with extension {ext:?} is not supported")]
    UnsupportedFileType { ext: String },
    #[error("Error converting string found in file to Secret")]
    StringConversionError(#[from] SecretParseError),
    #[error("Error reading the file")]
    FileReadError(#[from] std::io::Error),
}
