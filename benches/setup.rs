use std::fs;
use std::path::{Path, PathBuf};
use std::thread::LocalKey;

use dapol::accumulators::{NdmSmt, NdmSmtConfigBuilder};
use dapol::{read_write_utils, EntityId, Height, InclusionProof, MaxThreadCount};
use log::info;

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

pub fn build_ndm_smt(tup: (Height, MaxThreadCount, u64)) -> Result<NdmSmt, ()> {
    let height_int = tup.0.as_raw_int();
    let max_users_for_height = 2_u64.pow((height_int - 1) as u32);


    if tup.2 > max_users_for_height {
        return Err(())
    }

    if tup.1.get_value() > MaxThreadCount::default().get_value() {
        return Err(())
    }

    Ok(NdmSmtConfigBuilder::default()
        .height(tup.0)
        .max_thread_count(tup.1)
        .num_entities(tup.2)
        .build()
        .map_err(|_| ())?
        .parse()
        .map_err(|_| ())?)
}

pub fn generate_proof(ndm_smt: &NdmSmt, entity_id: &EntityId) -> InclusionProof {
    NdmSmt::generate_inclusion_proof(ndm_smt, entity_id).expect("Unable to generate proof")
}

pub fn serialize_tree(tree: &NdmSmt, dir: PathBuf) {
    let mut file_name = tree.root_hash().to_string();
    file_name.push('.');
    file_name.push_str("dapoltree");

    let path = dir.join(file_name);
    info!("Serializing tree build to path {:?}", path);

    read_write_utils::serialize_to_bin_file(&tree, path.clone())
        .expect("Unable to serialize proof");

    let file_size = fs::metadata(path)
        .expect("Unable to get tree metadata for {tree.root_hash()}")
        .len();

    println!("Tree file size: {} kB", file_size / 1024u64);
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
