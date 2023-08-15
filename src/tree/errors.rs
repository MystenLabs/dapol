use crate::tree::sparse_binary_tree::{Coordinate, InclusionProofError};

pub type TreeResult<T> = ::core::result::Result<T, TreeError>;

#[derive(Debug)]
pub enum TreeError {
    MissingNode(Coordinate),
    CorruptedInclusionProof(InclusionProofError),
    // CorruptedProof,
    // EmptyProof,
    // EmptyKeys,
    // IncorrectNumberOfLeaves { expected: usize, actual: usize },
    // Store(string::String),
    // CorruptedStack,
    // NonSiblings,
    // InvalidCode(u8),
    // NonMergableRange,
}
