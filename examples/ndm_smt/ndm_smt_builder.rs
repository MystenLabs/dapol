//! Example on how to use the builder pattern to construct an NDM-SMT tree.

use std::path::Path;

pub fn build_ndm_smt_using_builder_pattern() -> dapol::accumulators::NdmSmt {
    let src_dir = env!("CARGO_MANIFEST_DIR");
    let resources_dir = Path::new(&src_dir).join("examples");

    let secrets_file = resources_dir.join("ndm_smt_secrets_example.toml");
    let entities_file = resources_dir.join("entities_example.csv");

    let height = dapol::Height::try_from(16).unwrap();

    let config = dapol::accumulators::NdmSmtConfigBuilder::default()
        .height(height)
        .secrets_file_path(secrets_file)
        .entities_path(entities_file)
        .build()
        .unwrap();

    config.parse().unwrap()
}
