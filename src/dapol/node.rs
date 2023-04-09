use crate::DapolProofNode;
use bulletproofs::PedersenGens;
use curve25519_dalek_ng::{ristretto::RistrettoPoint, scalar::Scalar};
use digest::Digest;
use rand::{thread_rng, Rng};
use smtree::{
    index::TreeIndex,
    pad_secret::Secret,
    traits::{Mergeable, Paddable, ProofExtractable, Rand, TypeName},
};
use std::marker::PhantomData;

// DAPOL NODE
// ================================================================================================

/// A node of the DAPOL tree, consisting of the value, the blinding factor,
/// the Pedersen commitment and the hash.
#[derive(Default, Clone, Debug)]
pub struct DapolNode<D> {
    v: u64,                                 // The value.
    v_blinding: Scalar,                     // The blinding factor.
    com: RistrettoPoint,                    // The Pedersen commitment.
    hash: Vec<u8>,                          // The hash.
    _phantom_hash_function: PhantomData<D>, // The hash function.
}

impl<D: Digest> DapolNode<D> {
    /// The constructor.
    pub fn new(value: u64, v_blinding: Scalar) -> DapolNode<D> {
        // compute the Pedersen commitment to the value
        let com = PedersenGens::default().commit(Scalar::from(value), v_blinding);

        // compute the hash as the hashing of the commitment
        let mut hasher = D::new();
        hasher.update(&(com.compress().as_bytes().to_vec()));
        let hash = hasher.finalize().to_vec();

        DapolNode {
            v: value,
            v_blinding,
            com,
            hash,
            _phantom_hash_function: PhantomData,
        }
    }

    /// Returns the value of the DAPOL node.
    pub fn get_value(&self) -> u64 {
        self.v
    }

    /// Returns the blinding factor of the DAPOL node.
    pub fn get_blinding(&self) -> Scalar {
        self.v_blinding
    }
}

impl<D: Digest> Mergeable for DapolNode<D> {
    /// Returns the parent node by merging two child nodes.
    ///
    /// The value and blinding factor of the parent are the sums of the two children respectively.
    /// The commitment of the parent is the homomorphic sum of the two children.
    /// The hash of the parent is computed by hashing the concatenated commitments and hashes of two children.
    fn merge(lch: &DapolNode<D>, rch: &DapolNode<D>) -> DapolNode<D> {
        // H(parent) = Hash(C(L) || C(R) || H(L) || H(R))
        let mut hasher = D::new();
        hasher.update(lch.com.compress().as_bytes());
        hasher.update(rch.com.compress().as_bytes());
        hasher.update(&lch.hash);
        hasher.update(&rch.hash);

        // V/B/C(parent) = V/B/C(L) + V/B/C(R)
        DapolNode {
            v: lch.v + rch.v,
            v_blinding: lch.v_blinding + rch.v_blinding,
            com: lch.com + rch.com,
            hash: hasher.finalize().to_vec(),
            _phantom_hash_function: PhantomData,
        }
    }
}

impl<D: Digest> Paddable for DapolNode<D> {
    /// Returns a padding node with value 0 and a random blinding factor.
    /// TODO: check with Kostas if this padding is ok.
    fn padding(_idx: &TreeIndex, _secret: &Secret) -> DapolNode<D> {
        DapolNode::<D>::new(0, Scalar::random(&mut thread_rng()))
    }
}

impl<D> ProofExtractable for DapolNode<D> {
    type ProofNode = DapolProofNode<D>;
    fn get_proof_node(&self) -> Self::ProofNode {
        DapolProofNode::new(self.com, self.hash.clone())
    }
}

// TODO: this seems to be used for testing purposes only
impl<D: Digest> Rand for DapolNode<D> {
    /// Randomly generates a DAPOL node with random value and random blinding factor.
    fn randomize(&mut self) {
        // The value shouldn't be generated as u64 to prevent overflow of sums.
        let tmp: u32 = thread_rng().gen();
        *self = DapolNode::<D>::new(tmp as u64, Scalar::random(&mut thread_rng()));
    }
}

impl<D: TypeName> TypeName for DapolNode<D> {
    /// Returns the type name of DAPOL nodes with corresponding hash function (for logging purpose).
    fn get_name() -> String {
        format!("DAPOL Node ({})", D::get_name())
    }
}

impl<D> PartialEq for DapolNode<D> {
    /// Two DAPOL nodes are considered equal iff the values are equal.
    fn eq(&self, other: &Self) -> bool {
        self.v == other.v
    }
}

impl<D> Eq for DapolNode<D> {}
