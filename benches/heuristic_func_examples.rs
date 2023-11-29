// EXAMPLE 1 (custom ChatGPT heuristic func)
use std::mem;

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

// EXAMPLE 1

// This Python code to Rust:
// https://scikit-spatial.readthedocs.io/en/stable/gallery/fitting/plot_plane.html

use nalgebra::{Matrix3, Vector3};
use plotters::prelude::*;

fn plot() {
    // Define points
    let points = vec![
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 3.0, 5.0),
        Vector3::new(-5.0, 6.0, 3.0),
        Vector3::new(3.0, 6.0, 7.0),
        Vector3::new(-2.0, 6.0, 7.0),
    ];

    // Find the best-fit plane
    let plane = best_fit_plane(&points);

    // Plot 3D points and the plane
    plot_3d(points, plane);
}

fn best_fit_plane(points: &Vec<Vector3<f64>>) -> (Vector3<f64>, Vector3<f64>) {
    let centroid = compute_centroid(points);
    let centered_points: Vec<Vector3<f64>> = points.iter().map(|p| p - centroid).collect();

    let covariance_matrix = compute_covariance_matrix(&centered_points);
    let eigenvectors = covariance_matrix.symmetric_eigen().eigenvalues;
    let normal = eigenvectors.column(0);

    (centroid, normal.into())
}

fn compute_centroid(points: &Vec<Vector3<f64>>) -> Vector3<f64> {
    points.iter().fold(Vector3::zeros(), |acc, &p| acc + p) / points.len() as f64
}

fn compute_covariance_matrix(points: &Vec<Vector3<f64>>) -> Matrix3<f64> {
    let n = points.len() as f64;
    let centroid = compute_centroid(points);

    let mut covariance_matrix = Matrix3::zeros();

    for p in points {
        let centered_point = p - centroid;
        covariance_matrix += centered_point * centered_point.transpose();
    }

    covariance_matrix /= n;

    covariance_matrix
}

fn plot_3d(points: Vec<Vector3<f64>>, plane: (Vector3<f64>, Vector3<f64>)) {
    let root = BitMapBackend::new("3d_plot.png", (800, 600)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&root)
        .caption("3D Plot", ("sans-serif", 20))
        .build_cartesian_3d(-10.0..10.0, -10.0..10.0, -10.0..10.0)
        .unwrap();

    chart.configure_axes().draw().unwrap();

    // Plot 3D points
    chart
        .draw_series(points.into_iter().map(|p| {
            return Circle::new((p.x as i32, p.y as i32), 5, ShapeStyle::from(&BLACK));
        }))
        .unwrap();

    // Plot the best-fit plane
    let normal = plane.1;
    let d = -normal.dot(&plane.0);
    let plane_points = (0..100)
        .flat_map(|i| (0..100).map(move |j| (i as f64 - 50.0, j as f64 - 50.0)))
        .map(|(i, j)| {
            let k = -(normal.x * i + normal.y * j + d) / normal.z;
            (i, j, k)
        });

    chart
        .draw_series(SurfaceSeries::new(plane_points, 100, &WHITE.mix(0.5)))
        .unwrap();
}

// EXAMPLE 2

// This Python code to Rust
// https://math.stackexchange.com/a/99317

// use nalgebra::{Matrix3, Vector3, U2};

fn get_vectors() {
    // Generate some random test points
    let m = 20; // number of points
    let delta = 0.01; // size of random displacement
    let origin = Vector3::new(rand::random(), rand::random(), rand::random()); // random origin for the plane
    let basis = Matrix3::from_fn(|_, _| rand::random()); // random basis vectors for the plane
    let coefficients = Matrix3::from_fn(|_, _| rand::random()); // random coefficients for points on the plane

    // Generate random points on the plane and add random displacement
    let points = basis * coefficients + origin.broadcast(m);

    // Now find the best-fitting plane for the test points

    // Subtract out the centroid and take the SVD
    let centroid = points.column_mean();
    let centered_points = points - centroid.broadcast(m);
    let svd = centered_points.svd(true, true);

    // Extract the left singular vectors
    let left = svd.u.unwrap();

    // Print the left singular vectors
    println!("Left singular vectors:\n{}", left);
}
