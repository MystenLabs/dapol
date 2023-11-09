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
use log::error;
use sha2::Sha256;
use std::convert::From;

// -------------------------------------------------------------------------------------------------
// Main struct & implementation.

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
/// HKDF requires 3 inputs: salt, Initial Key Material (IKM), info. Both the
/// `salt` and `info` parameters and optional. The reason for this is that the
/// DAPOL paper only specifies 2 inputs to its KDF, but the HKDF takes 3 inputs.
/// In some of the cases `salt` is preferred, and in some `info` is. At least
/// one of `salt` or `info` must be set, otherwise the function will panic;
/// since this state is a potential security vulnerability, and should only be
/// reachable if there is a bug in the code, a panic is the best option.
///
/// The Output Key Material (OKM) is returned as a [Key] type.
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

// -------------------------------------------------------------------------------------------------
// Unit tests.

#[cfg(test)]
mod tests {
    use super::*;

    // The following tool was used as a comparison: https://asecuritysite.com/encryption/HKDF
    // These were the parameters used:
    // - Passphrase: hello
    // - Salt: 877a0e600574c903bec992ba508a61dc
    // - Info: cf0d57a2f9a2f9
    // - Key length: 32
    // - Hash function: Sha256
    #[test]
    fn generate_key_matches_external_tool() {
        let ikm = b"hello";
        let info: [u8; 7] = [0xcf, 0x0d, 0x57, 0xa2, 0xf9, 0xa2, 0xf9];
        let salt: [u8; 16] = [
            0x87, 0x7a, 0x0e, 0x60, 0x05, 0x74, 0xc9, 0x03, 0xbe, 0xc9, 0x92, 0xba, 0x50, 0x8a,
            0x61, 0xdc,
        ];
        let expected_okm: [u8; 32] = [
            0x32, 0x1c, 0x30, 0x53, 0x26, 0xd9, 0x14, 0x94, 0xb9, 0x81, 0x1f, 0x54, 0x33, 0xaa,
            0xb2, 0xf8, 0x79, 0x44, 0xd5, 0x49, 0xa3, 0x18, 0xee, 0x1b, 0xdf, 0xc2, 0xcb, 0xe3,
            0x19, 0xc5, 0x39, 0x85,
        ];

        let key = generate_key(Some(&salt), ikm, Some(&info));
        assert_eq!(key.0, expected_okm);
    }
}
