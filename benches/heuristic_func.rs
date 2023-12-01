extern crate nalgebra as na;

use std::collections::HashMap;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::mem;
use std::path::PathBuf;

use csv::ByteRecord;
use serde::Deserialize;

use gnuplot::{
    Figure,
    PlotOption::{self},
};
use na::{ArrayStorage, Const, Matrix};

// Assuming each hash value is 32 bytes (adjust based on your use case)
const HASH_SIZE_BYTES: usize = 32;

type Data = HashMap<Variable, Metrics>;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize)]
struct Variable(String);

#[derive(Debug, Deserialize)]
struct Metrics {
    compute_time: f64,
    mem_usage: f64,
    file_size: f64,
}

impl TryFrom<&Record> for Metrics {
    type Error = &'static str;

    fn try_from(value: &Record) -> Result<Self, Self::Error> {
        let compute_time = if let Some(c) = value.compute_time {
            c
        } else {
            return Err("missing compute_time");
        };

        let mem_usage = if let Some(m) = value.mem_usage {
            m
        } else {
            return Err("missing mem_usage");
        };

        let file_size = if let Some(f) = value.file_size {
            f
        } else {
            return Err("missing file_size");
        };

        Ok(Self {
            compute_time,
            mem_usage,
            file_size,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Record {
    _variable: Variable,
    compute_time: Option<f64>,
    mem_usage: Option<f64>,
    file_size: Option<f64>,
}

// Heuristic function to estimate memory usage for a Merkle Tree
pub fn estimate_memory_usage(height: u8, num_users: u64) -> usize {
    // Calculate the number of hash values in the Merkle Tree
    let num_hash_values = 2u32.pow(height as u32);

    // Calculate the total memory usage
    let memory_usage_bytes =
        num_users as usize * HASH_SIZE_BYTES + num_hash_values as usize * mem::size_of::<u8>();

    memory_usage_bytes
}

pub fn plot() -> Result<(), Box<dyn Error>> {
    // Define points
    let data: Data = get_data(PathBuf::from("benches/bench_data.csv"))?;

    let mut points: Vec<na::Point3<f64>> = Vec::new();

    data.values().for_each(|m| {
        points.push(na::Point3::new(m.compute_time, m.mem_usage, m.file_size));
        // println!("{:#?}", m);
    });

    // Calculate best-fit plane
    let plane = fit_plane(&points);

    // Plot points and plane
    plot_3d(&points, plane);

    Ok(())
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
    fg.axes3d()
        .set_x_axis(true, &[PlotOption::Color("black")])
        .set_y_axis(true, &[PlotOption::Color("black")])
        .set_z_axis(true, &[PlotOption::Color("black")])
        .lines_points(x, y, z.clone(), &[PlotOption::Color("black"), PlotOption::PointSize(2.0)])
        .surface(
            plane.into_iter(),
            points.len(),
            points.len(),
            None,
            &[PlotOption::Color("blue"), PlotOption::Caption("Plane")],
        );

    // Show the plot
    fg.show().unwrap();

    // fg.save_to_png("benches/3d_plot.png", 640, 480)
}

// helper method
fn get_data(path: PathBuf) -> Result<Data, Box<dyn Error>> {
    let file: File = OpenOptions::new().read(true).open(&path)?; // open summaries

    println!("path: {:?}", &path);
    println!("file len: {:?}", file.metadata()?.len());

    let mut rdr: csv::Reader<File> = csv::ReaderBuilder::new()
        // .trim(csv::Trim::All)
        .from_reader(file);

    // rdr.byte_records().for_each(|r| println!("{:?}", r));

    let mut data: Data = HashMap::new();

    for result in rdr.byte_records() {
        let byte_record: ByteRecord = result?;
        let record: Record = byte_record.deserialize(None)?;
        // println!("{:?}", record);

        let variable: Variable = record.clone()._variable;
        let metrics: Metrics = Metrics::try_from(&record)?;

        // println!("{:?}, {:?}", variable, metrics);

        data.insert(variable, metrics);
    }

    Ok(data)
}
