//! Entity structure and methods.
//!
//! The proof of liabilities protocol operates on a list of objects. Each object
//! must be of the same type, and the structure of this type is defined by the
//! entity struct. There is a 1-1 mapping from entity to bottom layer leaf node
//! in the binary tree.
//!
//! More often than not the data fed to the protocol is expected to be related
//! to people, or users. So an entity can be thought of as a user. 'Entity' was
//! chosen above 'user' because it has a more general connotation.
//!
//! The entity struct has only 2 fields: ID and liability.

use logging_timer::time;
use rand::{
    distributions::{Alphanumeric, DistString, Uniform},
    thread_rng, Rng,
};
use serde::Deserialize;

use std::convert::From;
use std::path::PathBuf;
use std::str::FromStr;

// -------------------------------------------------------------------------------------------------
// Main structs & implementations.

#[derive(Deserialize)]
pub struct Entity {
    pub liability: u64,
    pub id: EntityId,
}

/// The max size of the entity ID is 256 bits, but this is a soft limit so it
/// can be increased if necessary. Note that the underlying array length will
/// also have to be increased.
// TODO this is not enforced on deserialization, do that
const ENTITY_ID_MAX_BYTES: usize = 32;

/// Abstract representation of an entity ID.
#[derive(PartialEq, Eq, Hash, Clone, Debug, Deserialize)]
pub struct EntityId(String);

impl FromStr for EntityId {
    type Err = EntityParseError;

    /// Constructor that takes in a string slice.
    /// If the length of the str is greater than the max then Err is returned.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > ENTITY_ID_MAX_BYTES {
            Err(EntityParseError::EntityIdTooLongError { id: s.into() })
        } else {
            Ok(EntityId(s.into()))
        }
    }
}

impl From<EntityId> for Vec<u8> {
    /// Conversion to byte vector.
    fn from(item: EntityId) -> Vec<u8> {
        item.0.as_bytes().to_vec()
    }
}

// -------------------------------------------------------------------------------------------------
// Entity parser.

/// Parser for files containing many entity records.
///
/// Supported file types: csv
/// Note that the file type is inferred from its path extension.
///
/// CSV format: id,liability
pub struct EntitiesParser {
    file_path: PathBuf,
}

/// Supported file types for the parser.
enum FileType {
    Csv,
}

impl EntitiesParser {
    /// Constructor.
    pub fn from_path(file_path: PathBuf) -> Self {
        EntitiesParser { file_path }
    }

    /// Open and parse the file, returning a vector of entities.
    /// The file is expected to hold 1 or more entity records.
    ///
    /// An error is returned if:
    /// a) the file cannot be opened
    /// b) the file type is not supported
    /// c) deserialization of any of the records in the file fails
    pub fn parse(self) -> Result<Vec<Entity>, EntityParseError> {
        let ext = self
            .file_path
            .extension()
            .map(|s| s.to_str())
            .flatten()
            .ok_or(EntityParseError::UnknownFileType)?;

        let mut entities = Vec::<Entity>::new();

        match FileType::from_str(ext)? {
            FileType::Csv => {
                let mut reader = csv::Reader::from_path(self.file_path)?;

                for record in reader.deserialize() {
                    let entity: Entity = record?;
                    entities.push(entity);
                }
            }
        };

        Ok(entities)
    }
}

impl FromStr for FileType {
    type Err = EntityParseError;

    fn from_str(ext: &str) -> Result<FileType, Self::Err> {
        match ext {
            "csv" => Ok(FileType::Csv),
            _ => Err(EntityParseError::UnsupportedFileType { ext: ext.into() }),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Random entities generator.

const STRING_CONVERSION_ERR_MSG: &str = "A failure should not be possible here because the length of the random string exactly matches the max allowed length";

#[time("debug")]
pub fn generate_random_entities(num_leaves: u64) -> Vec<Entity> {
    let mut rng = thread_rng();
    let mut result = Vec::with_capacity(num_leaves as usize);

    let liability_range = Uniform::new(0u64, u64::MAX / num_leaves);

    for _i in 0..num_leaves {
        let liability = rng.sample(liability_range);
        let rand_str = Alphanumeric.sample_string(&mut rng, ENTITY_ID_MAX_BYTES);
        let id = EntityId::from_str(&rand_str).expect(STRING_CONVERSION_ERR_MSG);

        result.push(Entity { liability, id })
    }

    result
}

// -------------------------------------------------------------------------------------------------
// Errors.

#[derive(thiserror::Error, Debug)]
pub enum EntityParseError {
    #[error("Unable to find file extension")]
    UnknownFileType,
    #[error("The file type with extension {ext:?} is not supported")]
    UnsupportedFileType { ext: String },
    #[error("Error opening or reading CSV file")]
    CsvError(#[from] csv::Error),
    #[error(
        "The given entity ID ({id:?}) is longer than the max allowed {ENTITY_ID_MAX_BYTES} bytes"
    )]
    EntityIdTooLongError { id: String },
}

// -------------------------------------------------------------------------------------------------
// Unit tests

// TODO add more tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::Path;

    #[test]
    fn parser_csv_file_happy_case() {
        let src_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(&src_dir).join("entities_example.csv");
        EntitiesParser::from_path(path.into()).parse().unwrap();
    }
}
