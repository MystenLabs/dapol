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

    pub fn with_path_opt(mut self, path: Option<PathBuf>) -> Self {
        self.path = path;
        self
    }

    pub fn with_path(self, path: PathBuf) -> Self {
        self.with_path_opt(Some(path))
    }

    pub fn with_num_entities_opt(mut self, num_entities: Option<u64>) -> Self {
        self.num_entities = num_entities;
        self
    }

    pub fn with_num_entities(self, num_entities: u64) -> Self {
        self.with_num_entities_opt(Some(num_entities))
    }

    /// Open and parse the file, returning a vector of entities.
    /// The file is expected to hold 1 or more entity records.
    ///
    /// An error is returned if:
    /// a) the file cannot be opened
    /// b) the file type is not supported
    /// c) deserialization of any of the records in the file fails
    #[time("debug")]
    pub fn parse_file(self) -> Result<Vec<Entity>, EntitiesParserError> {
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

    /// Generate a vector of entities with random IDs & liabilities.
    ///
    /// A cryptographic pseudo-random number generator is used to generate the
    /// data. `num_entities` determines the length of the vector.
    ///
    /// An error is returned if `num_entities` is not set.
    #[time("debug")]
    pub fn generate_random(self) -> Result<Vec<Entity>, EntitiesParserError> {
        let num_entities = self
            .num_entities
            .ok_or(EntitiesParserError::NumEntitiesNotSet)?;

        let mut rng = thread_rng();
        let mut result = Vec::with_capacity(num_entities as usize);

        let liability_range = Uniform::new(0u64, u64::MAX / num_entities);

        for _i in 0..num_entities {
            let liability = rng.sample(liability_range);
            let rand_str = Alphanumeric.sample_string(&mut rng, ENTITY_ID_MAX_BYTES);
            let id = EntityId::from_str(&rand_str).expect("A failure should not be possible here because the length of the random string exactly matches the max allowed length");

            result.push(Entity { liability, id })
        }

        Ok(result)
    }

    /// If a file path is present then parse the file, otherwise generate
    /// entity records randomly. The number of entity records generated must
    /// be provided.
    ///
    /// Errors are returned if:
    /// a) a file is present and [parse] gives an error
    /// b) neither a file nor a number of entities are present
    pub fn parse_file_or_generate_random(self) -> Result<Vec<Entity>, EntitiesParserError> {
        if self.path.is_some() {
            self.parse_file()
        } else {
            info!("No entity file provided, defaulting to generating random entities");
            self.generate_random()
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
// Unit tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parser_csv_file_happy_case() {
        let src_dir = env!("CARGO_MANIFEST_DIR");
        let path = Path::new(&src_dir).join("entities_example.csv");

        let entities = EntitiesParser::new().with_path(path).parse_file().unwrap();

        let first_entity = Entity {
            id: EntityId::from_str("john.doe@example.com").unwrap(),
            liability: 893267u64,
        };

        let last_entity = Entity {
            id: EntityId::from_str("david.martin@example.com").unwrap(),
            liability: 142798u64,
        };

        assert!(entities.contains(&first_entity));
        assert!(entities.contains(&last_entity));

        assert_eq!(entities.len(), 100);
    }

    // TODO fuzz on num entities
    #[test]
    fn generate_random_entities_happy_case() {
        let num_entities = 99;
        let entities = EntitiesParser::new()
            .with_num_entities(num_entities)
            .generate_random()
            .unwrap();
        assert_eq!(entities.len(), num_entities as usize);
    }
}
