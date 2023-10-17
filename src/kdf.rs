//! Specifics for the Key Derivation Function (KDF).
//!
//! HKDF is used, with the SHA256 hash function.
//!
//! The HKDF is split into 2 separate functions: extract & expand (both of which
//! utilize HMAC).
//!
//! `HKDF(salt, IKM, info, length) = HKDF-Expand(HKDF-Extract(salt, IKM), info, length)`
//! where `HKDF-Extract(salt, IKM) = HMAC(key=salt, message=IKM)`
//!
//! For more information check out these resources:
//! - [Cryptographic Extraction and Key Derivation: The HKDF Scheme](https://eprint.iacr.org/2010/264.pdf)
//! - [Wikipedia entry for HKDF](https://en.wikipedia.org/wiki/HKDF)

use hkdf::Hkdf;
use sha2::Sha256;
use std::convert::From;

/// Output of the KDF.
///
/// The output is 256 bits but this can be adjusted. If the size is adjusted the
/// hash function may need to change too.
pub struct Key([u8; 32]);

impl From<Key> for [u8; 32] {
    fn from(key: Key) -> [u8; 32] {
        key.0
    }
}

/// Use the KDF to generate a [Key].
///
/// HKDF requires 3 inputs: salt, IKM, info.
pub fn generate_key(ikm: &[u8], info: &[u8]) -> Key {
    let hk = Hkdf::<Sha256>::new(None, &ikm);
    let mut okm = [0u8; 32];
    hk.expand(&info, &mut okm)
        .expect("32 is a valid byte length for Sha256 to output");

    Key(okm)
}
