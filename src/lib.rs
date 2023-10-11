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

mod inclusion_proof;
pub use inclusion_proof::{InclusionProof, InclusionProofError};

mod primitives;
pub use primitives::D256;

mod entity;
pub use entity::{Entity, EntityId};

mod accumulators;
pub use accumulators::NdmSmt;

mod cli;
pub use cli::Args;

#[cfg(test)]
mod testing_utils;
