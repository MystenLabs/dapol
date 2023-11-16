//! Example of a full PoL workflow.
//!
//! 1. Build a tree
//! 2. Generate an inclusion proof
//! 3. Verify an inclusion proof
//!
//! At the time of writing (Nov 2023) only the NDM-SMT accumulator is supported
//! so this is the only type of tree that is used in this example.

use std::str::FromStr;

extern crate clap_verbosity_flag;
extern crate csv;
extern crate dapol;

mod ndm_smt_builder;
use ndm_smt_builder::build_ndm_smt_using_builder_pattern;

mod accumulator_config_parser;
use accumulator_config_parser::build_accumulator_using_config_file;

mod inclusion_proof_handling;
use inclusion_proof_handling::{simple_inclusion_proof_generation_and_verification, advanced_inclusion_proof_generation_and_verification};

fn main() {
    let log_level = clap_verbosity_flag::LevelFilter::Debug;
    dapol::utils::activate_logging(log_level);

    // =========================================================================
    // Tree building.

    let ndm_smt = build_ndm_smt_using_builder_pattern();
    let accumulator = build_accumulator_using_config_file();

    // The above 2 builder methods produce a different tree because the entities
    // are mapped randomly to points on the bottom layer, but the entity mapping
    // of one tree should simply be a permutation of the other. We check this:
    let ndm_smt_other = match accumulator {
        dapol::Accumulator::NdmSmt(ndm_smt_other) => {
            assert_ne!(ndm_smt_other.root_hash(), ndm_smt.root_hash());

            for (entity, _) in ndm_smt_other.entity_mapping() {
                assert!(ndm_smt.entity_mapping().contains_key(&entity));
            }

            ndm_smt_other
        }
    };

    // =========================================================================
    // Inclusion proof generation & verification.

    let entity_id = dapol::EntityId::from_str("john.doe@example.com").unwrap();
    simple_inclusion_proof_generation_and_verification(&ndm_smt, entity_id.clone());
    advanced_inclusion_proof_generation_and_verification(&ndm_smt_other, entity_id);
}
