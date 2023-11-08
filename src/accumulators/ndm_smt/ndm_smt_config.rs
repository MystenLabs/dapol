//! Configuration for the NDM-SMT.
//!
//! The config is defined by a struct. A builder pattern is used to construct
//! the config, but it can also be constructed by deserializing a file.
//! Construction is handled by [super][super][AccumulatorConfig] and so have
//! a look there for more details on file format for deserialization or examples
//! on how to use the parser. Currently only toml files are supported, with the
//! following format:
//!
//! ```toml,ignore
//! accumulator_type = "ndm-smt"
//!
//! # Height of the tree.
//! # If the height is not set the default height will be used.
//! height = 32
//!
//! # Path to the secrets file.
//! # If not present the secrets will be generated randomly.
//! secrets_file_path = "./secrets_example.toml"
//!
//! # At least one of file_path & generate_random must be present.
//! # If both are given then file_path is prioritized.
//! [entities]
//!
//! # Path to a file containing a list of entity IDs and their liabilities.
//! file_path = "./entities_example.csv"
//!
//! # Generate the given number of entities, with random IDs & liabilities.
//! generate_random = 4
//! ```
//!
//! Construction of this tree using a config file must be done via
//! [super][super][config][AccumulatorConfig].
//!
//! Example how to use the builder:
//! ```
//! use std::path::PathBuf;
//! use dapol::Height;
//! use dapol::NdmSmtConfigBuilder;
//!
//! let height = Height::from(8);
//!
//! let config = NdmSmtConfigBuilder::default()
//!     .height(height)
//!     .secrets_file_path(PathBuf::from("./secrets_example.toml"))
//!     .entities_path(PathBuf::from("./entities_example.csv"))
//!     .build()
//!     .unwrap();
//! ```

use std::path::PathBuf;

use derive_builder::Builder;
use log::{debug, info};
use serde::Deserialize;

use crate::binary_tree::Height;
use crate::entity::{self, EntitiesParser};
use crate::utils::{IfNoneThen, LogOnErr};

use super::{ndm_smt_secrets_parser, NdmSmt, SecretsParser};

#[derive(Deserialize, Debug, Builder)]
pub struct NdmSmtConfig {
    #[builder(setter(name = "height_opt"))]
    height: Option<Height>,
    #[builder(setter(name = "secrets_file_path_opt"))]
    secrets_file_path: Option<PathBuf>,
    #[builder(private)]
    entities: EntityConfig,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct EntityConfig {
    file_path: Option<PathBuf>,
    generate_random: Option<u64>,
}

impl NdmSmtConfig {
    /// Try to construct an NDM-SMT from the config.
    pub fn parse(self) -> Result<NdmSmt, NdmSmtParserError> {
        debug!("Parsing config to create a new NDM-SMT");

        let secrets =
            SecretsParser::from_path(self.secrets_file_path).parse_or_generate_random()?;

        let height = self
            .height
            .if_none_then(|| {
                info!("No height set, defaulting to {:?}", Height::default());
            })
            .unwrap_or_default();

        let entities = EntitiesParser::new()
            .with_path(self.entities.file_path)
            .with_num_entities(self.entities.generate_random)
            .parse_or_generate_random()?;

        let ndm_smt = NdmSmt::new(secrets, height, entities).log_on_err()?;

        debug!(
            "Successfully built NDM-SMT with root hash {:?}",
            ndm_smt.root_hash()
        );

        Ok(ndm_smt)
    }
}

impl NdmSmtConfigBuilder {
    pub fn height(&mut self, height: Height) -> &mut Self {
        self.height_opt(Some(height))
    }

    pub fn secrets_file_path(&mut self, path: PathBuf) -> &mut Self {
        self.secrets_file_path_opt(Some(path))
    }

    pub fn entities_path_opt(&mut self, path: Option<PathBuf>) -> &mut Self {
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

    pub fn entities_path(&mut self, path: PathBuf) -> &mut Self {
        self.entities_path_opt(Some(path))
    }

    pub fn num_entities_opt(&mut self, num_entites: Option<u64>) -> &mut Self {
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

    pub fn num_entities(&mut self, num_entites: u64) -> &mut Self {
        self.num_entities_opt(Some(num_entites))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum NdmSmtParserError {
    #[error("Secrets parsing failed while trying to parse NDM-SMT config")]
    SecretsError(#[from] ndm_smt_secrets_parser::SecretsParserError),
    #[error("Entities parsing failed while trying to parse NDM-SMT config")]
    EntitiesError(#[from] entity::EntitiesParserError),
    #[error("Tree construction failed after parsing NDM-SMT config")]
    BuildError(#[from] super::NdmSmtError),
}
