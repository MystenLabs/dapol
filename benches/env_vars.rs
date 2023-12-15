use dapol::{Height, MaxThreadCount};
use once_cell::sync::Lazy;
use std::str::FromStr;

/// Sets the lower bound on the number of total thread count for benchmarks.
///
/// A pre-determined list of entity numbers is looped over, and each one is used
/// as input for a benchmark. This env var sets the lower bound in that list.
pub static MIN_TOTAL_THREAD_COUNT: Lazy<MaxThreadCount> = Lazy::new(|| {
    MaxThreadCount::from_str(
        std::env::var("MIN_TOTAL_THREAD_COUNT")
            .unwrap_or("0".to_string())
            .as_str(),
    )
    .expect("MIN_TOTAL_THREAD_COUNT env var string parsing error")
});

/// Sets the lower bound on the number of entities for benchmarks.
///
/// A pre-determined list of entity numbers is looped over, and each one is used
/// as input for a benchmark. This env var sets the lower bound in that list.
pub static MIN_ENTITIES: Lazy<u64> = Lazy::new(|| {
    std::env::var("MIN_ENTITIES")
        .unwrap_or("0".to_string())
        .parse()
        .expect("MIN_ENTITIES env var string parsing error")
});

/// Sets the upper bound on the number of entities for benchmarks.
///
/// A pre-determined list of entity numbers is looped over, and each one is used
/// as input for a benchmark. This env var sets the upper bound in that list.
pub static MAX_ENTITIES: Lazy<u64> = Lazy::new(|| {
    std::env::var("MAX_ENTITIES")
        .unwrap_or("250000000".to_string())
        .parse()
        .expect("MAX_ENTITIES env var string parsing error")
});

/// Sets the lower bound on the tree height for benchmarks.
///
/// A pre-determined list of heights is looped over, and each one is used
/// as input for a benchmark. This env var sets the lower bound in that list.
pub static MIN_HEIGHT: Lazy<Height> = Lazy::new(|| {
    Height::from_str(
        std::env::var("MIN_HEIGHT")
            .unwrap_or(dapol::MIN_HEIGHT.as_u32().to_string())
            .as_str(),
    )
    .expect("MIN_HEIGHT env var string parsing error")
});

/// Sets the upper bound on the number of height for benchmarks.
///
/// A pre-determined list of entity numbers is looped over, and each one is used
/// as input for a benchmark. This env var sets the upper bound in that list.
pub static MAX_HEIGHT: Lazy<Height> = Lazy::new(|| {
    Height::from_str(
        std::env::var("MAX_HEIGHT")
            .unwrap_or(dapol::MAX_HEIGHT.as_u32().to_string())
            .as_str(),
    )
    .expect("MAX_HEIGHT env var string parsing error")
});

use clap_verbosity_flag::{Level, LevelFilter};

/// Set the log level of the dapol code.
pub static LOG_VERBOSITY: Lazy<LevelFilter> = Lazy::new(|| {
    std::env::var("LOG_VERBOSITY")
        .map(|x| Level::from_str(&x).unwrap().to_level_filter())
        .unwrap_or(LevelFilter::Off)
});
