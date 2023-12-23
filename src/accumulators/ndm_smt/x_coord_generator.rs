use crate::binary_tree::Height;
use rand::distributions::{Distribution, Uniform};
use std::collections::HashMap;

/// Used for generating unique x-coordinate values on the bottom layer of the
/// tree.
///
/// A struct is needed as opposed to just a function because the algorithm used
/// to generate new values requires keeping a memory of previously used values
/// so that it can generate new ones that are different from previous ones.
///
/// Private fields:
/// - `rng` is a cryptographically secure pseudo-random number generator.
/// - The `used_x_coords` map keeps track of which x-coords have already been
/// generated.
/// - `max_x_coord` is the upper bound on the generated values, 0 being the
/// lower bound.
/// - `i` is used to track the current position of the algorithm.
///
/// Example:
/// ```
/// use dapol::accumulators::RandomXCoordGenerator;
///
/// let height = dapol::Height::default();
/// let mut x_coord_generator = RandomXCoordGenerator::from_height(&height);
/// let x_coord = x_coord_generator.new_unique_x_coord().unwrap();
/// ```
///
/// After creating the struct you can repeatedly call
/// `new_unique_x_coord` any number of times in the range `[1, max_x_coord]`.
/// If the function is called more than `max_x_coord` times an error will be
/// returned.
///
/// The random values are generated using Durstenfeld’s shuffle algorithm,
/// optimized by a HashMap. This algorithm wraps the `rng`, efficiently avoiding
/// collisions. Here is some pseudo code explaining how it works:
///
/// Key:
/// - `n` is the number of users that need to be mapped to leaf nodes
/// - `x_coord` is the index of the leaf node (left-most x-coord is 0,
///   right-most x-coord is `max_x_coord`)
/// - `user_mapping` is the result of the algorithm, where each user is given a
///   leaf node index i.e. `user_mapping: users -> indices`
/// - `tracking_map` is used to determine which indices have been used
///
/// ```python,ignore
/// if n > max_x_coord throw error
///
/// user_mapping = new_empty_hash_map()
/// tracking_map = new_empty_hash_map()
///
/// for i in [0, n):
///   pick random k in range [i, max_x_coord]
///   if k in tracking_map then set v = traking_map[k]
///     while traking_map[v] exists: v = tracking_map[v]
///     set user_mapping[i] = v
///   else user_mapping[i] = k
///   set tracking_map[k] = i
/// ```
///
/// Assuming `rng` is constant-time and the HashMap is optimized by some
/// balanced search tree then the above algorithm has time and memory complexity
/// `O(n log(n))` in the worst case. Note that the second loop (the while loop)
/// will only execute a total of `n` times throughout the entire loop cycle of
/// the first loop. This is because the second loop will only execute if a chain
/// in the map exists, and the worst case happens when there is 1 long chain
/// containing all the elements of the map; in this case the second loop will
/// only execute on 1 of the iterations of the first loop.
pub struct RandomXCoordGenerator {
    rng: RngSelector,
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
    pub fn from_height(height: &Height) -> Self {
        RandomXCoordGenerator {
            used_x_coords: HashMap::<u64, u64>::new(),
            max_x_coord: height.max_bottom_layer_nodes(),
            rng: RngSelector::default(),
            i: 0,
        }
    }

    #[cfg(any(test, fuzzing))]
    pub fn from_seed(height: &Height, seed: u64) -> Self {
        RandomXCoordGenerator {
            used_x_coords: HashMap::<u64, u64>::new(),
            max_x_coord: height.max_bottom_layer_nodes(),
            rng: RngSelector::from_seed(seed),
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

        let random_x = self.rng.sample_range(self.i, self.max_x_coord);

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

// -------------------------------------------------------------------------------------------------
// Pick RNG based on feature.

use rng_selector::RngSelector;

trait Sampleable {
    fn sample_range(&mut self, lower: u64, upper: u64) -> u64;
}

#[cfg(not(any(test, fuzzing)))]
mod rng_selector {
    use rand::distributions::Uniform;
    use rand::{rngs::ThreadRng, thread_rng, Rng};

    use super::Sampleable;

    pub(super) struct RngSelector(ThreadRng);

    impl Default for RngSelector {
        fn default() -> Self {
            Self(thread_rng())
        }
    }

    impl Sampleable for RngSelector {
        fn sample_range(&mut self, lower: u64, upper: u64) -> u64 {
            let range = Uniform::from(lower..upper);
            self.0.sample(range)
        }
    }
}

#[cfg(any(test, fuzzing))]
mod rng_selector {
    use rand::Rng;
    use rand::{rngs::SmallRng, SeedableRng};

    use super::Sampleable;

    pub(super) struct RngSelector(SmallRng);

    impl Default for RngSelector {
        fn default() -> Self {
            Self(SmallRng::from_entropy())
        }
    }

    impl RngSelector {
        pub fn from_seed(seed: u64) -> Self {
            let mut bytes = [0u8; 32];
            let (left, _right) = bytes.split_at_mut(8);
            left.copy_from_slice(&seed.to_le_bytes());
            Self(SmallRng::from_seed(bytes))
        }
    }

    impl Sampleable for RngSelector {
        fn sample_range(&mut self, lower: u64, upper: u64) -> u64 {
            self.0.gen_range(lower..upper)
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Unit tests.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_tree::Height;
    use std::collections::HashSet;

    #[test]
    fn constructor_works() {
        let height = Height::expect_from(4u8);
        RandomXCoordGenerator::from_height(&height);
    }

    #[test]
    fn new_unique_value_works() {
        let height = Height::expect_from(4u8);
        let mut rxcg = RandomXCoordGenerator::from_height(&height);
        for _i in 0..height.max_bottom_layer_nodes() {
            rxcg.new_unique_x_coord().unwrap();
        }
    }

    #[test]
    fn generated_values_all_unique() {
        let height = Height::expect_from(4u8);
        let mut rxcg = RandomXCoordGenerator::from_height(&height);
        let mut set = HashSet::<u64>::new();
        for _i in 0..height.max_bottom_layer_nodes() {
            let x = rxcg.new_unique_x_coord().unwrap();
            if set.contains(&x) {
                panic!("{:?} was generated twice!", x);
            }
            set.insert(x);
        }
    }

    #[test]
    fn new_unique_value_fails_for_large_i() {
        use crate::utils::test_utils::assert_err;

        let height = Height::expect_from(4u8);
        let mut rxcg = RandomXCoordGenerator::from_height(&height);
        let max = height.max_bottom_layer_nodes();
        let mut res = rxcg.new_unique_x_coord();

        for _i in 0..max {
            res = rxcg.new_unique_x_coord();
        }

        assert_err!(res, Err(OutOfBoundsError { max_value: _ }));
    }
}
