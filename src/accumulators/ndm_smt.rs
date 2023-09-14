//! Non-deterministic mapping sparse Merkle tree (NDM_SMT).
//!
//! TODO more docs

use rand::{distributions::Uniform, rngs::ThreadRng, thread_rng, Rng};
use std::collections::HashMap;
use thiserror::Error;

use crate::binary_tree::{
    TreeBuilder, Coordinate, InputLeafNode, PathError, BinaryTree, TreeBuildError,
};
use crate::inclusion_proof::{AggregationFactor, InclusionProof, InclusionProofError};
use crate::kdf::generate_key;
use crate::node_content::FullNodeContent;
use crate::primitives::D256;
use crate::user::{User, UserId};

use std::time::SystemTime;

use rayon::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

// -------------------------------------------------------------------------------------------------
// NDM-SMT struct and methods

type Hash = blake3::Hasher;
type Content = FullNodeContent<Hash>;

/// Main struct containing tree object, master secret and the salts.
/// The user mapping structure is required because it is non-deterministic.
#[allow(dead_code)]
pub struct NdmSmt {
    master_secret: D256,
    salt_b: D256,
    salt_s: D256,
    tree: BinaryTree<Content>,
    user_mapping: HashMap<UserId, u64>,
}

impl NdmSmt {
    /// Constructor.
    /// TODO more docs
    #[allow(dead_code)]
    pub fn new(
        master_secret: D256,
        salt_b: D256,
        salt_s: D256,
        height: u8,
        users: Vec<User>,
    ) -> Result<Self, NdmSmtError> {
        let master_secret_bytes = master_secret.as_bytes();
        let salt_b_bytes = salt_b.as_bytes();
        let salt_s_bytes = salt_s.as_bytes();

        // needed for the thread-safe closure
        let master_secret_bytes_clone = master_secret_bytes.clone();
        let salt_b_bytes_clone = salt_b_bytes.clone();
        let salt_s_bytes_clone = salt_s_bytes.clone();

        // closure that is used to create new padding nodes
        let new_padding_node_content = move |coord: &Coordinate| {
            // TODO unfortunately we copy data here, maybe there is a way to do without copying
            let coord_bytes = coord.as_bytes();
            // pad_secret is given as 'w' in the DAPOL+ paper
            let pad_secret = generate_key(&master_secret_bytes_clone, &coord_bytes);
            let pad_secret_bytes: [u8; 32] = pad_secret.into();
            let blinding_factor = generate_key(&pad_secret_bytes, &salt_b_bytes_clone);
            let salt = generate_key(&pad_secret_bytes, &salt_s_bytes_clone);
            Content::new_pad(blinding_factor.into(), coord, salt.into())
        };

        let mut x_coord_generator = RandomXCoordGenerator::new(height);

        let start = SystemTime::now();
        println!(
            "  ndm start conversion of users to inputleafnode {:?}",
            start
        );

        // [single] first create vec with x_coords
        // join with users vec
        // [multiple] then generate keys, map to leaf node and
        // [single] add into map

        let mut x_coords = Vec::<u64>::with_capacity(users.len());
        for i in 0..users.len() {
            x_coords.push(x_coord_generator.new_unique_x_coord(i as u64)?);
        }

        let tuples = users
            .into_iter()
            .zip(x_coords.into_iter())
            .collect::<Vec<(User, u64)>>();

        let leaf_nodes = tuples
            .par_iter()
            // .into_par_iter()
            .map(|(user, x_coord)| {
                let w = generate_key(master_secret_bytes, &x_coord.to_le_bytes());
                let w_bytes: [u8; 32] = w.into();
                let blinding_factor = generate_key(&w_bytes, salt_b_bytes);
                let user_salt = generate_key(&w_bytes, salt_s_bytes);

                InputLeafNode {
                    content: Content::new_leaf(
                        user.liability,
                        blinding_factor.into(),
                        user.id.clone(),
                        user_salt.into(),
                    ),
                    x_coord: *x_coord,
                }
            })
            .collect::<Vec<InputLeafNode<Content>>>();

        let user_mapping = Arc::new(Mutex::new(HashMap::new()));
        let user_mapping_ref = Arc::clone(&user_mapping);
        let handle = thread::spawn(move || {
            let mut my_user_mapping = user_mapping_ref.lock().unwrap();
            tuples.into_iter().for_each(|(user, x_coord)| {
                my_user_mapping.insert(user.id, x_coord);
            });
        });
        // https://stackoverflow.com/questions/62613488/how-do-i-get-the-runtime-memory-size-of-an-object
        use std::mem::size_of_val;
        println!("The size of `input_leaf_nodes` is {}", size_of_val(&*leaf_nodes));

        // let mut leaves = Vec::with_capacity(users.len());
        // for user in users.into_iter() {
        //     let x_coord = x_coord_generator.new_unique_x_coord(i as u64)?;
        //     i = i + 1;

        //     let w = generate_key(master_secret_bytes, &x_coord.to_le_bytes());
        //     let w_bytes: [u8; 32] = w.into();
        //     let blinding_factor = generate_key(&w_bytes, salt_b_bytes);
        //     let user_salt = generate_key(&w_bytes, salt_s_bytes);

        //     leaves.push(InputLeafNode {
        //         content: Content::new_leaf(
        //             user.liability,
        //             blinding_factor.into(),
        //             user.id.clone(),
        //             user_salt.into(),
        //         ),
        //         x_coord,
        //     });

        //     user_mapping.insert(user.id, x_coord);
        // }

        let end = SystemTime::now();
        let dur = end.duration_since(start);
        println!("  end {:?}", end);
        println!("  duration {:?}", dur);

        // println!("leaves len {}", leaves.len());
        println!("leaves len {}", leaf_nodes.len());

        let start = SystemTime::now();
        println!("  ndm start single threaded build {:?}", start);

        let tree_2 = TreeBuilder::new()
            .with_height(height)?
            .with_leaf_nodes(leaf_nodes.clone())?
            .single_threaded()?
            .build(new_padding_node_content)?;

        let end = SystemTime::now();
        let dur = end.duration_since(start);
        println!("  end {:?}", end);
        println!("  duration {:?}", dur);

        let start = SystemTime::now();
        println!("  ndm start multi threaded build {:?}", start);

        let tree = TreeBuilder::new()
            .with_height(height)?
            .with_leaf_nodes(leaf_nodes)?
            .multi_threaded()?
            .build(new_padding_node_content)?;

        let end = SystemTime::now();
        let dur = end.duration_since(start);
        println!("  end {:?}", end);
        println!("  duration {:?}", dur);

        handle.join().unwrap();
        let lock = Arc::try_unwrap(user_mapping).expect("Lock still has multiple owners");
        let user_mapping = lock.into_inner().expect("Mutex cannot be locked");

        assert_eq!(tree.get_root(), tree_2.get_root());

        Ok(NdmSmt {
            tree,
            master_secret,
            salt_b,
            salt_s,
            user_mapping,
        })
    }

