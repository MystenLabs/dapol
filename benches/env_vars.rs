use dapol::Height;
use once_cell::sync::Lazy;
use std::str::FromStr;

/// Sets the lower bound on the number of entities for benchmarks.
///
/// A pre-determined list of entity numbers is looped over, and each one is used
/// as input for a benchmark. This env var sets the lower bound in that list.
pub static MIN_ENTITIES: Lazy<u64> = Lazy::new(|| {
    std::env::var("MIN_ENTITIES")
        .unwrap_or("0".to_string())
        .parse()
        .unwrap()
});

/// Sets the upper bound on the number of entities for benchmarks.
///
/// A pre-determined list of entity numbers is looped over, and each one is used
/// as input for a benchmark. This env var sets the upper bound in that list.
pub static MAX_ENTITIES: Lazy<u64> = Lazy::new(|| {
    std::env::var("MAX_ENTITIES")
        .unwrap_or("250000000".to_string())
        .parse()
        .unwrap()
});

/// Sets the lower bound on the tree height for benchmarks.
///
/// A pre-determined list of heights is looped over, and each one is used
/// as input for a benchmark. This env var sets the lower bound in that list.
pub static MIN_HEIGHT: Lazy<Height> = Lazy::new(|| {
    Height::from_str(
        std::env::var("MIN_HEIGHT")
            .unwrap_or("0".to_string())
            .as_str(),
    )
    .unwrap()
});

/// Sets the upper bound on the number of height for benchmarks.
///
/// A pre-determined list of entity numbers is looped over, and each one is used
/// as input for a benchmark. This env var sets the upper bound in that list.
pub static MAX_HEIGHT: Lazy<Height> = Lazy::new(|| {
    Height::from_str(
        std::env::var("MAX_HEIGHT")
            .unwrap_or("250000000".to_string())
            .as_str(),
    )
    .unwrap()
});

use clap_verbosity_flag::{Level, LevelFilter};

/// Set the log level of the dapol code.
pub static LOG_VERBOSITY: Lazy<LevelFilter> = Lazy::new(|| {
    std::env::var("LOG_VERBOSITY")
        .map(|x| Level::from_str(&x).unwrap().to_level_filter())
        .unwrap_or(LevelFilter::Off)
});
