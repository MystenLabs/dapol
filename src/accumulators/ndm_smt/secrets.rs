use std::{convert::TryFrom, fs::File, io::Read, path::PathBuf, str::FromStr};

use logging_timer::time;
use serde::{Deserialize, Serialize};

use crate::secret::{Secret, MAX_LENGTH_BYTES};

use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};

/// Values required for tree construction and inclusion proof generation.
///
/// The names of the secret values are exactly the same as the ones given in the
/// DAPOL+ paper.
// STENT TODO why is the deserialize trait here?
#[derive(Serialize, Deserialize)]
pub struct Secrets {
    pub master_secret: Secret,
    pub salt_b: Secret,
    pub salt_s: Secret,
}

/// This coding style is a bit ugly but it is the simplest way to get the
/// desired outcome, which is to deserialize string values into a byte array.
/// We can't deserialize automatically to [a secret] without a custom
/// implementation of the [deserialize trait]. Instead we deserialize to
/// [SecretsInput] and then convert the individual string fields to byte arrays.
///
/// [a secret] crate::secret::Secret
/// [deserialize trait] serde::Deserialize
// STENT TODO do we really need this?
#[derive(Deserialize)]
pub struct SecretsInput {
    master_secret: String,
    salt_b: String,
    salt_s: String,
}

/// Parser for files containing secrets.
///
/// Supported file types: toml
/// Note that the file type is inferred from its path extension.
///
/// TOML format:
/// ```toml,ignore
/// master_secret = "master_secret"
/// salt_b = "salt_b"
/// salt_s = "salt_s"
/// ```
const STRING_CONVERSION_ERR_MSG: &str = "A failure should not be possible here because the length of the random string exactly matches the max allowed length";

impl Secrets {
    #[time("debug", "NdmSmt::Secrets::{}")]
    pub fn generate_random() -> Self {
        let mut rng = thread_rng();
        let master_secret_str = Alphanumeric.sample_string(&mut rng, MAX_LENGTH_BYTES);
        let salt_b_str = Alphanumeric.sample_string(&mut rng, MAX_LENGTH_BYTES);
        let salt_s_str = Alphanumeric.sample_string(&mut rng, MAX_LENGTH_BYTES);

        Secrets {
            master_secret: Secret::from_str(&master_secret_str).expect(STRING_CONVERSION_ERR_MSG),
            salt_b: Secret::from_str(&salt_b_str).expect(STRING_CONVERSION_ERR_MSG),
            salt_s: Secret::from_str(&salt_s_str).expect(STRING_CONVERSION_ERR_MSG),
        }
    }
}

impl TryFrom<SecretsInput> for Secrets {
    type Error = super::secrets_parser::SecretsParseError;

    fn try_from(input: SecretsInput) -> Result<Secrets, Self::Error> {
        Ok(Secrets {
            master_secret: Secret::from_str(&input.master_secret)?,
            salt_b: Secret::from_str(&input.salt_b)?,
            salt_s: Secret::from_str(&input.salt_s)?,
        })
    }
}
