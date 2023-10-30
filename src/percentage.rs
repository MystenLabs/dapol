//! Copied from [percentage].
//! Only PercentageInteger was kept, PercentageDecimal was not needed.
//!
//! # Percentage
//!
//! `percentage` is a crate trying to make using percentages in a safer way and easier to debug.
//! Whenever you see a Percentage, you will know what is being calculated, instead of having to revise the code.

use num::{Num, NumCast};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct PercentageInteger {
    value: u8,
}

impl PercentageInteger {
    /// Returns the percentage applied to the number given.
    ///
    /// # Arguments
    ///
    /// * `value` - The number to apply the percentage.
    pub fn apply_to<T: Num + Ord + Copy + NumCast>(&self, value: T) -> T {
        (value * NumCast::from(self.value).unwrap()) / NumCast::from(100).unwrap()
    }

    /// Returns the percentage saved.
    pub fn value(&self) -> u8 {
        self.value
    }
}

pub struct Percentage;

impl Percentage {
    /// Returns a new `PercentageInteger` with the Given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The number to use as the percentage between 0 and 100.
    ///
    /// # Panics
    /// - Panics if `value` is over 100
    /// - Panics if `value` is below 0
    pub fn from<T: Num + Ord + Copy + NumCast>(value: T) -> PercentageInteger {
        let value: u8 = NumCast::from(value)
            .unwrap_or_else(|| panic!("Percentage value must be between 0 and 100"));
        if value > 100 {
            panic!("Percentage value must be between 0 and 100");
        }
        PercentageInteger { value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn from_should_panic_if_value_is_over_100() {
        Percentage::from(101);
    }

    #[test]
    #[should_panic]
    fn from_should_panic_if_value_is_below_0() {
        Percentage::from(-1);
    }

    #[test]
    fn from_should_save_value_on_u8_format() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15).value);
    }

    #[test]
    fn from_should_save_value_from_i8_or_u8() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as i8).value);
        assert_eq!(test, Percentage::from(15 as u8).value);
    }

    #[test]
    fn from_should_save_value_from_i16_or_u16() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as i16).value);
        assert_eq!(test, Percentage::from(15 as u16).value);
    }

    #[test]
    fn from_should_save_value_from_i32_or_u32() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as i32).value);
        assert_eq!(test, Percentage::from(15 as u32).value);
    }

    #[test]
    fn from_should_save_value_from_i64_or_u64() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as i64).value);
        assert_eq!(test, Percentage::from(15 as u64).value);
    }

    #[test]
    fn from_should_save_value_from_i128_or_u128() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as i128).value);
        assert_eq!(test, Percentage::from(15 as u128).value);
    }

    #[test]
    fn from_should_save_value_from_isize_or_usize() {
        let test: u8 = 15;
        assert_eq!(test, Percentage::from(15 as isize).value);
        assert_eq!(test, Percentage::from(15 as usize).value);
    }
}
