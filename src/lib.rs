// Copyright ⓒ 2023 SilverSixpence
// Licensed under the MIT license
// (see LICENSE or <http://opensource.org/licenses/MIT>) All files in the project carrying such
// notice may not be copied, modified, or distributed except according to those terms.

//! # Proof of Liabilities protocol implemented in Rust
//!
//! Implementation of the DAPOL+ protocol introduced in the "Generalized Proof of Liabilities" by Yan Ji and Konstantinos Chalkias ACM CCS 2021 paper, available [here](https://eprint.iacr.org/2021/1350)
//!
//! See the [top-level doc for the project](https://hackmd.io/p0dy3R0RS5qpm3sX-_zreA) if you would like to know more about Proof of Liabilities.
//!
//! ## What is contained in this code
//!
//! This library offers an efficient build algorithm for constructing a binary Merkle Sum Tree representing the liabilities of an organization. Efficiency is achieved through parallelization. Details on the algorithm used can be found in [the multi-threaded builder file](https://github.com/silversixpence-crypto/dapol/blob/main/src/binary_tree/tree_builder/multi_threaded.rs).
//!
//! The paper describes a few different accumulator variants. The Sparse Merkle Sum Tree is the DAPOL+ accumulator, but there are a few different axes of variation, such as how the list of entities is embedded within the tree. The 4 accumulator variants are simply slightly different versions of the Sparse Merkle Sum Tree. Only the Non-Deterministic Mapping Sparse Merkle Tree variant has been implemented so far.
//!
//! The code offers inclusion proof generation & verification using the Bulletproofs protocol for the range proofs.
//!
//! ## Still to be done
//!
//! This project is currently still a work in progress, but is ready for
//! use as is. The code has _not_ been audited yet (as of Nov 2023). Progress can be tracked [here](https://github.com/silversixpence-crypto/dapol/issues/91).
//!
//! A Rust crate has not been released yet, progress can be tracked [here](https://github.com/silversixpence-crypto/dapol/issues/13).
//!
//! A spec for this code still needs to be [written](https://github.com/silversixpence-crypto/dapol/issues/17).
//!
//! A fuzzing technique should be used for the unit [tests](https://github.com/silversixpence-crypto/dapol/issues/46).
//!
//! Performance can be [improved](https://github.com/silversixpence-crypto/dapol/issues/44).
//!
//! Alternate accumulators mentioned in the paper should be built:
//! - [Deterministic mapping SMT](https://github.com/silversixpence-crypto/dapol/issues/9)
//! - [ORAM-based SMT](https://github.com/silversixpence-crypto/dapol/issues/8)
//! - [Hierarchical SMTs](https://github.com/silversixpence-crypto/dapol/issues/7)
//!
//! Other than the above there are a few minor tasks to do, each of which has an issue for tracking.
//!
//! ## How this code can be used
//!
//! There is both a Rust API and a CLI. Details for the API can be found below, and details for the CLI can be found [here](https://github.com/silversixpence-crypto/dapol#cli).
//!
//! ### Rust API
//!
//! The library has not been released as a crate yet (as of Nov 2023) but the API has the following capabilities:
//! - build a tree using the builder pattern or a configuration file
//! - generate inclusion proofs from a list of entity IDs (tree required)
//! - verify an inclusion proof using a root hash (no tree required)
//!
//! ```
//! use std::str::FromStr;
//! use std::path::Path;
//!
//! use dapol::utils::LogOnErrUnwrap;
//!
//! fn main() {
//!     let log_level = clap_verbosity_flag::LevelFilter::Debug;
//!     dapol::utils::activate_logging(log_level);
//!
//!     // =========================================================================
//!     // Tree building.
//!
//!     let ndm_smt = build_ndm_smt_using_builder_pattern();
//!     let accumulator = build_accumulator_using_config_file();
//!
//!     // The above 2 builder methods produce a different tree because the entities
//!     // are mapped randomly to points on the bottom layer, but the entity mapping
//!     // of one tree should simply be a permutation of the other. We check this:
//!     let ndm_smt_other = match accumulator {
//!         dapol::Accumulator::NdmSmt(ndm_smt_other) => {
//!             assert_ne!(ndm_smt_other.root_hash(), ndm_smt.root_hash());
//!
//!             for (entity, _) in ndm_smt_other.entity_mapping() {
//!                 assert!(ndm_smt.entity_mapping().contains_key(&entity));
//!             }
//!
//!             ndm_smt_other
//!         }
//!     };
//!
//!     // =========================================================================
//!     // Inclusion proof generation & verification.
//!
//!     let entity_id = dapol::EntityId::from_str("john.doe@example.com").unwrap();
//!     simple_inclusion_proof_generation_and_verification(&ndm_smt, entity_id.clone());
//!     advanced_inclusion_proof_generation_and_verification(&ndm_smt_other, entity_id);
//! }
//!
//! /// Example on how to use the builder pattern to construct an NDM-SMT tree.
//! pub fn build_ndm_smt_using_builder_pattern() -> dapol::NdmSmt {
//!     let src_dir = env!("CARGO_MANIFEST_DIR");
//!     let resources_dir = Path::new(&src_dir).join("examples");
//!
//!     let secrets_file = resources_dir.join("ndm_smt_secrets_example.toml");
//!     let entities_file = resources_dir.join("entities_example.csv");
//!
//!     let height = dapol::Height::from(16);
//!
//!     let config = dapol::NdmSmtConfigBuilder::default()
//!         .height(height)
//!         .secrets_file_path(secrets_file)
//!         .entities_path(entities_file)
//!         .build()
//!         .unwrap();
//!
//!     config.parse().unwrap()
//! }
//!
//! /// An inclusion proof can be generated from only a tree + entity ID.
//! pub fn simple_inclusion_proof_generation_and_verification(
//!     ndm_smt: &dapol::NdmSmt,
//!     entity_id: dapol::EntityId,
//! ) {
//!     let inclusion_proof = ndm_smt.generate_inclusion_proof(&entity_id).unwrap();
//!     inclusion_proof.verify(ndm_smt.root_hash()).unwrap();
//! }
//!
//! /// The inclusion proof generation algorithm can be customized via some
//! /// parameters. See [dapol][InclusionProof] for more details.
//! pub fn advanced_inclusion_proof_generation_and_verification(
//!     ndm_smt: &dapol::NdmSmt,
//!     entity_id: dapol::EntityId,
//! ) {
//!     // Determines how many of the range proofs in the inclusion proof are
//!     // aggregated together. The ones that are not aggregated are proved
//!     // individually. The more that are aggregated the faster the proving
//!     // and verification times.
//!     let aggregation_percentage = dapol::percentage::ONE_HUNDRED_PERCENT;
//!     let aggregation_factor = dapol::AggregationFactor::Percent(aggregation_percentage);
//!     let aggregation_factor = dapol::AggregationFactor::default();
//!
//!     // 2^upper_bound_bit_length is the upper bound used in the range proof i.e.
//!     // the secret value is shown to reside in the range [0, 2^upper_bound_bit_length].
//!     let upper_bound_bit_length = 32u8;
//!     let upper_bound_bit_length = dapol::DEFAULT_RANGE_PROOF_UPPER_BOUND_BIT_LENGTH;
//!
//!     let inclusion_proof = ndm_smt
//!         .generate_inclusion_proof_with(&entity_id, aggregation_factor, upper_bound_bit_length)
//!         .unwrap();
//!
//!     inclusion_proof.verify(ndm_smt.root_hash()).unwrap();
//! }
//!
//! /// Example on how to build a tree using a config file.
//! ///
//! /// The config file can be used for any accumulator type since the type is
//! /// specified by the config file.
//! ///
//! /// This is also an example usage of [dapol][utils][LogOnErrUnwrap].
//! pub fn build_accumulator_using_config_file() -> dapol::Accumulator {
//!     let src_dir = env!("CARGO_MANIFEST_DIR");
//!     let resources_dir = Path::new(&src_dir).join("examples");
//!     let config_file = resources_dir.join("tree_config_example.toml");
//!
//!     dapol::AccumulatorConfig::deserialize(config_file)
//!         .log_on_err_unwrap()
//!         .parse()
//!         .log_on_err_unwrap()
//! }
//! ```

mod kdf;
mod node_content;

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

mod binary_tree;
pub use binary_tree::Height;

mod secret;
pub use secret::{Secret, SecretParserError};

mod inclusion_proof;
pub use inclusion_proof::{
    AggregationFactor, InclusionProof, InclusionProofError,
    DEFAULT_RANGE_PROOF_UPPER_BOUND_BIT_LENGTH,
};

mod entity;
pub use entity::{Entity, EntityId, EntityIdsParser, EntityIdsParserError};
