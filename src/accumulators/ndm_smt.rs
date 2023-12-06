use std::collections::HashMap;

use primitive_types::H256;
use serde::{Deserialize, Serialize};

use log::error;
use logging_timer::{timer, Level};

use rayon::prelude::*;

use crate::binary_tree::{
    BinaryTree, Coordinate, Height, InputLeafNode, PathSiblings, TreeBuilder,
};
use crate::entity::{Entity, EntityId};
use crate::inclusion_proof::{
    AggregationFactor, InclusionProof, DEFAULT_RANGE_PROOF_UPPER_BOUND_BIT_LENGTH,
};
use crate::kdf::generate_key;
use crate::node_content::FullNodeContent;
use crate::MaxThreadCount;

mod ndm_smt_secrets;
pub use ndm_smt_secrets::NdmSmtSecrets;

mod ndm_smt_secrets_parser;
pub use ndm_smt_secrets_parser::NdmSmtSecretsParser;

mod x_coord_generator;
pub use x_coord_generator::RandomXCoordGenerator;

mod ndm_smt_config;
pub use ndm_smt_config::{NdmSmtConfig, NdmSmtConfigBuilder, NdmSmtConfigParserError};

// -------------------------------------------------------------------------------------------------
// Main struct and implementation.

type Content = FullNodeContent;

/// Non-Deterministic Mapping Sparse Merkle Tree (NDM-SMT) accumulator type.
///
/// This accumulator variant is the simplest. Each entity is randomly mapped to
/// a bottom-layer node in the tree. The algorithm used to determine the mapping
/// uses a variation of Durstenfeld’s shuffle algorithm (see
/// [RandomXCoordGenerator]) and will not produce the same mapping for the same
/// inputs, hence the "non-deterministic" term in the title.
///
/// Construction of this tree can be done via [NdmSmtConfigBuilder].
///
/// The struct contains a tree object, secrets used for construction, and an
/// entity mapping.
///
/// The entity mapping structure is required because each entity is randomly
/// mapped to a leaf node, and this assignment is non-deterministic. The map
/// keeps track of which entity is assigned to which leaf node.

#[derive(Debug, Serialize, Deserialize)]
pub struct NdmSmt {
    secrets: NdmSmtSecrets,
    tree: BinaryTree<Content>,
    entity_mapping: HashMap<EntityId, u64>,
}

