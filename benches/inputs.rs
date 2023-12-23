//! Input values for benchmarking.

use dapol::{Height, MaxThreadCount};

/// We only bench for heights 16, 32 & 64 because smaller than 16 is fairly
/// useless in practice and greater than 64 is not supported yet.
pub fn tree_heights() -> Vec<Height> {
    let tree_heights: Vec<u8> = Vec::from([16, 32, 64]);
    tree_heights
        .into_iter()
        .map(|x| Height::expect_from(x))
        .collect()
}

pub fn tree_heights_in_range(lower: Height, upper: Height) -> Vec<Height> {
    tree_heights()
        .into_iter()
        .filter(|x| &lower <= x && x <= &upper)
        .collect()
}

/// For <10M entities we increase linearly, bumping the increment amount an
/// order of magnitude each time reach a new power of 10.
/// For >10M entities we increase in greater steps because each run can take
/// multiple hours to execute.
pub fn num_entities() -> Vec<u64> {
    Vec::from([
        10_000,
        20_000,
        30_000,
        40_000,
        50_000,
        60_000,
        70_000,
        80_000,
        90_000,
        100_000,
        200_000,
        300_000,
        400_000,
        500_000,
        600_000,
        700_000,
        800_000,
        900_000,
        1_000_000,
        2_000_000,
        3_000_000,
        4_000_000,
        5_000_000,
        6_000_000,
        7_000_000,
        8_000_000,
        9_000_000,
        10_000_000,
        30_000_000,
        50_000_000,
        70_000_000,
        90_000_000,
        100_000_000,
        125_000_000,
        250_000_000,
    ])
}

pub fn num_entities_in_range(lower: u64, upper: u64) -> Vec<u64> {
    num_entities()
        .into_iter()
        .filter(|x| &lower <= x && x <= &upper)
        .collect()
}

pub fn max_thread_counts() -> Vec<MaxThreadCount> {
    let mut tc: Vec<u8> = Vec::new();

    let max_thread_count: u8 = MaxThreadCount::default().as_u8();

    let step = if max_thread_count < 8 {
        1
    } else {
        max_thread_count >> 2
    };

    for i in (step..max_thread_count).step_by(step as usize) {
        tc.push(i);
    }
    tc.push(max_thread_count);

    println!("\nmax_thread_counts {:?}\n", tc);

    tc.into_iter().map(|x| MaxThreadCount::from(x)).collect()
}

pub fn max_thread_counts_greater_than(lower_bound: MaxThreadCount) -> Vec<MaxThreadCount> {
    max_thread_counts()
        .into_iter()
        .filter(|x| &lower_bound <= x)
        .collect()
}
