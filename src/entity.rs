//! Entity structure and methods.
//!
//! The proof of liabilities protocol operates on a list of objects. Each object
//! must be of the same type, and the structure of this type is defined by the
//! entity struct. There is a 1-1 mapping from entity to bottom layer leaf node in the binary tree.
//!
//! More often than not the data fed to the protocol is expected to be related
//! to people, or users. So an entity can be thought of as a user. 'Entity' was
//! chosen above 'user' because it has a more general connotation.
//!
//! The entity struct has only 2 fields: ID and liability.

use std::convert::From;
use std::str::FromStr;

// -------------------------------------------------------------------------------------------------
// Main structs.

pub struct Entity {
    pub liability: u64,
    pub id: EntityId,
}

/// The max size of the entity ID is 256 bits, but this is a soft limit so it
/// can be increased if necessary.
const ENTITY_ID_MAX_LENGTH: usize = 256;

/// Abstract representation of an entity ID.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct EntityId([u8; 32]);

// -------------------------------------------------------------------------------------------------
// Constructors.

impl FromStr for EntityId {
    type Err = EntityIdTooLongError;

    /// Constructor that takes in a string slice.
    /// If the length of the str is greater than the max then Err is returned.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > ENTITY_ID_MAX_LENGTH {
            Err(EntityIdTooLongError {})
        } else {
            let mut arr = [0u8; 32];
            // this works because string slices are stored fundamentally as u8 arrays
            arr[..s.len()].copy_from_slice(s.as_bytes());
            Ok(EntityId(arr))
        }
    }
}

impl From<EntityId> for [u8; 32] {
    fn from(item: EntityId) -> [u8; 32] {
        item.0
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

#[derive(Debug)]
pub struct EntityIdTooLongError;

// -------------------------------------------------------------------------------------------------
// Unit tests

// TODO