impl NdmSmt {
    /// Constructor.
    ///
    /// Each element in `entities` is converted to an
    /// [input leaf node] and randomly assigned a position on the
    /// bottom layer of the tree.
    ///
    /// An [NdmSmtError] is returned if:
    /// 1. There are more entities than the height allows i.e. more entities
    /// than would fit on the bottom layer.
    /// 2. The tree build fails for some reason.
    /// 3. There are duplicate entity IDs.
    ///
    /// The function will panic if there is a problem joining onto a spawned
    /// thread, or if concurrent variables are not able to be locked. It's not
    /// clear how to recover from these scenarios because variables may be in
    /// an unknown state, so rather panic.
    ///
    /// [input leaf node]: crate::binary_tree::InputLeafNode
    pub fn new(
        secrets: NdmSmtSecrets,
        height: Height,
        max_thread_count: MaxThreadCount,
        entities: Vec<Entity>,
    ) -> Result<Self, NdmSmtError> {
        let master_secret_bytes = secrets.master_secret.as_bytes();
        let salt_b_bytes = secrets.salt_b.as_bytes();
        let salt_s_bytes = secrets.salt_s.as_bytes();

        let (leaf_nodes, entity_coord_tuples) = {
            // Map the entities to bottom-layer leaf nodes.

            let tmr = timer!(Level::Debug; "Entity to leaf node conversion");

            let mut x_coord_generator = RandomXCoordGenerator::from(&height);
            let mut x_coords = Vec::<u64>::with_capacity(entities.len());

            for _i in 0..entities.len() {
                x_coords.push(x_coord_generator.new_unique_x_coord()?);
            }

            let entity_coord_tuples = entities
                .into_iter()
                .zip(x_coords.into_iter())
                .collect::<Vec<(Entity, u64)>>();

            let leaf_nodes = entity_coord_tuples
                .par_iter()
                .map(|(entity, x_coord)| {
                    // `w` is the letter used in the DAPOL+ paper.
                    let entity_secret: [u8; 32] =
                        generate_key(None, master_secret_bytes, Some(&x_coord.to_le_bytes()))
                            .into();
                    let blinding_factor = generate_key(Some(salt_b_bytes), &entity_secret, None);
                    let entity_salt = generate_key(Some(salt_s_bytes), &entity_secret, None);

                    InputLeafNode {
                        content: Content::new_leaf(
                            entity.liability,
                            blinding_factor.into(),
                            entity.id.clone(),
                            entity_salt.into(),
                        ),
                        x_coord: *x_coord,
                    }
                })
                .collect::<Vec<InputLeafNode<Content>>>();

            logging_timer::finish!(
                tmr,
                "Leaf nodes have length {} and size {} bytes",
                leaf_nodes.len(),
                std::mem::size_of_val(&*leaf_nodes)
            );

            (leaf_nodes, entity_coord_tuples)
        };

        // Create a map of EntityId -> XCoord, return an error if a duplicate
        // entity ID is found.
        let mut entity_mapping = HashMap::with_capacity(entity_coord_tuples.len());
        for (entity, x_coord) in entity_coord_tuples.into_iter() {
            if entity_mapping.contains_key(&entity.id) {
                return Err(NdmSmtError::DuplicateEntityIds(entity.id));
            }
            entity_mapping.insert(entity.id, x_coord);
        }

        let tree = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes)
            .with_max_thread_count(max_thread_count)
            .build_using_multi_threaded_algorithm(new_padding_node_content_closure(
                *master_secret_bytes,
                *salt_b_bytes,
                *salt_s_bytes,
            ))?;

        Ok(NdmSmt {
            tree,
            secrets,
            entity_mapping,
        })
    }

    /// Generate an inclusion proof for the given `entity_id`.
    ///
    /// The NdmSmt struct defines the content type that is used, and so must
    /// define how to extract the secret value (liability) and blinding
    /// factor for the range proof, which are both required for the range
    /// proof that is done in the [InclusionProof] constructor.
    ///
    /// `aggregation_factor` is used to determine how many of the range proofs
    /// are aggregated. Those that do not form part of the aggregated proof
    /// are just proved individually. The aggregation is a feature of the
    /// Bulletproofs protocol that improves efficiency.
    ///
    /// `upper_bound_bit_length` is used to determine the upper bound for the
    /// range proof, which is set to `2^upper_bound_bit_length` i.e. the
    /// range proof shows `0 <= liability <= 2^upper_bound_bit_length` for
    /// some liability. The type is set to `u8` because we are not expected
    /// to require bounds higher than $2^256$. Note that if the value is set
    /// to anything other than 8, 16, 32 or 64 the Bulletproofs code will return
    /// an Err.
    pub fn generate_inclusion_proof_with(
        &self,
        entity_id: &EntityId,
        aggregation_factor: AggregationFactor,
        upper_bound_bit_length: u8,
    ) -> Result<InclusionProof, NdmSmtError> {
        let master_secret_bytes = self.secrets.master_secret.as_bytes();
        let salt_b_bytes = self.secrets.salt_b.as_bytes();
        let salt_s_bytes = self.secrets.salt_s.as_bytes();
        let new_padding_node_content =
            new_padding_node_content_closure(*master_secret_bytes, *salt_b_bytes, *salt_s_bytes);

        let leaf_node = self
            .entity_mapping
            .get(entity_id)
            .and_then(|leaf_x_coord| self.tree.get_leaf_node(*leaf_x_coord))
            .ok_or(NdmSmtError::EntityIdNotFound)?;

        let path_siblings = PathSiblings::build_using_multi_threaded_algorithm(
            &self.tree,
            &leaf_node,
            new_padding_node_content,
        )?;

        Ok(InclusionProof::generate(
            leaf_node,
            path_siblings,
            aggregation_factor,
            upper_bound_bit_length,
        )?)
    }

    /// Generate an inclusion proof for the given entity_id.
    ///
    /// Use the default values for Bulletproof parameters:
    /// - `aggregation_factor`: half of all the range proofs are aggregated
    /// - `upper_bound_bit_length`: 64 (which should be plenty enough for most
    ///   real-world cases)
    pub fn generate_inclusion_proof(
        &self,
        entity_id: &EntityId,
    ) -> Result<InclusionProof, NdmSmtError> {
        self.generate_inclusion_proof_with(
            entity_id,
            AggregationFactor::default(),
            DEFAULT_RANGE_PROOF_UPPER_BOUND_BIT_LENGTH,
        )
    }

    /// Return the hash digest/bytes of the root node for the binary tree.
    pub fn root_hash(&self) -> H256 {
        self.tree.root().content.hash
    }

    /// Return the entity mapping, the x-coord that each entity is mapped to.
    pub fn entity_mapping(&self) -> &HashMap<EntityId, u64> {
        &self.entity_mapping
    }

    /// Return the height of the binary tree.
    pub fn height(&self) -> &Height {
        self.tree.height()
    }
}

