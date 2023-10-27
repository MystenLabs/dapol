//! Parser for files containing a list of entity IDs.
//!
//! Supported file types: csv
//! Note that the file type is inferred from its path extension.
//!
//! Formatting:
//! CSV: `id`

use std::path::PathBuf;
use std::str::FromStr;

use log::info;

use crate::entity::{EntityId, ENTITY_ID_MAX_BYTES};

pub struct EntityIdsParser {
    path: Option<PathBuf>,
}

/// Supported file types for the parser.
enum FileType {
    Csv,
}

impl EntityIdsParser {
    pub fn from_path(path: Option<PathBuf>) -> Self {
        EntityIdsParser { path: None }
    }

    /// Open and parse the file, returning a vector of entity IDs.
    /// The file is expected to hold 1 or more entity records.
    ///
    /// An error is returned if:
    /// a) the file cannot be opened
    /// b) the file type is not supported
    /// c) deserialization of any of the records in the file fails
    pub fn parse(self) -> Result<Vec<EntityId>, EntityIdsParserError> {
        info!(
            "Attempting to parse {:?} as a file containing a list of entity IDs",
            &self.path
        );

        let path = self.path.ok_or(EntityIdsParserError::PathNotSet)?;

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or(EntityIdsParserError::UnknownFileType)?;

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

        Ok(entity_ids)
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

#[derive(thiserror::Error, Debug)]
pub enum EntityIdsParserError {
    #[error("Expected path to be set but found none")]
    PathNotSet,
    #[error("Expected num_entities to be set but found none")]
    NumEntitiesNotSet,
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

