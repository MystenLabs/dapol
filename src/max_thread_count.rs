use log::{error, warn};
use serde::{Deserialize, Serialize};

/// The default max number of threads.
/// This value is used in the case where the number of threads cannot be
/// determined from the underlying hardware. 4 was chosen as the default because
/// most modern (circa 2023) architectures will have at least 4 cores.
pub const DEFAULT_MAX_THREAD_COUNT: u8 = 4;

/// Abstraction for the max number of threads.
///
/// This struct is used when determining how many threads can be spawned when
/// doing work in parallel.
///
/// Example:
/// ```
/// use dapol::MaxThreadCount;
/// use std::str::FromStr;
///
/// let max_thread_count = MaxThreadCount::default();
/// let max_thread_count = MaxThreadCount::from(8u8);
/// let max_thread_count = MaxThreadCount::from_str("8");
/// ```
#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
pub struct MaxThreadCount(u8);

impl MaxThreadCount {
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl From<u8> for MaxThreadCount {
    fn from(max_thread_count: u8) -> Self {
        Self(max_thread_count)
    }
}

// -------------------------------------------------------------------------------------------------
// Default.

impl Default for MaxThreadCount {
    fn default() -> Self {
        MaxThreadCount(MACHINE_PARALLELISM.with(|opt| match *opt.borrow() {
            None => {
                warn!(
                    "Machine parallelism not set, defaulting max thread count to {}",
                    DEFAULT_MAX_THREAD_COUNT
                );
                DEFAULT_MAX_THREAD_COUNT
            }
            Some(par) => par,
        }))
    }
}

// -------------------------------------------------------------------------------------------------
// From for str.

use std::str::FromStr;

impl FromStr for MaxThreadCount {
    type Err = MaxThreadCountError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(MaxThreadCount(u8::from_str(s)?))
    }
}

// -------------------------------------------------------------------------------------------------
// From for OsStr.

use clap::builder::{OsStr, Str};

impl From<MaxThreadCount> for OsStr {
    fn from(max_thread_count: MaxThreadCount) -> OsStr {
        OsStr::from(Str::from(max_thread_count.as_u8().to_string()))
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

#[derive(thiserror::Error, Debug)]
pub enum MaxThreadCountError {
    #[error("Malformed string input for u8 type")]
    MalformedString(#[from] std::num::ParseIntError),
}

// -------------------------------------------------------------------------------------------------
// Global variable.

use std::cell::RefCell;

// Guessing the number of cores.
// This variable must NOT be shared between more than 1 thread, it is not
// thread-safe.
// https://www.sitepoint.com/rust-global-variables/#singlethreadedglobalswithruntimeinitialization
thread_local!(pub static MACHINE_PARALLELISM: RefCell<Option<u8>> = RefCell::new(None));

/// Initialize [MACHINE_PARALLELISM] using [std][thread][available_parallelism].
///
/// This value is used as a default for how many threads can be spawned when
/// doing work in parallel.
pub fn initialize_machine_parallelism() {
    MACHINE_PARALLELISM.with(|opt| {
        *opt.borrow_mut() = std::thread::available_parallelism()
            .map_err(|err| {
                error!("Problem accessing machine parallelism: {}", err);
                err
            })
            .map_or(None, |par| Some(par.get() as u8))
    });
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_without_initializing_machine_parallelism() {
        assert_eq!(MaxThreadCount::default().as_u8(), DEFAULT_MAX_THREAD_COUNT);
    }
}
