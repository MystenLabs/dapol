//! Specifics for the Key Derivation Function (KDF).

use primitive_types::H256;
use std::convert::From;

// Currently the KDF is just the blake3 hash function.
pub struct KDF {
    // TODO need to find a better suited KDF implementation
    hasher: blake3::Hasher,
}

/// Output of the KDF.
pub struct Key([u8; 32]);

impl KDF {
    fn new() -> Self {
        KDF {
            hasher: blake3::Hasher::new(),
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    fn finalize(&self) -> Key {
        Key(self.hasher.finalize().into())
    }
}

impl From<Key> for [u8; 32] {
    fn from(key: Key) -> [u8; 32] {
        key.0
    }
}

pub fn generate_key(value1: &[u8], value2: &[u8]) -> Key {
    let mut kdf = KDF::new();
    kdf.update(value1);
    kdf.update(value2);
    kdf.finalize()
}
