extern crate nalgebra as na;

use std::mem;

use gnuplot::{
    Figure,
    PlotOption::{self},
};
use na::{ArrayStorage, Const, Matrix};

// Assuming each hash value is 32 bytes (adjust based on your use case)
const HASH_SIZE_BYTES: usize = 32;

// Heuristic function to estimate memory usage for a Merkle Tree
pub fn estimate_memory_usage(height: u8, num_users: u64) -> usize {
    // Calculate the number of hash values in the Merkle Tree
    let num_hash_values = 2u32.pow(height as u32);

    // Calculate the total memory usage
    let memory_usage_bytes =
        num_users as usize * HASH_SIZE_BYTES + num_hash_values as usize * mem::size_of::<u8>();

    memory_usage_bytes
}

pub fn plot() {
    // TODO: replace with actual data (already collected)
    // Define points
    let points = vec![
        na::Point3::new(0.0, 0.0, 0.0),
        na::Point3::new(1.0, 3.0, 5.0),
        na::Point3::new(-5.0, 6.0, 3.0),
        na::Point3::new(3.0, 6.0, 7.0),
        na::Point3::new(-2.0, 6.0, 7.0),
    ];

    // Calculate best-fit plane
    let plane = fit_plane(&points);

    // Plot points and plane
    plot_3d(&points, plane);
}

fn fit_plane(
    points: &Vec<na::Point3<f64>>,
) -> Matrix<f64, Const<3>, Const<1>, ArrayStorage<f64, 3, 1>> {
    // Convert points to a matrix
    let points_matrix = na::DMatrix::from_iterator(
        points.len(),
        3,
        points.iter().flat_map(|p| p.coords.iter().cloned()),
    );

    // Use SVD to calculate the best-fit plane
    let svd = points_matrix.svd(true, true);

    // Extract the normal vector from the right singular vectors
    let normal_vector = na::Vector3::new(
        svd.v_t.clone().unwrap()[(0, 2)],
        svd.v_t.clone().unwrap()[(1, 2)],
        svd.v_t.clone().unwrap()[(2, 2)],
    );

    normal_vector.normalize()
}

fn plot_3d(
    points: &Vec<na::Point3<f64>>,
    plane: Matrix<f64, Const<3>, Const<1>, ArrayStorage<f64, 3, 1>>,
) {
    let mut fg = Figure::new();

    let x = points.iter().map(|p| p.x).collect::<Vec<f64>>();
    let y = points.iter().map(|p| p.y).collect::<Vec<f64>>();
    let z = points.iter().map(|p| p.z).collect::<Vec<f64>>();

    // Plot points
    fg.axes3d().points(x, y, z, &[PlotOption::Color("black")]);

    fg.axes3d().surface(
        &plane,
        points.len(),
        3,
        None,
        &[PlotOption::Color("blue"), PlotOption::Caption("Plane")],
    );

    // Show the plot
    fg.show().unwrap();

    // fg.save_to_png("benches/3d_plot.png", 640, 480)
}
