use std::{convert::TryFrom, fs::File, io::Read, path::PathBuf, str::FromStr};

use derive_builder::Builder;
use log::warn;
use serde::Deserialize;
use thiserror::Error;

use crate::binary_tree::Height;
use crate::entity::EntitiesParser;
use crate::read_write_utils::{parse_tree_serialization_path, serialize_to_bin_file};
use crate::utils::LogOnErr;

pub mod ndm_smt;

#[derive(Deserialize, Debug)]
#[serde(tag = "accumulator_type", rename_all = "kebab-case")]
pub enum AccumulatorConfig {
    NdmSmt(NdmSmtConfig),
    // TODO other accumulators..
}

// STENT TODO this should live in the ndm_smt dir
#[derive(Deserialize, Debug, Builder)]
pub struct NdmSmtConfig {
    height: Option<Height>,
    secrets_file_path: Option<PathBuf>,
    serialization_path: Option<PathBuf>,
    entities: EntityConfig,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct EntityConfig {
    file_path: Option<PathBuf>,
    generate_random: Option<u64>,
}

impl NdmSmtConfigBuilder {
    pub fn entities_path(&mut self, path: Option<PathBuf>) -> &mut Self {
        match &mut self.entities {
            None => {
                self.entities = Some(EntityConfig {
                    file_path: path,
                    generate_random: None,
                })
            }
            Some(entities) => entities.file_path = path,
        }
        self
    }

    pub fn num_entities(&mut self, num_entites: Option<u64>) -> &mut Self {
        match &mut self.entities {
            None => {
                self.entities = Some(EntityConfig {
                    file_path: None,
                    generate_random: num_entites,
                })
            }
            Some(entities) => entities.generate_random = num_entites,
        }
        self
    }
}

impl NdmSmtConfig {
    pub fn parse(self) -> ndm_smt::NdmSmt {
        let secrets = ndm_smt::SecretsParser::from_path(self.secrets_file_path)
            .parse_or_generate_random()
            .unwrap();

        let height = self.height.unwrap_or_default();

        let entities = EntitiesParser::new()
            .with_path(self.entities.file_path)
            .with_num_entities(self.entities.generate_random)
            .parse_or_generate_random()
            .unwrap();

        // Do path checks before building so that the build does not have to be
        // repeated for problems with file names etc.
        let serialization_path = match self.serialization_path.clone() {
            Some(path) => {
                let path = parse_tree_serialization_path(path).log_on_err().unwrap();

                Some(path)
            }
            None => None,
        };

        let ndmsmt = ndm_smt::NdmSmt::new(secrets, height, entities)
            .log_on_err()
            .unwrap();

        // STENT TODO make this consume rather than map
        serialization_path.map(|path| serialize_to_bin_file(&ndmsmt, path).log_on_err().err());

        // STENT TODO log out all the above info

        ndmsmt
    }
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
