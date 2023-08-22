//! Non-deterministic mapping sparse Merkle tree (NDM_SMT).

use rand::rngs::ThreadRng;
use rand::{distributions::Uniform, thread_rng, Rng};
use std::collections::HashMap; // STENT TODO double check this is cryptographically safe randomness
use thiserror::Error;

use crate::binary_tree::{Coordinate, InputLeafNode, SparseBinaryTree, SparseBinaryTreeError};
use crate::kdf::generate_key;
use crate::node_content::FullNodeContent;
use crate::primitives::D256;
use crate::user::UserId;

type Content = FullNodeContent<blake3::Hasher>;

pub struct NdmSmt {
    master_secret: D256,
    tree: SparseBinaryTree<Content>,
}

impl NdmSmt {
    /// Constructor.
    fn new(
        master_secret: &D256,
        salt_b: &D256,
        salt_s: &D256,
        height: u8,
        users: Vec<User>,
    ) -> Result<Self, NdmSmtError> {
        let master_secret_bytes = master_secret.as_bytes();
        let salt_b_bytes = salt_b.as_bytes();
        let salt_s_bytes = salt_s.as_bytes();

        let new_padding_node_content = |coord: &Coordinate| {
            // STENT TODO check how much copying is going on here, maybe we can minimize
            let coord_bytes = coord.as_bytes();
            let w = generate_key(master_secret_bytes, &coord_bytes);
            let w_bytes: [u8; 32] = w.into();
            let blinding_factor = generate_key(&w_bytes, salt_b_bytes);
            let salt = generate_key(&w_bytes, salt_s_bytes);
            Content::new_pad(blinding_factor.into(), coord, salt.into())
        };

        let mut x_coord_generator = RandomXCoordGenerator::new(height);
        let mut i = 0;

        let leaves = users
            .into_iter()
            .map(|user| {
                let x_coord = x_coord_generator.new_unique_x_coord(i);
                i = i + 1;

                let w = generate_key(master_secret_bytes, &x_coord.to_le_bytes());
                let w_bytes: [u8; 32] = w.into();
                let blinding_factor = generate_key(&w_bytes, salt_b_bytes);
                let user_salt = generate_key(&w_bytes, salt_s_bytes);
                InputLeafNode {
                    content: Content::new_leaf(
                        user.liability,
                        blinding_factor.into(),
                        user.id,
                        user_salt.into(),
                    ),
                    x_coord,
                }
            })
            .collect();

        let tree = SparseBinaryTree::new(leaves, height, &new_padding_node_content)?;

        Ok(NdmSmt {
            tree,
            master_secret: master_secret.clone(),
        })
    }
}

#[derive(Error, Debug)]
pub enum NdmSmtError {
    #[error("temp")]
    TempErr(#[from] SparseBinaryTreeError),
}

pub struct User {
    liability: u64,
    id: UserId,
}

struct RandomXCoordGenerator {
    map: HashMap<u64, u64>,
    max_value: u64,
    rng: ThreadRng,
}

impl RandomXCoordGenerator {
    fn new(height: u8) -> Self {
        let num_leaves = 2u64.pow(height as u32);

        RandomXCoordGenerator {
            map: HashMap::<u64, u64>::new(),
            max_value: num_leaves,
            rng: thread_rng(),
        }
    }

    /// Durstenfeld’s shuffle algorithm optimized by HashMap.
    fn new_unique_x_coord(&mut self, i: u64) -> u64 {
        let range = Uniform::from(0..self.max_value);
        let mut x_coord = self.rng.sample(range);
        if let Some(value) = self.map.get(&x_coord) {
            x_coord = value.clone();
        }
        self.map.insert(x_coord, i);
        x_coord
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

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

        NdmSmt::new(&master_secret, &salt_b, &salt_s, height, users);
    }
}
