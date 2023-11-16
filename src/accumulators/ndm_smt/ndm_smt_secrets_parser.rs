use log::{info, warn};
use std::{ffi::OsString, fs::File, io::Read, path::PathBuf, str::FromStr};

use super::ndm_smt_secrets::{NdmSmtSecrets, NdmSmtSecretsInput};
use crate::secret::SecretParserError;

/// Parser for files containing NDM-SMT-related secrets.
///
/// Supported file types: toml
/// Note that the file type is inferred from its path extension.
///
/// TOML format:
/// ```toml,ignore
/// # None of these values should be shared. They should be kept with the tree
/// # creator.
///
/// # Used for generating secrets for each entity.
/// master_secret = "master_secret"
///
/// # Used for generating blinding factors for Pedersen commitments.
/// salt_b = "salt_b"
///
/// # Used as an input to the hash function when merging nodes.
/// salt_s = "salt_s"
/// ```
///
/// See [crate][accumulators][NdmSmtSecrets] for more details about the
/// secret values.
pub struct NdmSmtSecretsParser {
    path: Option<PathBuf>,
}

impl NdmSmtSecretsParser {
    /// Constructor.
    ///
    /// `Option` is used to wrap the parameter to make the code work more
    /// seamlessly with the config builders in [crate][accumulators].
    pub fn from_path_opt(path: Option<PathBuf>) -> Self {
        NdmSmtSecretsParser { path }
    }

    pub fn from_path(path: PathBuf) -> Self {
        NdmSmtSecretsParser { path: Some(path) }
    }

    /// Open and parse the file, returning a [NdmSmtSecrets] struct.
    ///
    /// An error is returned if:
    /// 1. The path is None (i.e. was not set).
    /// 2. The file cannot be opened.
    /// 3. The file cannot be read.
    /// 4. The file type is not supported.
    /// 5. Deserialization of any of the records in the file fails.
    pub fn parse(self) -> Result<NdmSmtSecrets, NdmSmtSecretsParserError> {
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
                let secrets: NdmSmtSecretsInput = toml::from_str(&buf)?;
                NdmSmtSecrets::try_from(secrets)?
            }
        };

        Ok(secrets)
    }

    pub fn parse_or_generate_random(self) -> Result<NdmSmtSecrets, NdmSmtSecretsParserError> {
        match &self.path {
            Some(_) => self.parse(),
            None => {
                warn!(
                    "Could not determine path for secrets file, defaulting to randomized secrets"
                );
                Ok(NdmSmtSecrets::generate_random())
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
    StringConversionError(#[from] SecretParserError),
    #[error("Error reading the file")]
    FileReadError(#[from] std::io::Error),
    #[error("Deserialization process failed")]
    DeserializationError(#[from] toml::de::Error),
}

// -------------------------------------------------------------------------------------------------
// Unit tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::assert_err;
    use crate::Secret;
    use std::path::Path;

    #[test]
    fn parser_toml_file_happy_case() {
        let src_dir = env!("CARGO_MANIFEST_DIR");
        let resources_dir = Path::new(&src_dir).join("examples");
        let path = resources_dir.join("ndm_smt_secrets_example.toml");

        let secrets = NdmSmtSecretsParser::from_path(path).parse().unwrap();

        assert_eq!(
            secrets.master_secret,
            Secret::from_str("master_secret").unwrap()
        );
        assert_eq!(secrets.salt_b, Secret::from_str("salt_b").unwrap());
        assert_eq!(secrets.salt_s, Secret::from_str("salt_s").unwrap());
    }

    #[test]
    fn unsupported_file_type() {
        let this_file = std::file!();
        let path = PathBuf::from(this_file);

        assert_err!(
            NdmSmtSecretsParser::from_path(path).parse(),
            Err(NdmSmtSecretsParserError::UnsupportedFileType { ext: _ })
        );
    }

    #[test]
    fn unknown_file_type() {
        let path = PathBuf::from("./");
        assert_err!(
            NdmSmtSecretsParser::from_path(path).parse(),
            Err(NdmSmtSecretsParserError::UnknownFileType(_))
        );
    }
}
