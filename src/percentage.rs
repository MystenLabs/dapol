//! Wrapper for holding an integer-valued percentage.

use clap::builder::{OsStr, Str};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, num::ParseIntError, str::FromStr};

pub const ZERO_PERCENT: Percentage = Percentage { value: 0 };
pub const FIFTY_PERCENT: Percentage = Percentage { value: 50 };
pub const ONE_HUNDRED_PERCENT: Percentage = Percentage { value: 100 };

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Percentage {
    value: u8,
}

impl Percentage {
    /// Returns a new `Percentage` with the given value.
    /// Panics if the value is greater than 100.
    pub fn expect_from(value: u8) -> Percentage {
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

impl TryFrom<u8> for Percentage {
    type Error = PercentageParserError;

    /// Returns a new `Percentage` with the given value.
    /// Returns an error if the value is greater than 100.
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 100 {
            Err(PercentageParserError::InputTooBig(value))
        } else {
            Ok(Percentage { value })
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

#[derive(thiserror::Error, Debug)]
pub enum PercentageParserError {
    #[error("Input value {0} cannot be greater than 100")]
    InputTooBig(u8),
    #[error("Malformed string input for u8")]
    MalformedString(#[from] ParseIntError),
}

// -------------------------------------------------------------------------------------------------
// From traits for the CLI.

impl FromStr for Percentage {
    type Err = PercentageParserError;

    /// Constructor that takes in a string slice.
    /// If the length of the str is greater than the max then Err is returned.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Percentage::try_from(u8::from_str(s)?)
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
    use crate::utils::test_utils::assert_err;

    #[test]
    #[should_panic]
    fn from_should_panic_if_value_is_over_100() {
        Percentage::expect_from(101);
    }

    #[test]
    fn from_should_give_err_if_value_is_over_100() {
        let res = Percentage::try_from(101);
        assert_err!(res, Err(PercentageParserError::InputTooBig(101)));
    }

    #[test]
    fn from_str_happy_case() {
        Percentage::from_str("50").unwrap();
    }

    #[test]
    #[should_panic]
    fn from_str_should_panic_if_value_is_over_100() {
        Percentage::from_str("101").unwrap();
    }

    #[test]
    #[should_panic]
    fn from_str_should_panic_if_value_is_not_u8() {
        Percentage::from_str("bleh").unwrap();
    }

    #[test]
    fn try_from_happy_case() {
        let test = 15;
        assert_eq!(test, Percentage::try_from(15).unwrap().value);
    }

    #[test]
    fn from_happy_case() {
        let test = 15;
        assert_eq!(test, Percentage::expect_from(15).value);
    }
}
