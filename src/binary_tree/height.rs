use log::error;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

const UNDERLYING_INT_TYPE_STR: &str = "u8";
type UnderlyingInt = u8;

/// Minimum tree height supported: 2.
///
/// It does not make any sense to have a tree of size 1 and the code may
/// actually break with this input so 2 is a reasonable minimum.
pub const MIN_HEIGHT: Height = Height(2);

/// Maximum tree height supported: 64.
///
/// This number does not have any theoretic reason for being 64,
/// it's just a soft limit that can be increased later if need be. If it is
/// increased then we will need to change the type of the x-coord because it is
/// currently u64, which gives a max tree height of 64.
pub const MAX_HEIGHT: Height = Height(64);
pub type XCoord = u64;

/// 2^32 is about half the human population so it is a reasonable default height
/// to have for any protocol involving people as the entities.
pub const DEFAULT_HEIGHT: UnderlyingInt = 32;

/// Abstraction for the height of the tree.
///
/// Example:
/// ```
/// use dapol::Height;
/// use std::str::FromStr;
///
/// let height = Height::default();
/// let height = Height::try_from(8u8).unwrap();
/// let height = Height::from_str("8");
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Height(UnderlyingInt);

impl Height {
    /// Return the height for the given y-coord.
    ///
    /// Why the offset? `y` starts from 0 but height starts from 1.
    /// See [crate][binary_tree][Coordinate] for more details.
    pub fn from_y_coord(y_coord: u8) -> Self {
        match Self::try_from(y_coord + 1) {
            Ok(h) => h,
            Err(e) => {
                error!("Malformed input, error: {:?}", e);
                panic!("Malformed input, error: {:?}", e);
            }
        }
    }

    /// Return the y-coord for the given height.
    ///
    /// Why the offset? `y` starts from 0 but height starts from 1.
    /// See [crate][binary_tree][Coordinate] for more details.
    pub fn as_y_coord(&self) -> u8 {
        self.0 - 1
    }

    /// Return the underlying integer value.
    pub fn as_raw_int(&self) -> UnderlyingInt {
        self.0
    }

    /// Return the underlying integer value as type usize.
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }

    /// Return the underlying integer value as type u32.
    pub fn as_u32(&self) -> u32 {
        self.0 as u32
    }

    /// The maximum number of leaf nodes on the bottom layer of the binary tree.
    ///
    /// $$\text{max} = 2^{\text{height}-1}$$
    pub fn max_bottom_layer_nodes(&self) -> u64 {
        2u64.pow(self.as_u32() - 1)
    }
}

// -------------------------------------------------------------------------------------------------
// TryFrom for u8.

/// Create a [Height] object from `int`.
///
/// Returns an error if `int` is greater than [MAX_HEIGHT] or less than
/// [MIN_HEIGHT].
impl TryFrom<u8> for Height {
    type Error = HeightError;

    fn try_from(int: u8) -> Result<Self, Self::Error> {
        if int < MIN_HEIGHT.0 {
            Err(HeightError::InputTooSmall)
        } else if int > MAX_HEIGHT.0 {
            Err(HeightError::InputTooBig)
        } else {
            Ok(Height(int))
        }
    }
}

// -------------------------------------------------------------------------------------------------
// From for str.

use std::str::FromStr;

impl FromStr for Height {
    type Err = HeightError;

    /// Constructor that takes in a string slice.
    /// If the length of the str is greater than the max then Err is returned.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Height::try_from(UnderlyingInt::from_str(s)?)
    }
}

// -------------------------------------------------------------------------------------------------
// From for OsStr.

use clap::builder::{OsStr, Str};

impl From<Height> for OsStr {
    fn from(height: Height) -> OsStr {
        OsStr::from(Str::from(height.as_raw_int().to_string()))
    }
}

// -------------------------------------------------------------------------------------------------
// Default.

impl Default for Height {
    fn default() -> Self {
        Height(DEFAULT_HEIGHT)
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

#[derive(thiserror::Error, Debug)]
pub enum HeightError {
    #[error("Input is greater than the upper bound {MAX_HEIGHT:?}")]
    InputTooBig,
    #[error("Input is smaller than the lower bound {MIN_HEIGHT:?}")]
    InputTooSmall,
    #[error("Malformed string input for {UNDERLYING_INT_TYPE_STR:?} type")]
    MalformedString(#[from] std::num::ParseIntError),
}
