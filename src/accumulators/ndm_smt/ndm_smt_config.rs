use std::path::PathBuf;

use derive_builder::Builder;
use log::{debug, info};
use serde::Deserialize;

use crate::entity::{self, EntitiesParser};
use crate::utils::LogOnErr;
use crate::Height;
use crate::MaxThreadCount;

use super::{ndm_smt_secrets_parser, NdmSmt, NdmSmtSecretsParser};

/// Configuration needed to construct an NDM-SMT.
///
/// The config is defined by a struct. A builder pattern is used to construct
/// the config, but it can also be constructed by deserializing a file.
/// Construction is handled by [crate][AccumulatorConfig] and so have
/// a look there for more details on file format for deserialization or examples
/// on how to use the parser. Currently only toml files are supported, with the
/// following format:
///
/// ```toml,ignore
/// accumulator_type = "ndm-smt"
///
/// # Height of the tree.
/// # If the height is not set the default height will be used.
/// height = 32
///
/// # Max number of threads to be spawned for multi-threading algorithms.
/// # If the height is not set a default value will be used.
/// max_thread_count = 4
///
/// # Path to the secrets file.
/// # If not present the secrets will be generated randomly.
/// secrets_file_path = "./examples/ndm_smt_secrets_example.toml"
///
/// # At least one of file_path & generate_random must be present.
/// # If both are given then file_path is prioritized.
/// [entities]
///
/// # Path to a file containing a list of entity IDs and their liabilities.
/// file_path = "./examples/entities_example.csv"
///
/// # Generate the given number of entities, with random IDs & liabilities.
/// generate_random = 4
/// ```
///
/// Construction of this tree using a config file must be done via
/// [crate][AccumulatorConfig].
///
/// Example how to use the builder:
/// ```
/// use std::path::PathBuf;
/// use dapol::{Height, MaxThreadCount};
/// use dapol::accumulators::NdmSmtConfigBuilder;
///
/// let height = Height::expect_from(8);
/// let max_thread_count = MaxThreadCount::default();
///
/// let config = NdmSmtConfigBuilder::default()
///     .height(height)
///     .secrets_file_path(PathBuf::from("./examples/ndm_smt_secrets_example.toml"))
///     .entities_path(PathBuf::from("./examples/entities_example.csv"))
///     .build();
/// ```
#[derive(Deserialize, Debug, Builder)]
#[builder(build_fn(skip))]
pub struct NdmSmtConfig {
    height: Height,
    max_thread_count: MaxThreadCount,
    #[builder(setter(strip_option))]
    secrets_file_path: Option<PathBuf>,
    #[builder(private)]
    entities: EntityConfig,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct EntityConfig {
    file_path: Option<PathBuf>,
    num_random_entities: Option<u64>,
}

impl NdmSmtConfig {
    /// Try to construct an NDM-SMT from the config.
    pub fn parse(self) -> Result<NdmSmt, NdmSmtConfigParserError> {
        debug!("Parsing config to create a new NDM-SMT: {:?}", self);

        let secrets = NdmSmtSecretsParser::from(self.secrets_file_path)
            .parse_or_generate_random()?;

        let height = self.height;
        let max_thread_count = self.max_thread_count;

        let entities = EntitiesParser::new()
            .with_path_opt(self.entities.file_path)
            .with_num_entities_opt(self.entities.num_random_entities)
            .parse_file_or_generate_random()?;

        let ndm_smt = NdmSmt::new(secrets, height, max_thread_count, entities).log_on_err()?;

        info!(
            "Successfully built NDM-SMT with root hash {:?}",
            ndm_smt.root_hash()
        );

        Ok(ndm_smt)
    }
}

impl NdmSmtConfigBuilder {
    pub fn secrets_file_path_opt(&mut self, path: Option<PathBuf>) -> &mut Self {
        self.secrets_file_path = Some(path);
        self
    }

    pub fn entities_path_opt(&mut self, path: Option<PathBuf>) -> &mut Self {
        match &mut self.entities {
            None => {
                self.entities = Some(EntityConfig {
                    file_path: path,
                    num_random_entities: None,
                })
            }
            Some(entities) => entities.file_path = path,
        }
        self
    }

    pub fn entities_path(&mut self, path: PathBuf) -> &mut Self {
        self.entities_path_opt(Some(path))
    }

    pub fn num_random_entities_opt(&mut self, num_entities: Option<u64>) -> &mut Self {
        match &mut self.entities {
            None => {
                self.entities = Some(EntityConfig {
                    file_path: None,
                    num_random_entities: num_entities,
                })
            }
            Some(entities) => entities.num_random_entities = num_entities,
        }
        self
    }

    pub fn num_random_entities(&mut self, num_entities: u64) -> &mut Self {
        self.num_random_entities_opt(Some(num_entities))
    }

