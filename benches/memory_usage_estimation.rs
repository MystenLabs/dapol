use dapol::{Height};

// TODO we need to rather fit an arbitrary surface than a plane.
// See the python file TODO for more details.

/// Estimated memory usage in MB.
/// The equation was calculated using the plane_of_best_fit.py script
/// and data that was gathered from running some of the benchmarks on a Macbook Pro.
pub fn estimated_total_memory_usage_mb(height: &Height, num_entities: &u64) -> f64 {
    let x = height.as_u8() as f64;
    let y = *num_entities as f64;
    return 1.276870f64 * x + 0.000772f64 * y + -21.818744f64;
}
