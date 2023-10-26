//! TODO

use std::path::PathBuf;

use derive_builder::Builder;
use serde::Deserialize;

use crate::binary_tree::Height;
use crate::entity::EntitiesParser;
use crate::read_write_utils::{parse_tree_serialization_path, serialize_to_bin_file};
use crate::utils::{Consume, LogOnErr};

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
                let path = parse_tree_serialization_path(path, FILE_PREFIX).log_on_err().unwrap();

                Some(path)
            }
            None => None,
        };

        let ndmsmt = NdmSmt::new(secrets, height, entities).log_on_err().unwrap();

        serialization_path.consume(|path| {
            serialize_to_bin_file(&ndmsmt, path).log_on_err();
        });

        // STENT TODO log out all the above info

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
