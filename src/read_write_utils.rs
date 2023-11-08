use std::fmt::Debug;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::{ffi::OsString, fs::File};

use log::error;
use logging_timer::{executing, finish, stime, stimer, Level};
use serde::{de::DeserializeOwned, Serialize};

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
    let tmr = stimer!(Level::Debug; "Serialization");

    let encoded: Vec<u8> = bincode::serialize(&structure)?;
    executing!(tmr, "Done encoding");

    let mut file = File::create(path)?;
    file.write_all(&encoded)?;
    finish!(tmr, "Done writing file");

    Ok(())
}

/// Try to deserialize the given binary file to the specified type.
///
/// The file is assumed to be in [bincode] format.
///
/// An error is returned if
/// 1. The file cannot be opened.
/// 2. The [bincode] deserializer fails.
#[stime("debug")]
pub fn deserialize_from_bin_file<T: DeserializeOwned>(path: PathBuf) -> Result<T, ReadWriteError> {
    let file = File::open(path)?;
    let buf_reader = BufReader::new(file);
    let decoded: T = bincode::deserialize_from(buf_reader)?;
    Ok(decoded)
}

/// Parse `path` as one that points to a file that will be used for
/// serialization.
///
/// `path` can be either of the following:
/// 1. Existing directory: in this case a default file name is appended to
/// `path`. 2. Non-existing directory: in this case all dirs in the path are
/// created, and a default file name is appended.
/// 3. File in existing dir: in this case the extension is checked to be
/// `expected_extension`, then `path` is returned.
/// 4. File in non-existing dir: dirs in the path are created and the file
/// extension is checked.
///
/// The default file name is `default_file_name_prefix + "_" + <timestamp> + "."
/// + extension`.
///
/// Example:
/// ```
/// use dapol::read_write_utils::parse_serialization_path;
/// use std::path::PathBuf;
///
/// let extension = "test";
/// let default_file_name_prefix = "file_prefix";
/// let dir = PathBuf::from("./");
///
/// let path = parse_serialization_path(dir, extension, default_file_name_prefix).unwrap();
/// ```
pub fn parse_serialization_path(
    mut path: PathBuf,
    extension: &str,
    default_file_name_prefix: &str,
) -> Result<PathBuf, ReadWriteError> {
    if let Some(ext) = path.extension() {
        // If `path` leads to a file.

        if ext != extension {
            return Err(ReadWriteError::UnsupportedFileExtension {
                expected: extension.to_owned(),
                actual: ext.to_os_string(),
            });
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

        let mut file_name: String = default_file_name_prefix.to_owned();
        let now = chrono::offset::Local::now();
        file_name.push_str(&now.timestamp().to_string());
        file_name.push_str(".");
        file_name.push_str(extension);
        path.push(file_name);

        Ok(path)
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

#[derive(thiserror::Error, Debug)]
pub enum ReadWriteError {
    #[error("Problem serializing/deserializing with bincode")]
    BincodeSerdeError(#[from] bincode::Error),
    #[error("Problem writing to file")]
    FileWriteError(#[from] std::io::Error),
    #[error("Unknown file extension {actual:?}, expected {expected}")]
    UnsupportedFileExtension { expected: String, actual: OsString },
    #[error("Expected a file but only a path was given: {0:?}")]
    NotAFile(OsString),
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

#[cfg(test)]
mod tests {
    mod parse_serialization_path {
        use super::super::*;

        #[test]
        fn parse_serialization_path_for_existing_directory_gives_correct_file_name() {
            let path = PathBuf::from("./");
            let expected_extension = "test";
            let default_file_name_prefix = "test_prefix";

            let path = parse_serialization_path(path, expected_extension, default_file_name_prefix)
                .unwrap();

            let ext = path.extension().unwrap().to_str().unwrap();
            assert_eq!(ext, expected_extension);

            let file_name_without_extension = path.file_stem().unwrap().to_str().unwrap();
            assert!(file_name_without_extension.contains(default_file_name_prefix));
        }

        #[test]
        fn parse_serialization_path_for_existing_file() {
            let this_file = std::file!();
            let path = PathBuf::from(this_file);
            let expected_extension = "rs";
            let default_file_name_prefix = "test_prefix";

            parse_serialization_path(path, expected_extension, default_file_name_prefix).unwrap();
        }

        #[test]
        #[should_panic]
        fn parse_serialization_path_for_existing_file_wrong_extension() {
            let this_file = std::file!();
            let path = PathBuf::from(this_file);
            let expected_extension = "bad_ext";
            let default_file_name_prefix = "test_prefix";

            parse_serialization_path(path, expected_extension, default_file_name_prefix).unwrap();
        }

        // TODO test that intermediate dirs are created, but how to do this
        // without actually creating dirs?
    }
}