// -------------------------------------------------------------------------------------------------
// Helper functions.

/// Create a new closure that generates padding node content using the secret
/// values.
fn new_padding_node_content_closure(
    master_secret_bytes: [u8; 32],
    salt_b_bytes: [u8; 32],
    salt_s_bytes: [u8; 32],
) -> impl Fn(&Coordinate) -> Content {
    // closure that is used to create new padding nodes
    move |coord: &Coordinate| {
        // TODO unfortunately we copy data here, maybe there is a way to do without
        // copying
        let coord_bytes = coord.as_bytes();
        // pad_secret is given as 'w' in the DAPOL+ paper
        let pad_secret = generate_key(None, &master_secret_bytes, Some(&coord_bytes));
        let pad_secret_bytes: [u8; 32] = pad_secret.into();
        let blinding_factor = generate_key(Some(&salt_b_bytes), &pad_secret_bytes, None);
        let salt = generate_key(Some(&salt_s_bytes), &pad_secret_bytes, None);
        Content::new_pad(blinding_factor.into(), coord, salt.into())
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

/// Errors encountered when handling [NdmSmt].
#[derive(thiserror::Error, Debug)]
pub enum NdmSmtError {
    #[error("Problem constructing the tree")]
    TreeError(#[from] crate::binary_tree::TreeBuildError),
    #[error("Number of entities cannot be bigger than 2^(height-1)")]
    HeightTooSmall(#[from] x_coord_generator::OutOfBoundsError),
    #[error("Inclusion proof generation failed when trying to build the path in the tree")]
    InclusionProofPathSiblingsGenerationError(#[from] crate::binary_tree::PathSiblingsBuildError),
    #[error("Inclusion proof generation failed")]
    InclusionProofGenerationError(#[from] crate::inclusion_proof::InclusionProofError),
    #[error("Entity ID not found in the entity mapping")]
    EntityIdNotFound,
    #[error("Entity ID {0:?} was duplicated")]
    DuplicateEntityIds(EntityId),
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

// TODO test that the tree error propagates correctly (how do we mock in rust?)
// TODO we should fuzz on these tests because the code utilizes a random number
// generator
// TODO test that duplicate entity IDs gives an error on NdmSmt::new
// TODO test serialization & deserialization
#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret::Secret;
    use std::str::FromStr;

    #[test]
    fn constructor_works() {
        let master_secret: Secret = 1u64.into();
        let salt_b: Secret = 2u64.into();
        let salt_s: Secret = 3u64.into();
        let secrets = NdmSmtSecrets {
            master_secret,
            salt_b,
            salt_s,
        };

        let height = Height::try_from(4u8).unwrap();
        let max_thread_count = MaxThreadCount::default();
        let entities = vec![Entity {
            liability: 5u64,
            id: EntityId::from_str("some entity").unwrap(),
        }];

        NdmSmt::new(secrets, height, max_thread_count, entities).unwrap();
    }
}