    /// Generate an inclusion proof for the given user_id.
    ///
    /// The NdmSmt struct defines the content type that is used, and so must define how to extract
    /// the secret value (liability) and blinding factor for the range proof, which are both
    /// required for the range proof that is done in the [InclusionProof] constructor.
    ///
    /// `aggregation_factor` is used to determine how many of the range proofs are aggregated.
    /// Those that do not form part of the aggregated proof are just proved individually. The
    /// aggregation is a feature of the Bulletproofs protocol that improves efficiency.
    //j
    /// `upper_bound_bit_length` is used to determine the upper bound for the range proof, which
    /// is set to `2^upper_bound_bit_length` i.e. the range proof shows
    /// `0 <= liability <= 2^upper_bound_bit_length` for some liability. The type is set to `u8`
    /// because we are not expected to require bounds higher than $2^256$. Note that if the value
    /// is set to anything other than 8, 16, 32 or 64 the Bulletproofs code will return an Err.
    pub fn generate_inclusion_proof_with_custom_range_proof_params(
        &self,
        user_id: &UserId,
        aggregation_factor: AggregationFactor,
        upper_bound_bit_length: u8,
    ) -> Result<InclusionProof<Hash>, NdmSmtError> {
        let leaf_x_coord = self
            .user_mapping
            .get(user_id)
            .ok_or(NdmSmtError::UserIdNotFound)?;

        let path = self.tree.build_path_for(*leaf_x_coord)?;

        Ok(InclusionProof::generate(
            path,
            aggregation_factor,
            upper_bound_bit_length,
        )?)
    }

    /// Generate an inclusion proof for the given user_id.
    ///
    /// Use the default values for Bulletproof parameters:
    /// - `aggregation_factor`: half of all the range proofs are aggregated
    /// - `upper_bound_bit_length`: 64 (which should be plenty enough for most real-world cases)
    pub fn generate_inclusion_proof(
        &self,
        user_id: &UserId,
    ) -> Result<InclusionProof<Hash>, NdmSmtError> {
        let aggregation_factor = AggregationFactor::Divisor(2u8);
        let upper_bound_bit_length = 64u8;
        self.generate_inclusion_proof_with_custom_range_proof_params(
            user_id,
            aggregation_factor,
            upper_bound_bit_length,
        )
    }
}

