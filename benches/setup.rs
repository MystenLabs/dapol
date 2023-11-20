use bulletproofs::PedersenGens;
use curve25519_dalek_ng::scalar::Scalar;
use log::info;
use primitive_types::H256;
use serde::Serialize;

use core::fmt::Debug;
use std::path::{Path, PathBuf};

use dapol::accumulators::{NdmSmt, NdmSmtConfigBuilder};
use dapol::node_content::FullNodeContent;
use dapol::{read_write_utils, EntityId, MaxThreadCount};
use dapol::{Coordinate, Mergeable};
use dapol::{Hasher, Height, InclusionProof};

// CONSTANTS
// ================================================================================================

pub const TREE_HEIGHTS: [u8; 3] = [16, 32, 64];
pub const NUM_USERS: [u64; 35] = [
    10_000,
    20_000,
    30_000,
    40_000,
    50_000,
    60_000,
    70_000,
    80_000,
    90_000,
    100_000,
    200_000,
    300_000,
    400_000,
    500_000,
    600_000,
    700_000,
    800_000,
    900_000,
    1_000_000,
    2_000_000,
    3_000_000,
    4_000_000,
    5_000_000,
    6_000_000,
    7_000_000,
    8_000_000,
    9_000_000,
    10_000_000,
    30_000_000,
    50_000_000,
    70_000_000,
    90_000_000,
    100_000_000,
    125_000_000,
    250_000_000,
];

// HELPER FUNCTIONS
// ================================================================================================

pub fn build_ndm_smt(
    height: Height,
    max_thread_count: MaxThreadCount,
    num_entities: u64,
) -> NdmSmt {
    let src_dir = env!("CARGO_MANIFEST_DIR");
    let resources_dir = Path::new(&src_dir).join("examples");
    let secrets_file_path = resources_dir.join("ndm_smt_secrets_example.toml");

    NdmSmtConfigBuilder::default()
        .height(height)
        .max_thread_count(max_thread_count)
        .secrets_file_path(secrets_file_path)
        .num_entities(num_entities)
        .build()
        .expect("Unable to build NdmSmtConfig")
        .parse()
        .expect("Unable to parse NdmSmt")
}

pub fn generate_proof(ndm_smt: &NdmSmt, entity_id: &EntityId) -> InclusionProof {
    NdmSmt::generate_inclusion_proof(ndm_smt, entity_id).expect("Unable to generate proof")
}

// pub fn serialize_proof(proof: InclusionProof, entity_id: EntityId, dir: PathBuf) -> PathBuf {
//     let mut file_name = entity_id.to_string();
//     file_name.push('.');
//     file_name.push_str("dapolproof");

//     let path = dir.join(file_name);
//     info!("Serializing inclusion proof to path {:?}", path);

//     read_write_utils::serialize_to_bin_file(&proof, path.clone())
//         .expect("Unable to serialize proof");

//     path
// }
