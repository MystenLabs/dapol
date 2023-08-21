//! Specifics for the Key Derivation function (KDF).
//!
//! Currently the KDF is just the blake3 hash function.
//! TODO need to find a better suited KDF implementation.

use primitive_types::H256;

struct KDF {
    hasher: blake3::Hasher,
}

impl KDF {
    fn new() -> Self {
        KDF {
            hasher: blake3::Hasher::new(),
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    fn finalize_as_h256(&self) -> H256 {
        H256(self.hasher.finalize().as_bytes().clone())
    }
}
