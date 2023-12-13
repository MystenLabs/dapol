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

/// Set the log level of the dapol code.
pub static LOG_VERBOSITY: Lazy<clap_verbosity_flag::LevelFilter> = Lazy::new(|| {
    clap_verbosity_flag::Level::from_str(
        &std::env::var("LOG_VERBOSITY").unwrap_or("WARN".to_string()),
    )
    .unwrap()
    .to_level_filter()
});
