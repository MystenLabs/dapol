//! Legacy

use rand::prelude::*;
use smtree::{
    error::DecodingError,
    pad_secret::{Secret, SECRET_LENGTH},
    utils::bytes_to_usize,
};

pub fn bytes_to_usize_with_error(
    bytes: &[u8],
    byte_num: usize,
    begin: &mut usize,
) -> Result<usize, DecodingError> {
    if bytes.len() < byte_num {
        return Err(DecodingError::BytesNotEnough);
    }
    bytes_to_usize(bytes, byte_num, begin)
}

pub fn get_secret() -> Secret {
    // building secret from bytes here because DAPOL and smtree use different
    // version of rand crate.
    // TODO: update dependencies to use the same version of rand.
    let mut rng = rand::thread_rng();
    let bytes: [u8; SECRET_LENGTH] = rng.gen();
    Secret::from_bytes(&bytes).expect("failed to build secret from bytes")
}
