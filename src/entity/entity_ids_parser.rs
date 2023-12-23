use std::str::FromStr;
use std::{ffi::OsString, path::PathBuf};

use log::debug;

use crate::entity::{EntityId, ENTITY_ID_MAX_BYTES};

/// Parser for files containing a list of entity IDs.
///
/// The entity IDs file is expected to be a list of entity IDs, each on a new
/// line.    All file formats are accepted. It is also possible to use the same
/// entity IDs &    liabilities file that is accepted by
/// [crate][EntitiesParser].
///
/// Example:
/// ```
/// use dapol::EntityIdsParser;
/// use std::path::PathBuf;
///
/// let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
/// path.push("./examples/entities_example.csv");
/// let entities = EntityIdsParser::from(path).parse().unwrap();
/// ```
pub struct EntityIdsParser {
    path: Option<PathBuf>,
}

/// Supported file types for the parser.
enum FileType {
    Csv,
}

impl EntityIdsParser {
    /// Open and parse the file, returning a vector of entity IDs.
    /// The file is expected to hold 1 or more entity records.
    ///
    /// An error is returned if:
    /// a) the file cannot be opened
    /// b) the file type is not supported
    /// c) deserialization of any of the records in the file fails
    pub fn parse(self) -> Result<Vec<EntityId>, EntityIdsParserError> {
        debug!(
            "Attempting to parse {:?} as a file containing a list of entity IDs",
            &self.path
        );

        let path = self.path.ok_or(EntityIdsParserError::PathNotSet)?;

        let ext = path.extension().and_then(|s| s.to_str()).ok_or(
            EntityIdsParserError::UnknownFileType(path.clone().into_os_string()),
        )?;

        let mut entity_ids = Vec::<EntityId>::new();

        match FileType::from_str(ext)? {
            FileType::Csv => {
                let mut reader = csv::Reader::from_path(path)?;

                for record in reader.deserialize() {
                    let entity_id: EntityId = record?;
                    entity_ids.push(entity_id);
                }
            }
        };

        debug!("Successfully parsed entity IDs file",);

        Ok(entity_ids)
    }
}

impl From<PathBuf> for EntityIdsParser {
    fn from(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }
}

impl FromStr for FileType {
    type Err = EntityIdsParserError;

    fn from_str(ext: &str) -> Result<FileType, Self::Err> {
        match ext {
            "csv" => Ok(FileType::Csv),
            _ => Err(EntityIdsParserError::UnsupportedFileType { ext: ext.into() }),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

/// Errors encountered when handling [EntityIdsParser].
#[derive(thiserror::Error, Debug)]
pub enum EntityIdsParserError {
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
        let resources_dir = Path::new(&src_dir).join("examples");
        let path = resources_dir.join("entities_example.csv");

        let entities = EntityIdsParser::from(path).parse().unwrap();

        let first_entity = EntityId::from_str("john.doe@example.com").unwrap();

        let last_entity = EntityId::from_str("david.martin@example.com").unwrap();

        assert!(entities.contains(&first_entity));
        assert!(entities.contains(&last_entity));

        assert_eq!(entities.len(), 100);
    }
}
