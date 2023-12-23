use crate::{
    binary_tree::Height,
    percentage::{Percentage, ONE_HUNDRED_PERCENT},
};

use serde::{Deserialize, Serialize};

/// For adjusting range proof aggregation in the Bulletproofs protocol.
///
/// The Bulletproofs protocol allows many range proofs to be proved together,
/// which is faster than proving them individually. [AggregationFactor] is used
/// to determine how many of the range proofs in an inclusion proof are
/// aggregated (proved together). Those that do not form part of the aggregated
/// proof are just proved individually.
///
/// [AggregationFactor] is an enum with 3 options:
///
/// Divisor: divide the number of nodes by this number to get the ratio of the
/// nodes to be used in the aggregated proof i.e.
/// `number_of_ranges_for_aggregation = tree_height / divisor` (any decimals are
/// truncated, not rounded). Note:
/// - if this number is 0 it means that none of the proofs should be aggregated
/// - if this number is 1 it means that all of the proofs should be aggregated
/// - if this number is `tree_height` it means that only the leaf node should be
///   aggregated
/// - if this number is `> tree_height` it means that none of the proofs should
///   be aggregated
///
/// Percentage: multiply the `tree_height` by this percentage to get the number
/// of nodes to be used in the aggregated proof i.e.
/// `number_of_ranges_for_aggregation = tree_height * percentage`.
///
/// Number: the exact number of nodes to be used in the aggregated proof. Note
/// that if this number is `> tree_height` it is treated as if it was equal to
/// `tree_height`.
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
                if *div == 0 || *div > tree_height.as_u8() {
                    0
                } else {
                    tree_height.as_u8() / div
                }
            }
            Self::Percent(per) => per.apply_to(tree_height.as_u8()),
            Self::Number(num) => *num.min(&tree_height.as_u8()),
        }
    }

    /// True if `apply_to` would result in 0 no matter the input `tree_height`.
    pub fn is_zero(&self, tree_height: &Height) -> bool {
        match self {
            Self::Divisor(div) => *div == 0 || *div > tree_height.as_u8(),
            Self::Percent(per) => per.value() == 0,
            Self::Number(num) => *num == 0,
        }
    }

    /// True if `apply_to` would result in the same as the input `tree_height`.
    pub fn is_max(&self, tree_height: &Height) -> bool {
        match self {
            Self::Divisor(div) => *div == 1,
            Self::Percent(per) => per == &ONE_HUNDRED_PERCENT,
            Self::Number(num) => *num >= tree_height.as_u8(),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Unit tests

#[cfg(test)]
mod tests {
    mod divisor {
        use super::super::*;
        use crate::Height;

        // TODO fuzz on tree height
        #[test]
        fn zero_divisor_gives_zero_aggregation() {
            let tree_height = Height::expect_from(10);
            let aggregation_factor = AggregationFactor::Divisor(0);
            assert_eq!(aggregation_factor.apply_to(&tree_height), 0);
            assert!(aggregation_factor.is_zero(&tree_height));
            assert!(!aggregation_factor.is_max(&tree_height));
        }

        // TODO fuzz on tree height
        #[test]
        fn one_divisor_gives_full_aggregation() {
            let tree_height = Height::expect_from(10);
            let aggregation_factor = AggregationFactor::Divisor(1);
            assert_eq!(
                aggregation_factor.apply_to(&tree_height),
                tree_height.as_u8()
            );
            assert!(!aggregation_factor.is_zero(&tree_height));
            assert!(aggregation_factor.is_max(&tree_height));
        }

        // TODO fuzz on tree height
        #[test]
        fn tree_height_divisor_gives_one_aggregation() {
            let tree_height = Height::expect_from(10);
            let aggregation_factor = AggregationFactor::Divisor(tree_height.as_u8());
            assert_eq!(aggregation_factor.apply_to(&tree_height), 1);
            assert!(!aggregation_factor.is_zero(&tree_height));
            assert!(!aggregation_factor.is_max(&tree_height));
        }

        // TODO fuzz on tree height & the number added to tree height
        #[test]
        fn greater_than_tree_height_divisor_gives_zero_aggregation() {
            let tree_height = Height::expect_from(10);
            let aggregation_factor = AggregationFactor::Divisor(tree_height.as_u8() + 1);
            assert_eq!(aggregation_factor.apply_to(&tree_height), 0);
            assert!(aggregation_factor.is_zero(&tree_height));
            assert!(!aggregation_factor.is_max(&tree_height));
        }
    }

    mod percent {
        use super::super::*;
        use crate::percentage::{Percentage, ONE_HUNDRED_PERCENT};
        use crate::Height;

        // TODO fuzz on tree height
        #[test]
        fn one_hundred_percent_gives_full_aggregation() {
            let tree_height = Height::expect_from(10);
            let aggregation_factor = AggregationFactor::Percent(ONE_HUNDRED_PERCENT);
            assert_eq!(
                aggregation_factor.apply_to(&tree_height),
                tree_height.as_u8()
            );
            assert!(!aggregation_factor.is_zero(&tree_height));
            assert!(aggregation_factor.is_max(&tree_height));
        }

        // TODO fuzz on tree height
        #[test]
        fn fifty_percent_gives_half_aggregation() {
            let tree_height = Height::expect_from(10);
            let aggregation_factor = AggregationFactor::Percent(Percentage::expect_from(50));
            assert_eq!(
                aggregation_factor.apply_to(&tree_height),
                tree_height.as_u8() / 2
            );
            assert!(!aggregation_factor.is_zero(&tree_height));
            assert!(!aggregation_factor.is_max(&tree_height));
        }

        // TODO fuzz on tree height
        #[test]
        fn zero_percent_gives_zero_aggregation() {
            let tree_height = Height::expect_from(10);
            let aggregation_factor = AggregationFactor::Percent(Percentage::expect_from(0));
            assert_eq!(aggregation_factor.apply_to(&tree_height), 0);
            assert!(aggregation_factor.is_zero(&tree_height));
            assert!(!aggregation_factor.is_max(&tree_height));
        }
    }

    mod number {
        use super::super::*;
        use crate::Height;

        // TODO fuzz on tree height
        #[test]
        fn zero_number_gives_zero_aggregation() {
            let tree_height = Height::expect_from(10);
            let aggregation_factor = AggregationFactor::Number(0);
            assert_eq!(aggregation_factor.apply_to(&tree_height), 0);
            assert!(aggregation_factor.is_zero(&tree_height));
            assert!(!aggregation_factor.is_max(&tree_height));
        }

        // TODO fuzz on tree height
        #[test]
        fn one_number_gives_one_aggregation() {
            let tree_height = Height::expect_from(10);
            let aggregation_factor = AggregationFactor::Number(1);
            assert_eq!(aggregation_factor.apply_to(&tree_height), 1);
            assert!(!aggregation_factor.is_zero(&tree_height));
            assert!(!aggregation_factor.is_max(&tree_height));
        }

        // TODO fuzz on tree height
        #[test]
        fn tree_height_number_gives_full_aggregation() {
            let tree_height = Height::expect_from(10);
            let aggregation_factor = AggregationFactor::Number(tree_height.as_u8());
            assert_eq!(
                aggregation_factor.apply_to(&tree_height),
                tree_height.as_u8()
            );
            assert!(!aggregation_factor.is_zero(&tree_height));
            assert!(aggregation_factor.is_max(&tree_height));
        }

        // TODO fuzz on tree height and the number added to tree height
        #[test]
        fn greater_than_tree_height_number_gives_full_aggregation() {
            let tree_height = Height::expect_from(10);
            let aggregation_factor = AggregationFactor::Number(tree_height.as_u8() + 1);
            assert_eq!(
                aggregation_factor.apply_to(&tree_height),
                tree_height.as_u8()
            );
            assert!(!aggregation_factor.is_zero(&tree_height));
            assert!(aggregation_factor.is_max(&tree_height));
        }
    }
}
