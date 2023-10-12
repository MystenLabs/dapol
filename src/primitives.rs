use std::convert::From;
use std::str::FromStr;

use primitive_types::H256;

use crate::kdf::Key;

// -------------------------------------------------------------------------------------------------
// Secret data type.

const BITS_256: usize = 256;

/// 256-bit data packet.
///
/// The main purpose for this struct is to abstract away the [u8; 32] storage array and offer
/// functions for moving data as apposed to copying.
///
/// Currently there is no need for the functionality provided by something like
/// [primitive_types::U256 ] or [num256::Uint256] but those are options for later need be.
#[derive(Clone)]
pub struct Secret([u8; 32]);

impl From<Key> for Secret {
    fn from(key: Key) -> Self {
        let bytes: [u8; 32] = key.into();
        Secret(bytes)
    }
}

impl From<u64> for Secret {
    /// Constructor that takes in a u64.
    fn from(num: u64) -> Self {
        let bytes = num.to_le_bytes();
        let mut arr = [0u8; 32];
        for i in 0..8 {
            arr[i] = bytes[i]
        }
        Secret(arr)
    }
}

impl FromStr for Secret {
    type Err = StringTooLongError;

    /// Constructor that takes in a string slice.
    /// If the length of the str is greater than the max then Err is returned.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > BITS_256 {
            Err(StringTooLongError {})
        } else {
            let mut arr = [0u8; 32];
            // this works because string slices are stored fundamentally as u8 arrays
            arr[..s.len()].copy_from_slice(s.as_bytes());
            Ok(Secret(arr))
        }
    }
}

impl From<Secret> for [u8; 32] {
    fn from(item: Secret) -> Self {
        item.0
    }
}

impl Secret {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

#[derive(Debug)]
pub struct StringTooLongError;

// -------------------------------------------------------------------------------------------------
// H256 extensions.

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
