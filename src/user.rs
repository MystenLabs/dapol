use std::convert::From;
use std::str::FromStr;

const USER_ID_MAX_LENGTH: usize = 256;

/// Abstract representation of a user ID.
/// For now the max size of the user ID is 256 bits.
pub struct UserId([u8; 32]);

impl FromStr for UserId {
    type Err = UserIdTooLongError;

    /// Constructor that takes in a slice.
    /// If the length of the str is greater than the max then Err is returned.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > USER_ID_MAX_LENGTH {
            Err(UserIdTooLongError {})
        } else {
            let mut arr = [0u8; 32];
            // this works because string slices are stored fundamentally as u8 arrays
            arr[..s.len()].copy_from_slice(s.as_bytes());
            Ok(UserId(arr))
        }
    }
}

impl From<UserId> for [u8; 32] {
    fn from(item: UserId) -> [u8; 32] {
        item.0
    }
}

#[derive(Debug)]
pub struct UserIdTooLongError;
