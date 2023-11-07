//! Wrapper for holding an integer-valued percentage.

use clap::builder::{OsStr, Str};
use serde::{Deserialize, Serialize};
use std::{convert::From, num::ParseIntError, str::FromStr};

pub const ONE_HUNDRED_PERCENT: Percentage = Percentage { value: 100 };

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Percentage {
    value: u8,
}

impl Percentage {
    /// Returns a new `Percentage` with the given value.
    /// Returns an error if the value is greater than 100.
    pub fn from_with_err(value: u8) -> Result<Percentage, ParsePercentageError> {
        if value > 100 {
            Err(ParsePercentageError::InputTooBig(value))
        } else {
            Ok(Percentage { value })
        }
    }

    /// Returns a new `Percentage` with the given value.
    /// Panics if the value is greater than 100.
    pub fn from(value: u8) -> Percentage {
        if value > 100 {
            panic!("Invalid percentage value {}", value);
        } else {
            Percentage { value }
        }
    }

    /// Returns the percentage applied to the number given.
    pub fn apply_to(&self, value: u8) -> u8 {
        ((value as u16 * self.value as u16) / 100u16) as u8
    }

    /// Returns the percentage saved.
    pub fn value(&self) -> u8 {
        self.value
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

#[derive(thiserror::Error, Debug)]
pub enum ParsePercentageError {
    #[error("Input value {0} cannot be greater than 100")]
    InputTooBig(u8),
    #[error("Malformed string input for u8")]
    MalformedString(#[from] ParseIntError),
}

// -------------------------------------------------------------------------------------------------
// From traits for the CLI.

impl FromStr for Percentage {
    type Err = ParsePercentageError;

    /// Constructor that takes in a string slice.
    /// If the length of the str is greater than the max then Err is returned.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Percentage::from_with_err(u8::from_str(s)?)
    }
}

impl From<Percentage> for OsStr {
    fn from(percentage: Percentage) -> OsStr {
        OsStr::from(Str::from(percentage.value.to_string()))
    }
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn from_should_panic_if_value_is_over_100() {
        Percentage::from_with_err(101).unwrap();
    }

    #[test]
    fn from_should_save_value_on_u8_format() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from_with_err(15).unwrap().value);
    }
}
