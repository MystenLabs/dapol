use std::fmt::{Debug};
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::{ffi::OsString, fs::File};

use log::error;
use logging_timer::{executing, finish, stimer, Level, stime};
use serde::{de::DeserializeOwned, Serialize};

pub const SERIALIZED_TREE_EXTENSION: &str = "dapoltree";

// -------------------------------------------------------------------------------------------------
// Utility functions.

/// Use [bincode] to serialize `structure` to a file at the given `path`.
///
/// An error is returned if
/// 1. [bincode] fails to serialize the file.
/// 2. There is an issue opening or writing the file.
///
/// Turning on debug-level logs will show timing.
pub fn serialize_to_bin_file<T: Serialize>(
    structure: &T,
    path: PathBuf,
) -> Result<(), ReadWriteError> {
    let tmr = stimer!(Level::Info; "Serialization");

    let encoded: Vec<u8> = bincode::serialize(&structure)?;
    executing!(tmr, "Done encoding");

    let mut file = File::create(path)?;
    file.write_all(&encoded)?;
    finish!(tmr, "Done writing file");

    Ok(())
}

/// Try to deserialize the given file to the specified type.
///
/// The file is assumed to be in [bincode] format.
///
/// An error is returned if
/// 1. The file cannot be opened.
/// 2. The [bincode] deserializer fails.
#[stime("info")]
pub fn deserialize_from_bin_file<T: DeserializeOwned>(path: PathBuf) -> Result<T, ReadWriteError> {
    let file = File::open(path)?;
    let buf_reader = BufReader::new(file);
    let decoded: T = bincode::deserialize_from(buf_reader)?;
    Ok(decoded)
}

/// Parse `path` as one that points to a serialized tree file.
///
/// `path` can be either of the following:
/// 1. Existing directory: in this case a default file name is appended to `path`.
/// 2. Non-existing directory: in this case all dirs in the path are created,
/// and a default file name is appended.
/// 3. File in existing dir: in this case the extension is checked to be
/// '.dapoltree', then `path` is returned.
/// 4. File in non-existing dir: dirs in the path are created and the file
/// extension is checked.
///
/// The default file name is `ndm_smt_<timestamp>.dapoltree`.
pub fn parse_tree_serialization_path(mut path: PathBuf) -> Result<PathBuf, ReadWriteError> {
    if let Some(ext) = path.extension() {
        // If `path` leads to a file.

        if ext != SERIALIZED_TREE_EXTENSION {
            return Err(ReadWriteError::UnsupportedTreeExtension(ext.to_os_string()));
        }

        if let Some(parent) = path.parent() {
            if !parent.is_dir() {
                // Create any intermediate, non-existent directories.
                std::fs::create_dir_all(parent)?;
            }
        }

        Ok(path)
    } else {
        // If `path` is a directory.

        if !path.is_dir() {
            // Create any intermediate, non-existent directories.
            std::fs::create_dir_all(path.clone())?;
        }

        // STENT TODO we need this tree name to be generic
        let mut file_name: String = "ndm_smt_".to_owned();
        let now = chrono::offset::Local::now();
        file_name.push_str(&now.timestamp().to_string());
        file_name.push_str(SERIALIZED_TREE_EXTENSION);
        path.push(file_name);

        Ok(path)
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReadWriteError {
    #[error("Problem serializing with bincode")]
    SerializationError(#[from] bincode::Error),
    #[error("Problem writing to file")]
    FileWriteError(#[from] std::io::Error),
    #[error("Unknown file extension {0:?}, expected {SERIALIZED_TREE_EXTENSION}")]
    UnsupportedTreeExtension(OsString),
}
