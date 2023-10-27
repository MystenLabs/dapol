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
//! # Height of the tree.
//! # If the height is not set the default height will be used.
//! height = 32
//!
//! # Path to the secrets file.
//! # If not present the secrets will be generated randomly.
//! secrets_file_path = "./secrets_example.toml"
//!
//! # Can be a file or directory (default file name given in this case)
//! # If not present then no serialization is done.
//! serialization_path = "./tree.dapoltree"
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
//! Example how to use the builder:
//! ```
//! use std::path::PathBuf;
//! use crate::binary_tree::Height;
//! use crate::accumulators::ndm_smt;
//!
//! let height = Height::default();
//!
//! let config = ndm_smt::NdmSmtConfigBuilder::default()
//!     .height(Some(height))
//!     .secrets_file_path(PathBuf::from("./secrets.toml"))
//!     .serialization_path(PathBuf::from("./ndm_smt.dapoltree"))
//!     .entities_path(PathBuf::from("./entities.csv"))
//!     .build()
//!     .unwrap();
//! ```

use std::path::PathBuf;

use derive_builder::Builder;
use log::info;
use serde::Deserialize;

use crate::binary_tree::Height;
use crate::entity::EntitiesParser;
use crate::read_write_utils::{parse_tree_serialization_path, serialize_to_bin_file};
use crate::utils::{Consume, IfNoneThen, LogOnErr};

use super::{NdmSmt, SecretsParser};

const FILE_PREFIX: &str = "ndm_smt_";

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

impl NdmSmtConfig {
    pub fn parse(self) -> NdmSmt {
        let secrets = SecretsParser::from_path(self.secrets_file_path)
            .parse_or_generate_random()
            .unwrap();

        let height = self
            .height
            .if_none_then(|| {
                info!("No height set, defaulting to {:?}", Height::default());
            })
            .unwrap_or_default();

        let entities = EntitiesParser::new()
            .with_path(self.entities.file_path)
            .with_num_entities(self.entities.generate_random)
            .parse_or_generate_random()
            .unwrap();

        // Do path checks before building so that the build does not have to be
        // repeated for problems with file names etc.
        let serialization_path = match self.serialization_path.clone() {
            Some(path) => {
                let path = parse_tree_serialization_path(path, FILE_PREFIX)
                    .log_on_err()
                    .unwrap();

                Some(path)
            }
            None => None,
        };

        let ndmsmt = NdmSmt::new(secrets, height, entities).log_on_err().unwrap();

        serialization_path
            .if_none_then(|| {
                info!("No serialization path set, skipping serialization of the tree");
            })
            .consume(|path| {
                serialize_to_bin_file(&ndmsmt, path).log_on_err();
            });

        ndmsmt
    }
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
