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
use log::error;

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
/// HKDF requires 3 inputs: salt, initial key material (IKM), info. Both the
/// `salt` and `info` parameters and optional. The reason for this is that the
/// DAPOL paper only specifies 2 inputs to its KDF, but the HKDF takes 3 inputs.
/// In some of the cases `salt` is preferred, and in some `info` is. At least
/// one of `salt` or `info` must be set, otherwise the function will panic;
/// since this state is a potential security vulnerability, and should only be
/// reachable if there is a bug in the code, a panic is the best option.
pub fn generate_key(salt: Option<&[u8]>, ikm: &[u8], info: Option<&[u8]>) -> Key {
    if salt.is_none() && info.is_none() {
        error!("At least one of salt/info must be set when using the KDF to generate keys");
        panic!("At least one of salt/info must be set when using the KDF to generate keys");
    }

    let hk = Hkdf::<Sha256>::new(salt, ikm);
    let mut okm = [0u8; 32];
    hk.expand(info.unwrap_or_default(), &mut okm)
        .expect("32 is a valid byte length for Sha256 to output");

    Key(okm)
}
