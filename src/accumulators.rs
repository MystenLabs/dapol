use derive_builder::Builder;
use serde::Deserialize;
use std::path::PathBuf;

use crate::binary_tree::Height;
use crate::entity::{EntitiesParser, EntityId};
use crate::inclusion_proof::InclusionProof;
use crate::read_write_utils::{parse_tree_serialization_path, serialize_to_bin_file, LogOnErr};

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

        assert!(serialization_path
            .map(|path| serialize_to_bin_file(&ndmsmt, path).log_on_err().err())
            .is_none());

        // STENT TODO log out all the above info

        ndmsmt
    }
}

impl AccumulatorConfig {
    // STENT TODO
    // pub fn parse<A: Accumulator>(self) -> A {
    pub fn parse(self) {
        match self {
            // STENT TODO proper error handling, no unwraps
            Self::NdmSmt(config) => {
                let a = config.parse();
            }
        }
    }
}
