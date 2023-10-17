mod binary_tree;
mod node_content;
mod kdf;
mod primitives;

mod secret;
pub use secret::Secret;

mod inclusion_proof;
pub use inclusion_proof::{InclusionProof, InclusionProofError};

mod entity;
pub use entity::{Entity, EntityId, EntitiesParser, generate_random_entities};

mod accumulators;
pub use accumulators::{NdmSmt, Secrets, SecretsParser};

mod cli;
pub use cli::Cli;

use env_logger;
use clap_verbosity_flag::{LevelFilter};

#[cfg(test)]
mod test_utils;

pub fn activate_logging(log_level: LevelFilter) {
    env_logger::Builder::new().filter_level(log_level).init();
}
