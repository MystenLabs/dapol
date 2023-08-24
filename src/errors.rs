//! Legacy

use displaydoc::Display;
use thiserror::Error;

/// Represents a generic error type
#[derive(Debug, Display, Error)]
pub enum DapolError {
    /// DAPOL tree height must not exceed {0}, but was {1}
    TreeHeightTooBig(usize, usize),
    /// For a liability set of {0} accounts, tree height must be at least {1}, but was {2}
    SparsityTooSmall(usize, usize, usize),
    /// Expected digest size to be {0}, but was {1}
    InvalidDigestSize(usize, usize),
    /// Liability set contains a duplicated internal ID {0:?}
    DuplicatedInternalId(Vec<u8>),
    /// Failed to map audit ID {0:?} to a tree index within {1} tries
    FailedToMapIndex(Vec<u8>, usize),
}
