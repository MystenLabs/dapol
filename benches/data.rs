extern crate nalgebra as na;

use std::collections::HashMap;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use csv::ByteRecord;
use gnuplot::{
    Figure,
    PlotOption::{self},
};
use serde::Deserialize;

// OBJECTS
// ================================================================================================

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
    variable: Variable,
    compute_time: Option<f64>,
    mem_usage: Option<f64>,
    file_size: Option<f64>,
}

// FUNCTIONS
// ================================================================================================

pub fn plot_data_points() {
    // Define points
    let data: Data = get_data(PathBuf::from("benches/bench_data.csv")).unwrap();
    let mut points: Vec<na::Point3<f64>> = Vec::new();

    data.values().for_each(|m| {
        points.push(na::Point3::new(m.compute_time, m.mem_usage, m.file_size));
        // println!("{:#?}", m);
    });

    let mut fg = Figure::new();

    let x = points.iter().map(|p| p.x).collect::<Vec<f64>>();
    let y = points.iter().map(|p| p.y).collect::<Vec<f64>>();
    let z = points.iter().map(|p| p.z).collect::<Vec<f64>>();

    // Plot points
    fg.set_title("Data points").axes3d().points(
        x,
        y,
        z,
        &[PlotOption::Color("black"), PlotOption::PointSize(2.0)],
    );

    // Show the plot
    fg.show().unwrap();
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

        let variable: Variable = record.clone().variable;
        let metrics: Metrics = Metrics::try_from(&record)?;

        // println!("{:?}, {:?}", variable, metrics);

        data.insert(variable, metrics);
    }

    Ok(data)
}
