//! Legacy

use crate::{errors::DapolError, DapolProof, DapolProofNode, RangeProvable, RangeVerifiable};
use curve25519_dalek_ng::scalar::Scalar;
use digest::Digest;
use smtree::{
    index::TreeIndex,
    pad_secret::Secret,
    proof::MerkleProof,
    traits::{ProofExtractable, Serializable},
    tree::SparseMerkleTree,
};
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    marker::PhantomData,
};

mod node;
pub use node::DapolNode;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

const MAX_TREE_HEIGHT: usize = 64;
const MIN_SPARSITY: usize = 2;
const DIGEST_SIZE: usize = 32;
const MAX_INDEX_RETRIES: usize = 128;

// SUPPORTING TYPES
// ================================================================================================

type TreeInputs<D> = Vec<(TreeIndex, DapolNode<D>)>;
type IdToIndexMap = HashMap<LiabilityId, TreeIndex>;

/// Represents either an internal or external liability ID, which is just a vector of bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LiabilityId(Vec<u8>);

/// Defines options for a specific instance of Dapol tree. `audit_id` could be a concatenation of
/// a randomly derived seed and a date of th audit. `tree_height` cannot exceed 64.
pub struct DapolOptions {
    audit_seed: Vec<u8>,
    tree_height: usize,
    aggregation_factor: usize,
    secret: Secret, // TODO: should we derive this from `audit_seed`?
}

/// Represents a single liability. `internal_id` is the unique identifier of an account internal
/// to the audited system, while `external_id` is a unique identifier known to the user (e.g. an
/// email address). External and internal IDs can be the same.
pub struct Liability {
    pub internal_id: LiabilityId,
    pub external_id: LiabilityId,
    pub value: u64,
}

// LIABILITY ID
// ================================================================================================

impl LiabilityId {
    /// Returns an new liability ID constructed from the specified string.
    pub fn from_str(id: &str) -> Self {
        LiabilityId(id.as_bytes().to_vec())
    }

