mod kdf;
pub mod node_content; // TODO: this as private in production

pub mod cli;
pub mod percentage;
pub mod read_write_utils;
pub mod utils;

mod hasher;
pub use hasher::Hasher;

mod accumulators;
pub use accumulators::{
    config::{AccumulatorConfig, AccumulatorConfigError},
    ndm_smt::{NdmSmt, NdmSmtConfig, NdmSmtConfigBuilder, NdmSmtError, NdmSmtParserError},
    Accumulator, AccumulatorError,
};

pub mod binary_tree; // TODO: this as private in production
pub use binary_tree::{Height, MAX_THREAD_COUNT};

mod secret;
pub use secret::{Secret, SecretParseError};

mod inclusion_proof;
pub use inclusion_proof::{
    AggregationFactor, InclusionProof, InclusionProofError,
    DEFAULT_RANGE_PROOF_UPPER_BOUND_BIT_LENGTH,
};

mod entity;
pub use entity::{Entity, EntityId, EntityIdsParser, EntityIdsParserError};
