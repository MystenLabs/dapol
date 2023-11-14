//! Utilities used across the whole crate.

// -------------------------------------------------------------------------------------------------
// Logging.

use clap_verbosity_flag::LevelFilter;

pub fn activate_logging(log_level: LevelFilter) {
    env_logger::Builder::new().filter_level(log_level).init();
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

pub trait LogOnErrUnwrap<T> {
    fn log_on_err_unwrap(self) -> T;
}
impl<T, E: Debug + Display> LogOnErrUnwrap<T> for Result<T, E> {
    /// Produce an error [log] if self is an Err, then unwrap.
    fn log_on_err_unwrap(self) -> T {
        match &self {
            Err(err) => error!("{:?} {}", err, err),
            Ok(_) => {}
        }
        self.unwrap()
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

pub trait IfSomeThen<T> {
    fn if_some_then<F>(self, f: F) -> Option<T>
    where
        F: FnOnce(&T);
}
impl<T> IfSomeThen<T> for Option<T> {
    /// If Some then execute the function on the underlying value. Always return
    /// Option as it was.
    fn if_some_then<F>(self, f: F) -> Option<T>
    where
        F: FnOnce(&T),
    {
        match &self {
            None => {}
            Some(x) => f(x),
        }
        self
    }
}

pub trait IfNoneThen<T> {
    fn if_none_then<F>(self, f: F) -> Option<T>
    where
        F: FnOnce();
}
impl<T> IfNoneThen<T> for Option<T> {
    /// If None then execute the function on the underlying value. Always return
    /// Option as it was.
    fn if_none_then<F>(self, f: F) -> Option<T>
    where
        F: FnOnce(),
    {
        match &self {
            None => f(),
            Some(_) => {},
        }
        self
    }
}

pub trait ErrOnSome {
    fn err_on_some<E>(&self, err: E) -> Result<(), E>;
}
impl<T> ErrOnSome for Option<T> {
    /// Return an error if `Some(_)`, otherwise do nothing.
    fn err_on_some<E>(&self, err: E) -> Result<(), E> {
        match self {
            None => Ok(()),
            Some(_) => Err(err),
        }
    }
}

pub trait ErrUnlessTrue {
    fn err_unless_true<E>(&self, err: E) -> Result<(), E>;
}
impl ErrUnlessTrue for Option<bool> {
    /// Return an error if `None` or `Some(false)`, otherwise do nothing.
    fn err_unless_true<E>(&self, err: E) -> Result<(), E> {
        match self {
            None => Err(err),
            Some(false) => Err(err),
            Some(true) => Ok(()),
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
