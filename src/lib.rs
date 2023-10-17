// legacy

mod dapol;
pub use crate::dapol::{Dapol, DapolNode};

mod proof;
pub use proof::{DapolProof, DapolProofNode};

mod range;
pub use range::{RangeProofPadding, RangeProofSplitting, RangeProvable, RangeVerifiable};

pub mod errors;
pub mod utils;

#[cfg(test)]
mod tests;

// new

mod binary_tree;
mod node_content;
mod kdf;
mod primitives;

mod secret;
pub use secret::Secret;

mod inclusion_proof;
pub use inclusion_proof::{InclusionProof, InclusionProofError};

mod entity;
pub use entity::{Entity, EntityId, EntityParser};

mod accumulators;
pub use accumulators::{NdmSmt, Secrets, SecretsParser};

mod cli;
pub use cli::Cli;

#[cfg(test)]
mod test_utils;
