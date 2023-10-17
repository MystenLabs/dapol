//! Non-deterministic mapping sparse Merkle tree (NDM_SMT).
//!
//! The accumulator variant is the simplest. Each entity is randomly mapped to
//! a bottom-layer node in the tree. The algorithm used to determine the mapping
//! uses a variation of Durstenfeld’s shuffle algorithm (see
//! [RandomXCoordGenerator]) and will not produce the same mapping for the same
//! inputs, hence the "non-deterministic" term in the title.
//!
//! The hash function chosen for the Merkle Sum Tree is blake3.

use rand::{
    distributions::{Alphanumeric, DistString, Uniform},
    rngs::ThreadRng,
    thread_rng, Rng,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use logging_timer::{finish, time, timer, Level};

use rayon::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use crate::entity::{Entity, EntityId};
use crate::inclusion_proof::{AggregationFactor, InclusionProof, InclusionProofError};
use crate::kdf::generate_key;
use crate::node_content::FullNodeContent;
use crate::secret::Secret;
use crate::{
    binary_tree::{
        BinaryTree, Coordinate, Height, InputLeafNode, PathBuildError, TreeBuildError, TreeBuilder,
    },
    secret,
};

// -------------------------------------------------------------------------------------------------
// Main struct and implementation.

type Hash = blake3::Hasher;
type Content = FullNodeContent<Hash>;

/// Main struct containing tree object, master secret and the salts.
/// The entity mapping structure is required because it is non-deterministic.
pub struct NdmSmt {
    secrets: Secrets,
    tree: BinaryTree<Content>,
    entity_mapping: HashMap<EntityId, u64>,
}

impl NdmSmt {
    /// Constructor.
    ///
    /// Each element in `entities` is converted to a
    /// [binary_tree][InputLeafNode] and randomly assigned a position on the
    /// bottom layer of the tree.
    ///
    /// An [NdmSmtError] is returned if
    ///
    /// The function will panic if there is a problem joining onto a spawned
    /// thread, or if concurrent variables are not able to be locked. It's not
    /// clear how to recover from these scenarios because variables may be in
    /// an unknown state, so rather panic.
    pub fn new(
        secrets: Secrets,
        height: Height,
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
                    let w = generate_key(master_secret_bytes, &x_coord.to_le_bytes());
                    let w_bytes: [u8; 32] = w.into();
                    let blinding_factor = generate_key(&w_bytes, salt_b_bytes);
                    let entity_salt = generate_key(&w_bytes, salt_s_bytes);

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

            // https://stackoverflow.com/questions/62613488/how-do-i-get-the-runtime-memory-size-of-an-object
            use std::mem::size_of_val;
            finish!(
                tmr,
                "Leaf nodes have length {} and size {}",
                leaf_nodes.len(),
                size_of_val(&*leaf_nodes)
            );

            (leaf_nodes, entity_coord_tuples)
        };

        // Spawn a new thread to convert the tuples object into a hashmap, while
        // the main thread goes ahead with the tree build.

        let entity_mapping = Arc::new(Mutex::new(HashMap::new()));
        let entity_mapping_ref = Arc::clone(&entity_mapping);

        let handle = thread::spawn(move || {
            let mut my_entity_mapping = entity_mapping_ref
                .lock()
                .expect("Cannot acquire lock on the entity map");
            entity_coord_tuples
                .into_iter()
                .for_each(|(entity, x_coord)| {
                    my_entity_mapping.insert(entity.id, x_coord);
                });
        });

        let tree_single_threaded = TreeBuilder::new()
            .with_height(height.clone())
            .with_leaf_nodes(leaf_nodes.clone())
            .build_using_single_threaded_algorithm(new_padding_node_content_closure(
                master_secret_bytes.clone(),
                salt_b_bytes.clone(),
                salt_s_bytes.clone(),
            ))?;

        let tree_multi_threaded = TreeBuilder::new()
            .with_height(height)
            .with_leaf_nodes(leaf_nodes)
            .build_using_multi_threaded_algorithm(new_padding_node_content_closure(
                master_secret_bytes.clone(),
                salt_b_bytes.clone(),
                salt_s_bytes.clone(),
            ))?;

        // If there are issues wrapping up the concurrency code then it's not
        // clear how to recover because variables may be in an unknown state,
        // so rather panic.
        handle
            .join()
            .expect("Cannot join thread, possibly due to a panic within the thread");
        let lock = Arc::try_unwrap(entity_mapping).expect("Lock still has multiple owners");
        let entity_mapping = lock.into_inner().expect("Mutex cannot be locked");

        assert_eq!(tree_multi_threaded.root(), tree_single_threaded.root());

        Ok(NdmSmt {
            tree: tree_multi_threaded,
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
        let new_padding_node_content = new_padding_node_content_closure(
            master_secret_bytes.clone(),
            salt_b_bytes.clone(),
            salt_s_bytes.clone(),
        );

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
    pub fn generate_inclusion_proof(
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
// Random shuffle algorithm.

/// Used for generating unique x-coordinate values on the bottom layer of the
/// tree.
///
/// A struct is needed as apposed to just a function because the algorithm used
/// to generate new values requires keeping a memory of previously used values
/// so that it can generate new ones that are different from previous ones.
///
/// Fields:
/// - `rng` is a cryptographically secure pseudo-random number generator.
/// - The `used_x_coords` map keeps track of which x-coords have already been
/// generated.
/// - `max_x_coord` is the upper bound on the generated values, 0 being the
/// lower bound.
/// - `i` is used to track the current position of the algorithm.
///
/// Usage:
/// After creating the struct the calling code can repeatedly call
/// `new_unique_x_coord` any number of times in the range `[1, max_x_coord]`.
/// If the function is called more than `max_x_coord` times an error will be
/// returned.
///
/// The random values are generated using Durstenfeld’s shuffle algorithm
/// optimized by HashMap. This algorithm wraps the `rng`, efficiently avoiding
/// collisions. Here is some pseudo code explaining how it works:
///
/// ```bash,ignore
/// if N > max_x_coord throw error
/// for i in range [0, N]:
/// - pick random k in range [i, max_x_coord]
/// - if k in map then set v = map[k]
///   - while map[v] exists: v = map[v]
///   - result = v
/// - else result = k
/// - set map[k] = i
/// ```
///
/// Assuming `rng` is constant time the above algorithm has time complexity
/// `O(N)`. Note that the second loop (the while loop) will only execute a
/// total of `N` times throughout the entire loop cycle of the first loop.
/// This is because the second loop will only execute if a chain in the map
/// exists, and the worst case happens when there is 1 long chain containing
/// all the elements of the map; in this case the second loop will only execute
/// on 1 of the iterations of the first loop.
// TODO DOCS the above explanation is not so good, improve it
struct RandomXCoordGenerator {
    rng: ThreadRng,
    used_x_coords: HashMap<u64, u64>,
    max_x_coord: u64,
    i: u64,
}

impl RandomXCoordGenerator {
    /// Constructor.
    ///
    /// `height` is used to determine `max_x_coords`: `2^(height-1)`. This means
    /// that `max_x_coords` is the total number of available leaf nodes on the
    /// bottom layer of the tree.
    fn from(height: &Height) -> Self {
        use crate::binary_tree::max_bottom_layer_nodes;

        RandomXCoordGenerator {
            used_x_coords: HashMap::<u64, u64>::new(),
            max_x_coord: max_bottom_layer_nodes(height),
            rng: thread_rng(),
            i: 0,
        }
    }

    /// Generate a new unique random x-coord using Durstenfeld’s shuffle
    /// algorithm optimized by HashMap.
    ///
    /// An error is returned if this function is called more than `max_x_coord`
    /// times.
    fn new_unique_x_coord(&mut self) -> Result<u64, OutOfBoundsError> {
        if self.i >= self.max_x_coord {
            return Err(OutOfBoundsError {
                max_value: self.max_x_coord,
            });
        }

        let range = Uniform::from(self.i..self.max_x_coord);
        let random_x = self.rng.sample(range);

        let x = match self.used_x_coords.get(&random_x) {
            Some(mut existing_x) => {
                // follow the full chain of linked numbers until we find the leaf
                while self.used_x_coords.contains_key(existing_x) {
                    existing_x = self.used_x_coords.get(existing_x).unwrap();
                }
                existing_x.clone()
            }
            None => random_x,
        };

        self.used_x_coords.insert(random_x, self.i);
        self.i += 1;
        Ok(x)
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

// -------------------------------------------------------------------------------------------------
// Secrets struct & parser.

use serde::Deserialize;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::str::FromStr;

/// This coding style is a bit ugly but it is the simplest way to get the
/// desired outcome, which is to deserialize string values into a byte array.
/// We can't deserialize automatically to
/// [crate][secret][Secret] without a custom implementation of the
/// [serde][Deserialize] trait. Instead we deserialize to [SecretsInput] and
/// then convert the individual string fields to byte arrays.
#[derive(Deserialize)]
pub struct SecretsInput {
    master_secret: String,
    salt_b: String,
    salt_s: String,
}

/// Values required for tree construction and inclusion proof generation.
pub struct Secrets {
    master_secret: Secret,
    salt_b: Secret,
    salt_s: Secret,
}

/// Supported file types for the parser.
enum FileType {
    Toml,
}

/// Parser for files containing secrets.
///
/// Supported file types: toml
/// Note that the file type is inferred from its path extension.
///
/// TOML format:
/// ```toml,ignore
/// master_secret = "master_secret"
/// salt_b = "salt_b"
/// salt_s = "salt_s"
/// ```
pub struct SecretsParser {
    file_path: PathBuf,
}

static STRING_CONVERSION_ERR_MSG: &str = "A failure should not be possible here because the length of the random string exactly matches the max allowed length";

impl Secrets {
    #[time("debug", "NdmSmt::Secrets::{}")]
    pub fn generate_random() -> Self {
        let mut rng = thread_rng();
        let master_secret_str = Alphanumeric.sample_string(&mut rng, secret::MAX_LENGTH_BYTES);
        let salt_b_str = Alphanumeric.sample_string(&mut rng, secret::MAX_LENGTH_BYTES);
        let salt_s_str = Alphanumeric.sample_string(&mut rng, secret::MAX_LENGTH_BYTES);

        Secrets {
            master_secret: Secret::from_str(&master_secret_str).expect(STRING_CONVERSION_ERR_MSG),
            salt_b: Secret::from_str(&salt_b_str).expect(STRING_CONVERSION_ERR_MSG),
            salt_s: Secret::from_str(&salt_s_str).expect(STRING_CONVERSION_ERR_MSG),
        }
    }
}

impl TryFrom<SecretsInput> for Secrets {
    type Error = SecretsParseError;

    fn try_from(input: SecretsInput) -> Result<Secrets, SecretsParseError> {
        Ok(Secrets {
            master_secret: Secret::from_str(&input.master_secret)?,
            salt_b: Secret::from_str(&input.salt_b)?,
            salt_s: Secret::from_str(&input.salt_s)?,
        })
    }
}

impl SecretsParser {
    /// Constructor.
    pub fn from_path(file_path: PathBuf) -> Self {
        SecretsParser { file_path }
    }

    /// Open and parse the file, returning a [Secrets] struct.
    ///
    /// An error is returned if:
    /// a) the file cannot be opened
    /// b) the file cannot be read
    /// c) the file type is not supported
    /// d) deserialization of any of the records in the file fails
    pub fn parse(self) -> Result<Secrets, SecretsParseError> {
        let ext = self
            .file_path
            .extension()
            .map(|s| s.to_str())
            .flatten()
            .ok_or(SecretsParseError::UnknownFileType)?;

        let secrets = match FileType::from_str(ext)? {
            FileType::Toml => {
                let mut buf = String::new();
                File::open(self.file_path)?.read_to_string(&mut buf)?;
                let secrets: SecretsInput = toml::from_str(&buf).unwrap();
                Secrets::try_from(secrets)?
            }
        };

        Ok(secrets)
    }
}

impl FromStr for FileType {
    type Err = SecretsParseError;

    fn from_str(ext: &str) -> Result<FileType, Self::Err> {
        match ext {
            "toml" => Ok(FileType::Toml),
            _ => Err(SecretsParseError::UnsupportedFileType { ext: ext.into() }),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Errors.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NdmSmtError {
    #[error("Problem constructing the tree")]
    TreeError(#[from] TreeBuildError),
    #[error("Number of entities cannot be bigger than 2^height")]
    HeightTooSmall(#[from] OutOfBoundsError),
    #[error("Inclusion proof generation failed when trying to build the path in the tree")]
    InclusionProofPathGenerationError(#[from] PathBuildError),
    #[error("Inclusion proof generation failed")]
    InclusionProofGenerationError(#[from] InclusionProofError),
    #[error("Entity ID not found in the entity mapping")]
    EntityIdNotFound,
}

#[derive(Error, Debug)]
#[error("Counter i cannot exceed max value {max_value:?}")]
pub struct OutOfBoundsError {
    max_value: u64,
}

#[derive(Error, Debug)]
pub enum SecretsParseError {
    #[error("Unable to find file extension")]
    UnknownFileType,
    #[error("The file type with extension {ext:?} is not supported")]
    UnsupportedFileType { ext: String },
    #[error("Error converting string found in file to Secret")]
    StringConversionError(#[from] secret::SecretParseError),
    #[error("Error reading the file")]
    FileReadError(#[from] std::io::Error),
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

// TODO test that the tree error propagates correctly (how do we mock in rust?)
// TODO we should fuzz on these tests because the code utilizes a random number
// generator
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
