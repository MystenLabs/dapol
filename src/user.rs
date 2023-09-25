use std::convert::From;
use std::str::FromStr;

use thiserror::Error;

/// Main struct containing all user information.
/// There is a 1-1 mapping from user to bottom layer leaf node in the tree.
pub struct User {
    liability: u64,
    id: UserId,
}

/// For now the max size of the user ID is 256 bits.
const USER_ID_MAX_LENGTH: usize = 256;

static mut USER_IDS: Vec<UserId> = Vec::new();

/// Abstract representation of a user ID.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct UserId([u8; 32]);

#[derive(Error, Debug)]
pub enum UserIdError {
    #[error("Input string too long")]
    UserIdTooLong,
    #[error("User ID alrady exists")]
    UserIdAlreadyExists,
    #[error("User ID vector is full")]
    UserIdVectorOverflow,
}

impl FromStr for UserId {
    type Err = UserIdError;

    /// Constructor that takes in a string slice.
    /// If the length of the str is greater than the max then Err is returned.
    fn from_str(s: &str) -> Result<Self, UserIdError> {
        let mut arr = [0u8; 32];

        if s.len() > USER_ID_MAX_LENGTH {
            return Err(UserIdError::UserIdTooLong);
        } else {
            // this works because string slices are stored fundamentally as u8 arrays
            arr[..s.len()].copy_from_slice(s.as_bytes());
        }

        let user_id = UserId(arr);

        if user_id_exists(&user_id) {
            return Err(UserIdError::UserIdAlreadyExists);
        }

        Ok(user_id)
    }
}

impl User {
    pub fn build(liability: u64, id: UserId) -> Result<User, UserIdError> {
        if user_id_exists(&id) {
            return Err(UserIdError::UserIdAlreadyExists);
        } else {
            push_user_id(id.clone())?;
        }

        Ok(User { liability, id })
    }

    pub fn liability(&self) -> u64 {
        self.liability
    }

    pub fn id(&self) -> UserId {
        self.id.clone()
    }
}

impl From<UserId> for [u8; 32] {
    fn from(item: UserId) -> [u8; 32] {
        item.0
    }
}

fn push_user_id(id: UserId) -> Result<(), UserIdError> {
    unsafe {
        if user_ids().len() < isize::MAX.try_into().unwrap() {
            USER_IDS.push(id);
        } else {
            return Err(UserIdError::UserIdVectorOverflow);
        }
    }

    Ok(())
}

fn user_id_exists(id: &UserId) -> bool {
    if user_ids().contains(id) {
        true
    } else {
        false
    }
}

pub fn user_ids() -> &'static Vec<UserId> {
    unsafe { &USER_IDS }
}
