//! Abstracted height data type.

use std::{num::ParseIntError, str::FromStr};

static UNDERLYING_INT_TYPE_STR: &str = "u8";
type UnderlyingInt = u8;

/// Minimum tree height supported.
/// It does not make any sense to have a tree of size 1 and the code may
/// actually break with this input so 2 is a reasonable minimum.
pub static MIN_HEIGHT: UnderlyingInt = 2;

/// Maximum tree height supported.
/// This number does not have any programmatic/theoretic reason for being 64,
/// it's just a soft limit that can be increased later if need be.
pub static MAX_HEIGHT: UnderlyingInt = 64;

/// 2^32 is about half the human population so it is a reasonable default height
/// to have for any protocol involving people as the entities.
pub static DEFAULT_HEIGHT: UnderlyingInt = 32;

#[derive(Clone, Debug)]
pub struct Height(UnderlyingInt);

impl Height {
    fn from(int: UnderlyingInt) -> Result<Self, HeightError> {
        if int < MIN_HEIGHT {
            Err(HeightError::InputTooSmall)
        } else if int > MAX_HEIGHT {
            Err(HeightError::InputTooBig)
        } else {
            Ok(Height(int))
        }
    }
}

impl FromStr for Height {
    type Err = HeightError;

    /// Constructor that takes in a string slice.
    /// If the length of the str is greater than the max then Err is returned.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Height::from(UnderlyingInt::from_str(s)?)?)
    }
}

impl Default for Height {
    fn default() -> Self {
        Height(DEFAULT_HEIGHT)
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum HeightError {
    #[error("Input is greater than the upper bound {MAX_HEIGHT:?}")]
    InputTooBig,
    #[error("Input is smaller than the lower bound {MIN_HEIGHT:?}")]
    InputTooSmall,
    #[error("Malformed string input for {UNDERLYING_INT_TYPE_STR:?} type")]
    MalformedString(#[from] ParseIntError),
}