#[derive(Error, Debug)]
pub enum NdmSmtError {
    #[error("Problem constructing the tree")]
    TreeError(#[from] TreeBuildError),
    #[error("Number of users cannot be bigger than 2^height")]
    HeightTooSmall(#[from] OutOfBoundsError),
    #[error("Inclusion proof generation failed when trying to build the path in the tree")]
    InclusionProofPathGenerationError(#[from] PathError),
    #[error("Inclusion proof generation failed")]
    InclusionProofGenerationError(#[from] InclusionProofError),
    #[error("User ID not found in the user mapping")]
    UserIdNotFound,
}

// -------------------------------------------------------------------------------------------------
// Random shuffle algorithm

/// Used for generating x-coordinate values on the bottom layer of the tree.
///
/// A struct is needed is because the algorithm used to generate new values keeps a memory of
/// previously used values so that it can generate new ones randomly different from previous ones.
///
/// The map is necessary for the algorithm used to get new unique values.
struct RandomXCoordGenerator {
    map: HashMap<u64, u64>,
    max_value: u64,
    rng: ThreadRng,
}

impl RandomXCoordGenerator {
    /// Constructor.
    ///
    /// The max value is the max number of bottom-layer leaves for the given height because we are trying to
    /// generate x-coords on the bottom layer of the tree.
    fn new(height: u8) -> Self {
        use crate::binary_tree::num_bottom_layer_nodes;

        RandomXCoordGenerator {
            map: HashMap::<u64, u64>::new(),
            max_value: num_bottom_layer_nodes(height),
            rng: thread_rng(),
        }
    }

    /// Durstenfeld’s shuffle algorithm optimized by HashMap.
    ///
    /// TODO put this into latex
    /// Iterate over i:
    /// - pick random k in range [i, max_value]
    /// - if k in map then set v = map[k]
    ///   - while v = map[v] exists
    ///   - result = v
    /// - else result = k
    /// - set map[k] = i
    ///
    /// This algorithm provides a constant-time random mapping of all i's without chance of
    /// collision, as long as i <= max_value.
    fn new_unique_x_coord(&mut self, i: u64) -> Result<u64, OutOfBoundsError> {
        if i > self.max_value {
            return Err(OutOfBoundsError {
                max_value: self.max_value,
            });
        }

        let range = Uniform::from(i..self.max_value);
        let k = self.rng.sample(range);

        let x = match self.map.get(&k) {
            Some(mut existing_x) => {
                // follow the full chain of linked numbers until we find the leaf
                while self.map.contains_key(existing_x) {
                    existing_x = self.map.get(existing_x).unwrap();
                }
                existing_x.clone()
            }
            None => k,
        };

        self.map.insert(k, i);
        Ok(x)
    }
}

#[derive(Error, Debug)]
#[error("Counter i cannot exceed max value {max_value:?}")]
pub struct OutOfBoundsError {
    max_value: u64,
}

// -------------------------------------------------------------------------------------------------
// Unit tests

// TODO test that the tree error propagates correctly (how do we mock in rust?)
// TODO we should fuzz on these tests because the code utilizes a random number generator
#[cfg(test)]
mod tests {
    mod ndm_smt {
        use std::str::FromStr;

        use super::super::*;

        #[test]
        fn constructor_works() {
            let master_secret: D256 = 1u64.into();
            let salt_b: D256 = 2u64.into();
            let salt_s: D256 = 3u64.into();
            let height = 4u8;
            let users = vec![User {
                liability: 5u64,
                id: UserId::from_str("some user").unwrap(),
            }];

            NdmSmt::new(master_secret, salt_b, salt_s, height, users).unwrap();
        }
    }

    mod random_x_coord_generator {
        use std::collections::HashSet;

        use super::super::{OutOfBoundsError, RandomXCoordGenerator};
        use crate::binary_tree::num_bottom_layer_nodes;

        #[test]
        fn constructor_works() {
            let height = 4u8;
            RandomXCoordGenerator::new(height);
        }

        #[test]
        fn new_unique_value_works() {
            let height = 4u8;
            let mut rxcg = RandomXCoordGenerator::new(height);
            for i in 0..num_bottom_layer_nodes(height) {
                rxcg.new_unique_x_coord(i).unwrap();
            }
        }

        #[test]
        fn generated_values_all_unique() {
            let height = 4u8;
            let mut rxcg = RandomXCoordGenerator::new(height);
            let mut set = HashSet::<u64>::new();
            for i in 0..num_bottom_layer_nodes(height) {
                let x = rxcg.new_unique_x_coord(i).unwrap();
                if set.contains(&x) {
                    panic!("{:?} was generated twice!", x);
                }
                set.insert(x);
            }
        }

        #[test]
        fn new_unique_value_fails_for_large_i() {
            use crate::testing_utils::assert_err;

            let height = 4u8;
            let max = num_bottom_layer_nodes(height);
            let mut rxcg = RandomXCoordGenerator::new(height);
            let res = rxcg.new_unique_x_coord(max + 1);

            assert_err!(res, Err(OutOfBoundsError { max_value: max }));
        }
    }
}
