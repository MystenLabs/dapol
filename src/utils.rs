//! Utilities used across the whole crate.

// -------------------------------------------------------------------------------------------------
// Logging.

use clap_verbosity_flag::LevelFilter;

pub fn activate_logging(log_level: LevelFilter) {
    env_logger::Builder::new().filter_level(log_level).init();
}

// -------------------------------------------------------------------------------------------------
// H256 extensions.

use primitive_types::H256;

/// Trait for a hasher to output [primitive_types][H256].
pub trait H256Finalizable {
    fn finalize_as_h256(&self) -> H256;
}

impl H256Finalizable for blake3::Hasher {
    fn finalize_as_h256(&self) -> H256 {
        let bytes: [u8; 32] = self.finalize().into();
        H256(bytes)
    }
}

// -------------------------------------------------------------------------------------------------
// Traits for Option & Result.

use log::error;
use std::fmt::{Debug, Display};

pub trait LogOnErr {
    fn log_on_err(self) -> Self;
}

impl<T, E: Debug + Display> LogOnErr for Result<T, E> {
    /// Produce an error [log] if self is an Err.
    fn log_on_err(self) -> Self {
        match &self {
            Err(err) => error!("{:?} {}", err, err),
            Ok(_) => {}
        }
        self
    }
}

pub trait Consume<T> {
    fn consume<F>(self, f: F)
    where
        F: FnOnce(T);
}

impl<T> Consume<T> for Option<T> {
    /// If `None` then do nothing and return nothing. If `Some` then call the
    /// given function `f` with the value `T` but do not return anything.
    fn consume<F>(self, f: F)
    where
        F: FnOnce(T),
    {
        match self {
            None => {}
            Some(x) => f(x),
        }
    }
}


// -------------------------------------------------------------------------------------------------
// Global variables.

use std::cell::RefCell;

// Guessing the number of cores.
// This variable must NOT be shared between more than 1 thread, it is not
// thread-safe.
// https://www.sitepoint.com/rust-global-variables/#singlethreadedglobalswithruntimeinitialization
thread_local!(pub static DEFAULT_PARALLELISM_APPROX: RefCell<Option<u8>> = RefCell::new(None));

// -------------------------------------------------------------------------------------------------
// Testing utils.

#[cfg(test)]
pub mod test_utils {
    /// Check 2 errors are the same.
    /// https://stackoverflow.com/a/65618681
    macro_rules! assert_err {
    ($expression:expr, $($pattern:tt)+) => {
        match $expression {
            $($pattern)+ => (),
            ref e => panic!("expected `{}` but got `{:?}`", stringify!($($pattern)+), e),
        }
    }
}
    pub(crate) use assert_err;

    /// Same as [assert_err] but without needing debug
    /// https://stackoverflow.com/a/65618681
    macro_rules! assert_err_simple {
        ($expression:expr, $($pattern:tt)+) => {
            match $expression {
                $($pattern)+ => (),
                _ => panic!("expected a specific error but did not get it"),
            }
        }
    }
    pub(crate) use assert_err_simple;

    pub fn init_logger() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace"))
                .is_test(true)
                .try_init();
    }
}
