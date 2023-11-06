//! Parser for files containing a list of entity records.
//!
//! Supported file types: csv
//! Note that the file type is inferred from its path extension.
//!
//! Formatting:
//! CSV: `id,liability`
//!
//! Fields:
//! - `path`: path to the file containing the entity records
//! - `num_entities`: number of entities to be randomly generated
//!
//! At least on of the 2 fields must be set for the parser to succeed. If both
//! fields are set then the path is prioritized.

use std::{ffi::OsString, path::PathBuf, str::FromStr};

use rand::{
    distributions::{Alphanumeric, DistString, Uniform},
    thread_rng, Rng,
};

use log::info;
use logging_timer::time;

use super::{Entity, EntityId, ENTITY_ID_MAX_BYTES};

pub struct EntitiesParser {
    path: Option<PathBuf>,
    num_entities: Option<u64>,
}

/// Supported file types for the parser.
enum FileType {
    Csv,
}

impl EntitiesParser {
    pub fn new() -> Self {
        EntitiesParser {
            path: None,
            num_entities: None,
        }
    }

    pub fn with_path(mut self, path: Option<PathBuf>) -> Self {
        self.path = path;
        self
    }

    pub fn with_num_entities(mut self, num_entities: Option<u64>) -> Self {
        self.num_entities = num_entities;
        self
    }

    /// Open and parse the file, returning a vector of entities.
    /// The file is expected to hold 1 or more entity records.
    ///
    /// An error is returned if:
    /// a) the file cannot be opened
    /// b) the file type is not supported
    /// c) deserialization of any of the records in the file fails
    pub fn parse(self) -> Result<Vec<Entity>, EntitiesParserError> {
        info!(
            "Attempting to parse {:?} as a file containing a list of entity IDs and liabilities",
            &self.path
        );

        let path = self.path.ok_or(EntitiesParserError::PathNotSet)?;

        let ext = path.extension().and_then(|s| s.to_str()).ok_or(
            EntitiesParserError::UnknownFileType(path.clone().into_os_string()),
        )?;

        let mut entities = Vec::<Entity>::new();

        match FileType::from_str(ext)? {
            FileType::Csv => {
                let mut reader = csv::Reader::from_path(path)?;

                for record in reader.deserialize() {
                    let entity: Entity = record?;
                    entities.push(entity);
                }
            }
        };

        Ok(entities)
    }

    /// If a file path is present then parse the file, otherwise generate
    /// entity records randomly. The number of entity records generated must
    /// be provided.
    ///
    /// Errors are returned if:
    /// a) a file is present and [parse] gives an error
    /// b) neither a file nor a number of entities are present
    pub fn parse_or_generate_random(self) -> Result<Vec<Entity>, EntitiesParserError> {
        match &self.path {
            Some(_) => self.parse(),
            None => {
                info!("No entity file provided, defaulting to generating random entities");
                Ok(generate_random_entities(
                    self.num_entities
                        .ok_or(EntitiesParserError::NumEntitiesNotSet)?,
                ))
            }
        }
    }
}

impl FromStr for FileType {
    type Err = EntitiesParserError;

    fn from_str(ext: &str) -> Result<FileType, Self::Err> {
        match ext {
            "csv" => Ok(FileType::Csv),
            _ => Err(EntitiesParserError::UnsupportedFileType { ext: ext.into() }),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

#[derive(thiserror::Error, Debug)]
pub enum EntitiesParserError {
    #[error("Expected path to be set but found none")]
    PathNotSet,
    #[error("Expected num_entities to be set but found none")]
    NumEntitiesNotSet,
    #[error("Unable to find file extension for path {0:?}")]
    UnknownFileType(OsString),
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
// Random entities generator.

const STRING_CONVERSION_ERR_MSG: &str = "A failure should not be possible here because the length of the random string exactly matches the max allowed length";

/// Generate a vector of entities with random IDs & liabilities.
/// A cryptographic pseudo-random number generator is used to generate the data.
/// `num_leaves` determines the length of the vector.
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
        EntitiesParser::new()
            .with_path(path.into())
            .parse()
            .unwrap();
    }
}
