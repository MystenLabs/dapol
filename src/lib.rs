mod binary_tree;
mod node_content;
mod kdf;

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

// -------------------------------------------------------------------------------------------------
// Logging.

pub fn activate_logging(log_level: LevelFilter) {
    env_logger::Builder::new().filter_level(log_level).init();
}

// -------------------------------------------------------------------------------------------------
// H256 extensions.

use primitive_types::H256;

/// Trait for a hasher to output [primitive_types][H256].
pub trait H256Finalizable {
    fn finalize_as_h256(&self) -> H256;
}

impl H256Finalizable for blake3::Hasher {
    fn finalize_as_h256(&self) -> H256 {
        let bytes: [u8; 32] = self.finalize().into();
        H256(bytes)
    }
}
