//! Example on how to build a tree using a config file.
//!
//! The config file can be used for any accumulator type since the type is
//! specified by the config file.
//!
//! This is also an example usage of [dapol][utils][LogOnErrUnwrap].

use std::path::Path;
use dapol::utils::LogOnErrUnwrap;

pub fn build_accumulator_using_config_file() -> dapol::Accumulator {
    let src_dir = env!("CARGO_MANIFEST_DIR");
    let resources_dir = Path::new(&src_dir).join("examples");
    let config_file = resources_dir.join("tree_config_example.toml");

    dapol::AccumulatorConfig::deserialize(config_file)
        .log_on_err_unwrap()
        .parse()
        .log_on_err_unwrap()
}
