//! Hash function.
//!
//! The main purpose of the hash function is usage in the binary tree merge
//! function. The reason it has it's own file is so that we can create a
//! wrapper around the underlying hash function, allowing it to be easily
//! changed.
//!
//! The current hash function used is blake3.
//!
//! Example:
//! ```
//! use dapol::Hasher;
//! let mut hasher = Hasher::new();
//! hasher.update("leaf".as_bytes());
//! let hash = hasher.finalize();
//! ```

use primitive_types::H256;

pub struct Hasher(blake3::Hasher);

impl Hasher {
    pub fn new() -> Self {
        Hasher(blake3::Hasher::new())
    }

    pub fn update(&mut self, input: &[u8]) -> &mut Self {
        self.0.update(input);
        self
    }

    pub fn finalize(&self) -> H256 {
        let bytes: [u8; 32] = self.0.finalize().into();
        H256(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensures Blake 3 library produces correct hashed output.
    // Comparison hash derived through the following urls:
    // https://toolkitbay.com/tkb/tool/BLAKE3
    // https://connor4312.github.io/blake3/index.html
    // https://asecuritysite.com/hash/blake3
    #[test]
    fn verify_hasher() {
        use std::str::FromStr;

        let mut hasher = Hasher::new();
        hasher.update("dapol-PoR".as_bytes());
        let hash = hasher.finalize();
        assert_eq!(
            hash,
            H256::from_str("e4bf4e238e74eb8d253191a56b594565514201a71373c86e304628ed623c4850")
                .unwrap()
        );
    }
}