    /// Returns byte representation of this ID.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

// DAPOL
// ================================================================================================

#[derive(Default, Debug)]
pub struct Dapol<D, R> {
    smt: SparseMerkleTree<DapolNode<D>>,
    id_to_idx_map: IdToIndexMap,
    aggregation_factor: usize,
    _phantom_r: PhantomData<R>,
}

impl<D, R> Dapol<D, R>
where
    D: Digest + Default + Clone + std::fmt::Debug,
    R: Clone + RangeProvable + RangeVerifiable + Serializable,
{
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Builds a new instance of DAPOL tree from the provided set of liabilities;
    ///
    /// Returns an error if:
    /// * The specified digest does not output 32-byte hashes.
    /// * Requested tree height exceeds 64.
    /// * The number of liabilities is not at least 4 times smaller than the total number of
    ///   of leaves in the tree.
    /// * List of liabilities contains duplicated internal IDs.
    pub fn new(liabilities: Vec<Liability>, options: DapolOptions) -> Result<Self, DapolError> {
        if D::output_size() != DIGEST_SIZE {
            return Err(DapolError::InvalidDigestSize(DIGEST_SIZE, D::output_size()));
        }
        if options.tree_height > MAX_TREE_HEIGHT {
            return Err(DapolError::TreeHeightTooBig(
                MAX_TREE_HEIGHT,
                options.tree_height,
            ));
        }
        if 2usize.pow(options.tree_height as u32) < liabilities.len() * MIN_SPARSITY {
            return Err(DapolError::SparsityTooSmall(
                liabilities.len(),
                liabilities.len() * MIN_SPARSITY,
                options.tree_height,
            ));
        }

        let mut smt = SparseMerkleTree::<DapolNode<D>>::new(options.tree_height);
        let (tree_inputs, id_to_idx_map) =
            build_leaf_nodes(liabilities, &options.audit_seed, options.tree_height)?;
        smt.build(&tree_inputs, &options.secret);
        Ok(Dapol {
            smt,
            id_to_idx_map,
            aggregation_factor: options.aggregation_factor,
            _phantom_r: PhantomData,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the root of the DAPOL tree.
    pub fn root_raw(&self) -> &DapolNode<D> {
        self.smt.get_root_raw()
    }

    /// Returns the root of the DAPOL tree as a proof node.
    pub fn root(&self) -> DapolProofNode<D> {
        self.smt.get_root()
    }

    // PUBLIC METHODS
    // --------------------------------------------------------------------------------------------

    /// Returns a proof for a single liability identified by the specified `internal_id`. If a
    /// liability for the specified ID does not exist in the tree, None is returned.
    pub fn generate_proof_for_id(&self, internal_id: &LiabilityId) -> Option<DapolProof<D, R>> {
        let idx = self.id_to_idx_map.get(internal_id)?;
        self.generate_proof(idx)
    }

    /// Returns a batch proof for a list of liabilities identified by the specified `internal_ids`.
    /// If a liability for any of the specified IDs does not exist in the tree, None is returned.
    pub fn generate_proof_batch_for_ids(
        &self,
        internal_ids: &[LiabilityId],
    ) -> Option<DapolProof<D, R>> {
        let mut indexes = Vec::with_capacity(internal_ids.len());
        for id in internal_ids.iter() {
            indexes.push(*self.id_to_idx_map.get(id)?);
        }
        self.generate_proof_batch(indexes.as_slice())
    }

    /// Returns a proof for a single liability located at the specified leaf.
    pub fn generate_proof(&self, leaf_idx: &TreeIndex) -> Option<DapolProof<D, R>> {
        self.generate_proof_batch(&[*leaf_idx])
    }

    /// Returns a bach proof for a list of liabilities located at the specified leaves.
    pub fn generate_proof_batch(&self, leaf_indexes: &[TreeIndex]) -> Option<DapolProof<D, R>> {
        let refs = self.smt.get_merkle_path_ref_batch(leaf_indexes)?;

        let mut merkle_proof: MerkleProof<DapolNode<D>> = MerkleProof::new_batch(leaf_indexes);

        let mut values: Vec<u64> = Vec::new();
        let mut blindings: Vec<Scalar> = Vec::new();
        for item in refs.iter().skip(leaf_indexes.len()) {
            let node = self.smt.get_node_by_ref(*item).get_value();
            merkle_proof.add_sibling(node.get_proof_node());
            values.push(node.get_value());
            blindings.push(node.get_blinding());
        }

        Some(DapolProof::new(
            merkle_proof,
            R::generate_proof(&values, &blindings, self.aggregation_factor),
        ))
    }

    // TEST METHODS
    // --------------------------------------------------------------------------------------------
    // TODO: methods below are used for testing only and should ideally be moved to a test module

    pub fn new_blank(height: usize, aggregation_factor: usize) -> Dapol<D, R> {
        let smt = SparseMerkleTree::<DapolNode<D>>::new(height);
        Dapol {
            smt,
            id_to_idx_map: HashMap::new(),
            aggregation_factor,
            _phantom_r: PhantomData,
        }
    }

    pub fn build(&mut self, input: &[(TreeIndex, DapolNode<D>)], secret: &Secret) {
        self.smt.build(input, secret);
    }

    #[cfg(test)]
    pub fn update(&mut self, idx: &TreeIndex, value: DapolNode<D>, secret: &Secret) {
        self.smt.update(idx, value, secret);
    }

    #[cfg(test)]
    pub fn generate_all_proofs(&self) {
        let mut secrets: Vec<u64> = Vec::new();
        let mut blindings: Vec<Scalar> = Vec::new();
        let mut merkle_siblings: Vec<DapolProofNode<D>> = Vec::new();
        let mut range_proof: R = R::new(&[], &[]);
        self.dfs(
            TreeIndex::zero(0),
            self.smt.get_root_ref(),
            &mut secrets,
            &mut blindings,
            &mut merkle_siblings,
            &mut range_proof,
        );
    }

    #[cfg(test)]
    fn dfs(
        &self,
        idx: TreeIndex,
        smt_ref: usize,
        secrets: &mut Vec<u64>,
        blindings: &mut Vec<Scalar>,
        merkle_siblings: &mut Vec<DapolProofNode<D>>,
        range_proof: &mut R,
    ) {
        if idx.get_height() > self.smt.get_height() {
            let leaf = self
                .smt
                .get_node_by_ref(smt_ref)
                .get_value()
                .get_proof_node();
            let mut merkle = MerkleProof::new(idx);
            merkle.set_siblings(merkle_siblings.to_vec());
            let proof = DapolProof::new(merkle, range_proof.clone());
            // TODO: write the proofs to a local file or send them out.
            if cfg!(test) {
                let serialized_proof = proof.serialize();
                let deserialized_proof =
                    DapolProof::<D, R>::deserialize(&serialized_proof).unwrap();
                assert!(deserialized_proof.verify(&self.root(), &leaf));
            }
            return;
        }

        let lch = self.smt.get_node_by_ref(smt_ref).get_lch();
        if let Some(x) = lch {
            self.dfs_handle_child(
                idx.get_lch_index(),
                x,
                secrets,
                blindings,
                merkle_siblings,
                range_proof,
            );
        }

        let rch = self.smt.get_node_by_ref(smt_ref).get_rch();
        if let Some(x) = rch {
            self.dfs_handle_child(
                idx.get_rch_index(),
                x,
                secrets,
                blindings,
                merkle_siblings,
                range_proof,
            );
        }
    }

    #[cfg(test)]
    fn dfs_handle_child(
        &self,
        idx: TreeIndex,
        smt_ref: usize,
        secrets: &mut Vec<u64>,
        blindings: &mut Vec<Scalar>,
        merkle_siblings: &mut Vec<DapolProofNode<D>>,
        range_proof: &mut R,
    ) {
        let node = self.smt.get_node_by_ref(smt_ref).get_value();
        secrets.push(node.get_value());
        blindings.push(node.get_blinding());
        merkle_siblings.push(node.get_proof_node());
        range_proof.generate_proof_by_new_com(secrets, blindings, self.aggregation_factor);

        self.dfs(
            idx,
            smt_ref,
            secrets,
            blindings,
            merkle_siblings,
            range_proof,
        );

        range_proof.remove_proof_by_last_com(secrets.len(), self.aggregation_factor);
        secrets.pop();
        blindings.pop();
        merkle_siblings.pop();
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Converts a list of liabilities into a list of (TreeIndex, DapolNode) tuples. Tree index is
/// derived from audit_id = hash(audit_seed || internal_id). The blinding factor is derived from
/// user_audit_id = hash(audit_id || external_id).
fn build_leaf_nodes<D: Digest>(
    liabilities: Vec<Liability>,
    audit_seed: &[u8],
    tree_height: usize,
) -> Result<(TreeInputs<D>, IdToIndexMap), DapolError> {
    let mut hasher = D::new();

    let mut internal_id_map = HashMap::with_capacity(liabilities.len());
    let mut tree_index_set = HashSet::with_capacity(liabilities.len());

    let mut result = Vec::with_capacity(liabilities.len());
    for Liability {
        internal_id,
        external_id,
        value,
    } in liabilities.into_iter()
    {
        // make sure all internal IDs are unique
        if internal_id_map.contains_key(&internal_id) {
            return Err(DapolError::DuplicatedInternalId(internal_id.0));
        }

        // compute audit ID as hash(audit_seed || internal_id); expect() should never be
        // triggered because we make sure that hash digest is 32 bytes in the constructor
        hasher.update(audit_seed);
        hasher.update(&internal_id.0);
        let audit_id: [u8; 32] = hasher
            .finalize_reset()
            .as_slice()
            .try_into()
            .expect("could not convert hash into a 32-byte value");

        // derive tree index from index_seed = hash(audit_id || "index_seed" || external_id). This
        // arrangement gives us the following properties:
        // - the user can compute index_seed directly from audit_id without any additional info;
        // - an auditor can be given hash(audit_di || "index_seed") and external_id and they will,
        //   be able to compute index_seed, but this info will not allow them to compute the
        //   blinding factor (computed below).
        hasher.update(audit_id);
        hasher.update("index_seed");
        hasher.update(&external_id.0);
        let index_seed = hasher
            .finalize_reset()
            .as_slice()
            .try_into()
            .expect("could not convert hash into a 32-byte value");
        let index = shuffle_index::<D>(index_seed, tree_height, &mut tree_index_set)
            .ok_or_else(|| DapolError::FailedToMapIndex(audit_id.to_vec(), MAX_INDEX_RETRIES))?;

        // derive blinding factor from blind_seed = hash(audit_id || "blind_seed" || external_id).
        // This arrangement gives us the following properties:
        // - the user can compute blinding factor directly from audit_id without any additional info;
        // - an auditor can be given the blind_seed, but this info won't be sufficient to
        //   learn user's identity (i.e. external_id).
        hasher.update(audit_id);
        hasher.update("blind_seed");
        hasher.update(&external_id.0);
        let blind_seed: [u8; 32] = hasher
            .finalize_reset()
            .as_slice()
            .try_into()
            .expect("could not convert hash into a 32-byte value");
        let blinding_factor = Scalar::from_bits(blind_seed);

        // create a mapping between internal_id and tree index
        internal_id_map.insert(internal_id, index);

        // create the node and add it to the results
        let node = DapolNode::<D>::new(value, blinding_factor);
        result.push((index, node));
    }

    // sort by index as smtree requires inputs to be sorted
    result.sort_by_key(|(index, _)| *index);

    Ok((result, internal_id_map))
}

/// Tries find an index for a node based on the provided index_seed. The algorithm works as
/// follows:
/// - take the first n bits of index_seed, where n is equal to the tree height;
/// - if the position implied by these bits is available, return; otherwise, hash index_seed and
///   and check again.
/// - Repeat the previous steps until we find an empty slot, or exhaust the number of allowed
///   tries.
fn shuffle_index<D: Digest>(
    mut index_seed: [u8; 32],
    tree_height: usize,
    tree_index_set: &mut HashSet<u64>,
) -> Option<TreeIndex> {
    let mut hasher = D::new();
    let mut tree_index: Option<TreeIndex> = None;

    for _ in 0..MAX_INDEX_RETRIES {
        // compute hash(index_seed || external_id); on the first iteration, it is the same
        // as hash(audit_id || external_id)
        hasher.update(&index_seed);
        index_seed = hasher
            .finalize_reset()
            .as_slice()
            .try_into()
            .expect("could not convert hash into a 32-byte value");

        // convert the first 8 bytes of the seed into a potential tree index;
        // unwrap is OK here because we know for sure that seed is at least 8 bytes.
        let index = u64::from_be_bytes(index_seed[..8].try_into().unwrap());

        // get rid of extra bits to make sure our index is within the bounds of the tree
        let index = index >> (64 - tree_height);

        // if the index is not in the set, build a tree index from it and break the loop
        if tree_index_set.insert(index) {
            tree_index = Some(TreeIndex::from_u64(tree_height, index));
            break;
        }
    }

    tree_index
}
