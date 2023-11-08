//! Used for generating unique x-coordinate values on the bottom layer of the
//! tree.
//!
//! A struct is needed as apposed to just a function because the algorithm used
//! to generate new values requires keeping a memory of previously used values
//! so that it can generate new ones that are different from previous ones.
//!
//! Fields:
//! - `rng` is a cryptographically secure pseudo-random number generator.
//! - The `used_x_coords` map keeps track of which x-coords have already been
//! generated.
//! - `max_x_coord` is the upper bound on the generated values, 0 being the
//! lower bound.
//! - `i` is used to track the current position of the algorithm.
//!
//! Usage:
//! After creating the struct the calling code can repeatedly call
//! `new_unique_x_coord` any number of times in the range `[1, max_x_coord]`.
//! If the function is called more than `max_x_coord` times an error will be
//! returned.
//!
//! The random values are generated using Durstenfeld’s shuffle algorithm
//! optimized by HashMap. This algorithm wraps the `rng`, efficiently avoiding
//! collisions. Here is some pseudo code explaining how it works:
//!
//! ```bash,ignore
//! if N > max_x_coord throw error
//! for i in range [0, N]:
//! - pick random k in range [i, max_x_coord]
//! - if k in map then set v = map[k]
//!   - while map[v] exists: v = map[v]
//!   - result = v
//! - else result = k
//! - set map[k] = i
//! ```
//!
//! Assuming `rng` is constant time the above algorithm has time complexity
//! `O(N)`. Note that the second loop (the while loop) will only execute a
//! total of `N` times throughout the entire loop cycle of the first loop.
//! This is because the second loop will only execute if a chain in the map
//! exists, and the worst case happens when there is 1 long chain containing
//! all the elements of the map; in this case the second loop will only execute
//! on 1 of the iterations of the first loop.
// TODO DOCS the above explanation is not so good, improve it

use crate::binary_tree::Height;
use rand::{distributions::Uniform, rngs::ThreadRng, thread_rng, Rng};
use std::collections::HashMap;

pub struct RandomXCoordGenerator {
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
    pub fn from(height: &Height) -> Self {
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
    pub fn new_unique_x_coord(&mut self) -> Result<u64, OutOfBoundsError> {
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
                *existing_x
            }
            None => random_x,
        };

        self.used_x_coords.insert(random_x, self.i);
        self.i += 1;
        Ok(x)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Counter i cannot exceed max value {max_value:?}")]
pub struct OutOfBoundsError {
    pub max_value: u64,
}
