//! Non-deterministic mapping sparse Merkle tree (NDM_SMT).
//!
//! The accumulator variant is the simplest. Each entity is randomly mapped to
//! a bottom-layer node in the tree. The algorithm used to determine the mapping
//! uses a variation of Durstenfeld’s shuffle algorithm (see
//! [RandomXCoordGenerator]) and will not produce the same mapping for the same
//! inputs, hence the "non-deterministic" term in the title.
//!
//! The hash function chosen for the Merkle Sum Tree is blake3.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use log::error;
use logging_timer::{timer, Level};

use rayon::prelude::*;

use crate::binary_tree::{BinaryTree, Coordinate, Height, InputLeafNode, TreeBuilder};
use crate::entity::{Entity, EntityId};
use crate::inclusion_proof::{AggregationFactor, InclusionProof};
use crate::kdf::generate_key;
use crate::node_content::FullNodeContent;
use crate::utils;

mod secrets;
use secrets::Secrets;
mod secrets_parser;
pub use secrets_parser::SecretsParser;
mod x_coord_generator;
use x_coord_generator::RandomXCoordGenerator;

// -------------------------------------------------------------------------------------------------
// Main struct and implementation.

type Hash = blake3::Hasher;
type Content = FullNodeContent<Hash>;

/// Main struct containing tree object, master secret and the salts.
///
/// The entity mapping structure is required because each entity is randomly
/// mapped to a leaf node, and this assignment is non-deterministic. The map
/// keeps track of which entity is assigned to which leaf node.
#[derive(Serialize, Deserialize)]
pub struct NdmSmt {
    secrets: Secrets,
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
        secrets: Secrets,
        height: Height,
        entities: Vec<Entity>,
    ) -> Result<Self, NdmSmtError> {
        // This is used to determine the number of threads to spawn in the
        // multi-threaded builder.
        crate::utils::DEFAULT_PARALLELISM_APPROX.with(|opt| {
            *opt.borrow_mut() = std::thread::available_parallelism()
                .map_err(|err| {
                    error!("Problem accessing machine parallelism: {}", err);
                    err
                })
                .map_or(None, |par| Some(par.get() as u8))
        });

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
                    let w: [u8; 32] =
                        generate_key(master_secret_bytes, &x_coord.to_le_bytes()).into();
                    let blinding_factor = generate_key(&w, salt_b_bytes);
                    let entity_salt = generate_key(&w, salt_s_bytes);

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

    /// Generate an inclusion proof for the given entity_id.
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
    //j
    /// `upper_bound_bit_length` is used to determine the upper bound for the
    /// range proof, which is set to `2^upper_bound_bit_length` i.e. the
    /// range proof shows `0 <= liability <= 2^upper_bound_bit_length` for
    /// some liability. The type is set to `u8` because we are not expected
    /// to require bounds higher than $2^256$. Note that if the value is set
    /// to anything other than 8, 16, 32 or 64 the Bulletproofs code will return
    /// an Err.
    pub fn generate_inclusion_proof_with_custom_range_proof_params(
        &self,
        entity_id: &EntityId,
        aggregation_factor: AggregationFactor,
        upper_bound_bit_length: u8,
    ) -> Result<InclusionProof<Hash>, NdmSmtError> {
        let leaf_x_coord = self
            .entity_mapping
            .get(entity_id)
            .ok_or(NdmSmtError::EntityIdNotFound)?;

        let master_secret_bytes = self.secrets.master_secret.as_bytes();
        let salt_b_bytes = self.secrets.salt_b.as_bytes();
        let salt_s_bytes = self.secrets.salt_s.as_bytes();
        let new_padding_node_content =
            new_padding_node_content_closure(*master_secret_bytes, *salt_b_bytes, *salt_s_bytes);

        let path = self
            .tree
            .path_builder()
            .with_leaf_x_coord(*leaf_x_coord)
            .build_using_multi_threaded_algorithm(new_padding_node_content)?;

        Ok(InclusionProof::generate(
            path,
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
    fn generate_inclusion_proof(
        &self,
        entity_id: &EntityId,
    ) -> Result<InclusionProof<Hash>, NdmSmtError> {
        let aggregation_factor = AggregationFactor::Divisor(2u8);
        let upper_bound_bit_length = 64u8;
        self.generate_inclusion_proof_with_custom_range_proof_params(
            entity_id,
            aggregation_factor,
            upper_bound_bit_length,
        )
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
        let pad_secret = generate_key(&master_secret_bytes, &coord_bytes);
        let pad_secret_bytes: [u8; 32] = pad_secret.into();
        let blinding_factor = generate_key(&pad_secret_bytes, &salt_b_bytes);
        let salt = generate_key(&pad_secret_bytes, &salt_s_bytes);
        Content::new_pad(blinding_factor.into(), coord, salt.into())
    }
}

/// Try deserialize an NDM-SMT from the given file path.
///
/// The file is assumed to be in [bincode] format.
///
/// An error is logged and returned if
/// 1. The file cannot be opened.
/// 2. The [bincode] deserializer fails.
pub fn deserialize(path: PathBuf) -> Result<NdmSmt, NdmSmtError> {
    use crate::read_write_utils::deserialize_from_bin_file;
    use crate::utils::LogOnErr;

    let deserialized = deserialize_from_bin_file::<NdmSmt>(path).log_on_err()?;
    Ok(deserialized)
}

// -------------------------------------------------------------------------------------------------
// Errors.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NdmSmtError {
    #[error("Problem constructing the tree")]
    TreeError(#[from] crate::binary_tree::TreeBuildError),
    #[error("Number of entities cannot be bigger than 2^(height-1)")]
    HeightTooSmall(#[from] x_coord_generator::OutOfBoundsError),
    #[error("Inclusion proof generation failed when trying to build the path in the tree")]
    InclusionProofPathGenerationError(#[from] crate::binary_tree::PathBuildError),
    #[error("Inclusion proof generation failed")]
    InclusionProofGenerationError(#[from] crate::inclusion_proof::InclusionProofError),
    #[error("Entity ID not found in the entity mapping")]
    EntityIdNotFound,
    #[error("Entity ID {0:?} was duplicated")]
    DuplicateEntityIds(EntityId),
    #[error("Error deserializing file")]
    DeserializationError(#[from] crate::read_write_utils::ReadWriteError),
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
    mod ndm_smt {
        use super::super::*;
        use crate::binary_tree::Height;
        use std::str::FromStr;

        #[test]
        fn constructor_works() {
            let master_secret: Secret = 1u64.into();
            let salt_b: Secret = 2u64.into();
            let salt_s: Secret = 3u64.into();
            let secrets = Secrets {
                master_secret,
                salt_b,
                salt_s,
            };

            let height = Height::from(4u8);
            let entities = vec![Entity {
                liability: 5u64,
                id: EntityId::from_str("some entity").unwrap(),
            }];

            NdmSmt::new(secrets, height, entities).unwrap();
        }
    }

    mod random_x_coord_generator {
        use std::collections::HashSet;

        use super::super::{OutOfBoundsError, RandomXCoordGenerator};
        use crate::binary_tree::{max_bottom_layer_nodes, Height};

        #[test]
        fn constructor_works() {
            let height = Height::from(4u8);
            RandomXCoordGenerator::from(&height);
        }

        #[test]
        fn new_unique_value_works() {
            let height = Height::from(4u8);
            let mut rxcg = RandomXCoordGenerator::from(&height);
            for i in 0..max_bottom_layer_nodes(&height) {
                rxcg.new_unique_x_coord().unwrap();
            }
        }

        #[test]
        fn generated_values_all_unique() {
            let height = Height::from(4u8);
            let mut rxcg = RandomXCoordGenerator::from(&height);
            let mut set = HashSet::<u64>::new();
            for i in 0..max_bottom_layer_nodes(&height) {
                let x = rxcg.new_unique_x_coord().unwrap();
                if set.contains(&x) {
                    panic!("{:?} was generated twice!", x);
                }
                set.insert(x);
            }
        }

        #[test]
        fn new_unique_value_fails_for_large_i() {
            use crate::test_utils::assert_err;

            let height = Height::from(4u8);
            let mut rxcg = RandomXCoordGenerator::from(&height);
            let max = max_bottom_layer_nodes(&height);
            let mut res = rxcg.new_unique_x_coord();

            for i in 0..max {
                res = rxcg.new_unique_x_coord();
            }

            assert_err!(res, Err(OutOfBoundsError { max_value: max }));
        }
    }
}
