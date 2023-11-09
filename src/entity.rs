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

use serde::{Deserialize, Serialize};

use std::{convert::From};
use std::str::FromStr;

mod entities_parser;
pub use entities_parser::{EntitiesParser, EntitiesParserError};

mod entity_ids_parser;
pub use entity_ids_parser::{EntityIdsParser, EntityIdsParserError};

// -------------------------------------------------------------------------------------------------
// Main structs & implementations.

#[derive(Deserialize, PartialEq)]
pub struct Entity {
    pub liability: u64,
    pub id: EntityId,
}

/// The max size of the entity ID is 256 bits, but this is a soft limit so it
/// can be increased if necessary. Note that the underlying array length will
/// also have to be increased.
// STENT TODO this is not enforced on deserialization, do that
pub const ENTITY_ID_MAX_BYTES: usize = 32;

/// Abstract representation of an entity ID.
#[derive(PartialEq, Eq, Hash, Clone, Debug, Deserialize, Serialize)]
pub struct EntityId(String);

impl FromStr for EntityId {
    type Err = EntitiesParserError;

    /// Constructor that takes in a string slice.
    /// If the length of the str is greater than the max then Err is returned.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > ENTITY_ID_MAX_BYTES {
            Err(EntitiesParserError::EntityIdTooLongError { id: s.into() })
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

use std::fmt;

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
