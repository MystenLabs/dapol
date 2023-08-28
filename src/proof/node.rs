//! Legacy

use curve25519_dalek_ng::{ristretto::CompressedRistretto, ristretto::RistrettoPoint};
use digest::Digest;
use smtree::{
    error::DecodingError,
    traits::{Mergeable, Serializable, TypeName},
};
use std::marker::PhantomData;

const COM_BYTE_NUM: usize = 32;

// DAPOL PROOF NODE
// ================================================================================================

/// A node of the DAPOL proof, consisting of the Pedersen commitment and the hash.
#[derive(Default, Clone, Debug)]
pub struct DapolProofNode<D> {
    com: RistrettoPoint,                    // The Pedersen commitment.
    hash: Vec<u8>,                          // The hash.
    _phantom_hash_function: PhantomData<D>, // The hash function.
}

impl<D> PartialEq for DapolProofNode<D> {
    /// Two proof nodes are equal iff both the commitments and the hashes are the same.
    fn eq(&self, other: &Self) -> bool {
        self.com == other.com && self.hash == other.hash
    }
}
impl<D> Eq for DapolProofNode<D> {}

impl<D> DapolProofNode<D> {
    /// The constructor.
    pub fn new(com: RistrettoPoint, hash: Vec<u8>) -> DapolProofNode<D> {
        DapolProofNode {
            com,
            hash,
            _phantom_hash_function: PhantomData,
        }
    }

    /// Returns the Pedersen commitment.
    pub fn get_com(&self) -> RistrettoPoint {
        self.com
    }

    /// Returns the hash.
    pub fn get_hash(&self) -> &Vec<u8> {
        &self.hash
    }
}

impl<D: Digest> Mergeable for DapolProofNode<D> {
    /// Returns the parent node by merging two child nodes.
    ///
    /// The commitment of the parent is the homomorphic sum of the two children.
    /// The hash of the parent is computed by hashing the concatenated commitments and hashes of two children.
    fn merge(lch: &DapolProofNode<D>, rch: &DapolProofNode<D>) -> DapolProofNode<D> {
        // C(parent) = C(L) + C(R)
        let com = lch.com + rch.com;

        // H(parent) = Hash(C(L) || C(R) || H(L) || H(R))
        let mut hasher = D::new();
        hasher.update(&(lch.com.compress().as_bytes().to_vec()));
        hasher.update(&(rch.com.compress().as_bytes().to_vec()));
        hasher.update(&lch.hash);
        hasher.update(&rch.hash);
        let hash = hasher.finalize().to_vec();

        DapolProofNode::new(com, hash)
    }
}

impl<D: Digest> Serializable for DapolProofNode<D> {
    /// (Com_1 || Hash_1) || ... || (Com_n || Hash_n)
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.get_com().compress().as_bytes()[..]);
        result.extend_from_slice(&self.get_hash());
        result
    }

    fn deserialize_as_a_unit(bytes: &[u8], begin: &mut usize) -> Result<Self, DecodingError> {
        let unit_byte_num = COM_BYTE_NUM + D::output_size();
        if bytes.len() - *begin < unit_byte_num {
            return Err(DecodingError::BytesNotEnough);
        }

        let commitment =
            CompressedRistretto::from_slice(&bytes[*begin..*begin + COM_BYTE_NUM]).decompress();
        *begin += COM_BYTE_NUM;
        if commitment.is_none() {
            return Err(DecodingError::ValueDecodingError {
                msg: "Not the canonical encoding of a point.".to_string(),
            });
        }
        let node = DapolProofNode {
            com: commitment.unwrap(),
            hash: bytes[*begin..*begin + D::output_size()].to_vec(),
            _phantom_hash_function: PhantomData,
        };
        *begin += D::output_size();
        Ok(node)
    }

    /// (Com_1 || Hash_1) || ... || (Com_n || Hash_n)
    fn deserialize(bytes: &[u8]) -> Result<Self, DecodingError> {
        let mut begin = 0;
        Self::deserialize_as_a_unit(bytes, &mut begin)
    }
}

impl<D: TypeName> TypeName for DapolProofNode<D> {
    /// Returns the type name of DAPOL proof nodes with corresponding hash function (for logging purpose).
    fn get_name() -> String {
        format!("DAPOL Proof Node ({})", D::get_name())
    }
}