    pub fn build(&self) -> NdmSmtConfig {
        let entities = EntityConfig {
            file_path: self.entities.clone().and_then(|e| e.file_path).or(None),
            num_random_entities: self
                .entities
                .clone()
                .and_then(|e| e.num_random_entities)
                .or(None),
        };

        NdmSmtConfig {
            height: self.height.clone().unwrap_or_default(),
            max_thread_count: self.max_thread_count.clone().unwrap_or_default(),
            secrets_file_path: self.secrets_file_path.clone().unwrap_or(None),
            entities,
        }
    }
}

/// Errors encountered when parsing [crate][accumulators][NdmSmtConfig].
#[derive(thiserror::Error, Debug)]
pub enum NdmSmtConfigParserError {
    #[error("Secrets parsing failed while trying to parse NDM-SMT config")]
    SecretsError(#[from] ndm_smt_secrets_parser::NdmSmtSecretsParserError),
    #[error("Entities parsing failed while trying to parse NDM-SMT config")]
    EntitiesError(#[from] entity::EntitiesParserError),
    #[error("Tree construction failed after parsing NDM-SMT config")]
    BuildError(#[from] super::NdmSmtError),
}

// -------------------------------------------------------------------------------------------------
// Unit tests

#[cfg(test)]
mod tests {
    use crate::utils::test_utils::assert_err;

    use super::*;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    #[test]
    fn builder_with_entities_file() {
        let height = Height::expect_from(8);

        let src_dir = env!("CARGO_MANIFEST_DIR");
        let resources_dir = Path::new(&src_dir).join("examples");
        let secrets_file_path = resources_dir.join("ndm_smt_secrets_example.toml");
        let entities_file_path = resources_dir.join("entities_example.csv");

        let entities_file = File::open(entities_file_path.clone()).unwrap();
        // "-1" because we don't include the top line of the csv which defines
        // the column headings.
        let num_entities = BufReader::new(entities_file).lines().count() - 1;

        let ndm_smt = NdmSmtConfigBuilder::default()
            .height(height.clone())
            .secrets_file_path(secrets_file_path)
            .entities_path(entities_file_path)
            .build()
            .parse()
            .unwrap();

        assert_eq!(ndm_smt.entity_mapping.len(), num_entities);
        assert_eq!(ndm_smt.height(), &height);
    }

    #[test]
    fn builder_with_random_entities() {
        let height = Height::expect_from(8);
        let num_random_entities = 10;

        let src_dir = env!("CARGO_MANIFEST_DIR");
        let resources_dir = Path::new(&src_dir).join("examples");
        let secrets_file = resources_dir.join("ndm_smt_secrets_example.toml");

        let ndm_smt = NdmSmtConfigBuilder::default()
            .height(height.clone())
            .secrets_file_path(secrets_file)
            .num_random_entities(num_random_entities)
            .build()
            .parse()
            .unwrap();

        assert_eq!(ndm_smt.entity_mapping.len(), num_random_entities as usize);
        assert_eq!(ndm_smt.height(), &height);
    }

    #[test]
    fn builder_without_height_should_give_default() {
        let num_random_entities = 10;

        let src_dir = env!("CARGO_MANIFEST_DIR");
        let resources_dir = Path::new(&src_dir).join("examples");
        let secrets_file = resources_dir.join("ndm_smt_secrets_example.toml");

        let ndm_smt = NdmSmtConfigBuilder::default()
            .secrets_file_path(secrets_file)
            .num_random_entities(num_random_entities)
            .build()
            .parse()
            .unwrap();

        assert_eq!(ndm_smt.entity_mapping.len(), num_random_entities as usize);
        assert_eq!(ndm_smt.height(), &Height::default());
    }

    #[test]
    fn builder_without_any_values_fails() {
        use crate::entity::EntitiesParserError;
        let res = NdmSmtConfigBuilder::default().build().parse();
        assert_err!(
            res,
            Err(NdmSmtConfigParserError::EntitiesError(
                EntitiesParserError::NumEntitiesNotSet
            ))
        );
    }

    #[test]
    fn builder_with_all_values() {
        let height = Height::expect_from(8);
        let num_random_entities = 10;

        let src_dir = env!("CARGO_MANIFEST_DIR");
        let resources_dir = Path::new(&src_dir).join("examples");
        let secrets_file_path = resources_dir.join("ndm_smt_secrets_example.toml");
        let entities_file_path = resources_dir.join("entities_example.csv");

        let entities_file = File::open(entities_file_path.clone()).unwrap();
        // "-1" because we don't include the top line of the csv which defines
        // the column headings.
        let num_entities = BufReader::new(entities_file).lines().count() - 1;

        let ndm_smt = NdmSmtConfigBuilder::default()
            .height(height.clone())
            .secrets_file_path(secrets_file_path)
            .entities_path(entities_file_path)
            .num_random_entities(num_random_entities)
            .build()
            .parse()
            .unwrap();

        assert_eq!(ndm_smt.entity_mapping.len(), num_entities);
        assert_eq!(ndm_smt.height(), &height);
    }

    #[test]
    fn builder_without_secrets_file_path() {
        let num_random_entities = 10;

        let _ndm_smt = NdmSmtConfigBuilder::default()
            .num_random_entities(num_random_entities)
            .build()
            .parse()
            .unwrap();
    }
}
