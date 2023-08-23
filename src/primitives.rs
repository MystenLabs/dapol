use std::convert::From;
use primitive_types::H256;

use crate::kdf::Key;

// ======================================================
// D256 data type

/// 256-bit data packet.
///
/// The main purpose for this struct is to abstract away the [u8; 32] storage array and offer
/// functions for moving data as apposed to copying.
///
/// Currently there is no need for the functionality provided by something like
/// [primitive_types::U256 ] or [num256::Uint256] but those are options for later need be.
pub struct D256([u8; 32]);

impl From<Key> for D256 {
    fn from(key: Key) -> Self {
        let bytes: [u8; 32] = key.into();
        D256(bytes)
    }
}

impl From<u64> for D256 {
    // TODO is there a way to do this without copying? By taking ownership?
    fn from(num: u64) -> Self {
        let bytes = num.to_le_bytes();
        let mut arr = [0u8; 32];
        for i in 0..8 {
            arr[i] = bytes[i]
        }
        D256(arr)
    }
}

impl From<D256> for [u8; 32] {
    fn from(item: D256) -> Self {
        item.0
    }
}

impl D256 {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

// ======================================================
// H256 extensions

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
