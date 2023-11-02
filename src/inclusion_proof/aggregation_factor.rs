//! Aggregation factor.
//!
//! Method used to determine how many of the range proofs are aggregated. Those
//! that do not form part of the aggregated proof are just proved individually.
//!
//! Divisor: divide the number of nodes by this number to get the ratio of the
//! nodes to be used in the aggregated proof i.e.
//! `number_of_ranges_for_aggregation = tree_height / divisor` (any decimals are
//! truncated, not rounded). Note:
//! - if this number is 0 it means that none of the proofs should be aggregated
//! - if this number is 1 it means that all of the proofs should be aggregated
//! - if this number is `tree_height` it means that only the leaf node should be
//!   aggregated
//! - if this number is `> tree_height` it means that none of the proofs should
//!   be aggregated
//!
//! Percentage: multiply the `tree_height` by this percentage to get the number of
//! nodes to be used in the aggregated proof i.e.
//! `number_of_ranges_for_aggregation = tree_height * percentage`.
//!
//! Number: the exact number of nodes to be used in the aggregated proof. Note
//! that if this number is `> tree_height` it is treated as if it was equal to
//! `tree_height`.

use crate::{binary_tree::Height, percentage::{Percentage, ONE_HUNDRED_PERCENT}};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AggregationFactor {
    Divisor(u8),
    Percent(Percentage),
    Number(u8),
}

/// The default number of proofs to aggregate is all of them because this gives
/// the fastest proving and verification time for a single inclusion proof.
impl Default for AggregationFactor {
    fn default() -> Self {
        AggregationFactor::Percent(ONE_HUNDRED_PERCENT)
    }
}

impl AggregationFactor {
    /// Transform the aggregation factor into a u8, representing the number of
    /// ranges that should aggregated together into a single Bulletproof.
    pub fn apply_to(&self, tree_height: &Height) -> u8 {
        match self {
            Self::Divisor(div) => {
                if *div == 0 || *div > tree_height.as_raw_int() {
                    0
                } else {
                    tree_height.as_raw_int() / div
                }
            }
            Self::Percent(per) => per.apply_to(tree_height.as_raw_int()),
            Self::Number(num) => *num.min(&tree_height.as_raw_int()),
        }
    }

    /// True if `apply_to` would result in 0 no matter the input `tree_height`.
    pub fn is_zero(&self, tree_height: &Height) -> bool {
        match self {
            Self::Divisor(div) => *div == 0 || *div > tree_height.as_raw_int(),
            Self::Percent(per) => per.value() == 0,
            Self::Number(num) => *num == 0,
        }
    }

    /// True if `apply_to` would result in the same as the input `tree_height`.
    pub fn is_max(&self, tree_height: &Height) -> bool {
        match self {
            Self::Divisor(div) => *div == 1,
            Self::Percent(per) => per.value() == 100,
            Self::Number(num) => *num == tree_height.as_raw_int(),
        }
    }
}
