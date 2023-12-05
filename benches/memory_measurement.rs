//! !!! This does not work !!!
//!
//! The idea was to create a custom Criterion measurement to measure the memory
//! amount of memory that a variable takes up.
//!
//! It seems that the construction and destruction of the values in the bench
//! closure don't coincide with the memory readings, since 'after' values are
//! sometimes lower than 'before' values.

use criterion::measurement::{Measurement, ValueFormatter};
use criterion::Throughput;
use jemalloc_ctl::{epoch, stats};

pub struct Memory;

impl Measurement for Memory {
    type Intermediate = usize;
    type Value = usize;

    fn start(&self) -> Self::Intermediate {
        epoch::advance().unwrap();
        let before = stats::allocated::read().unwrap();
        before
    }

    fn end(&self, i: Self::Intermediate) -> Self::Value {
        epoch::advance().unwrap();
        let after = stats::allocated::read().unwrap();
        abs_diff(after, i)
    }

    fn add(&self, v1: &Self::Value, v2: &Self::Value) -> Self::Value {
        *v1 + *v2
    }

    fn zero(&self) -> Self::Value {
        0
    }

    fn to_f64(&self, val: &Self::Value) -> f64 {
        *val as f64
    }

    fn formatter(&self) -> &dyn ValueFormatter {
        &MemoryFormatter
    }
}

struct MemoryFormatter;
impl ValueFormatter for MemoryFormatter {
    fn format_value(&self, value: f64) -> String {
        bytes_as_string(value as usize)
    }

    fn format_throughput(&self, throughput: &Throughput, value: f64) -> String {
        match *throughput {
            Throughput::Bytes(bytes) => format!("{} bytes / bytes", bytes / (value as u64),),
            Throughput::Elements(elems) => format!("{} elem / bytes", elems / (value as u64),),
            Throughput::BytesDecimal(bd) => format!("{} bd / bytes", bd / (value as u64),),
        }
    }

    fn scale_values(&self, typical_value: f64, values: &mut [f64]) -> &'static str {
        for val in values {
            *val = ((*val / 1024u64.pow(2) as f64) * 1000.0).round() / 1000.0;
        }
        "MB"
    }

    fn scale_throughputs(
        &self,
        typical_value: f64,
        throughput: &Throughput,
        values: &mut [f64],
    ) -> &'static str {
        match *throughput {
            Throughput::Bytes(bytes) => {
                for val in values {
                    *val =
                        (bytes as f64) / ((*val / 1024u64.pow(2) as f64) * 1000.0).round() / 1000.0;
                }
                "MB"
            }
            Throughput::Elements(elems) => {
                for val in values {
                    *val =
                        (elems as f64) / ((*val / 1024u64.pow(2) as f64) * 1000.0).round() / 1000.0;
                }
                "MB"
            }
            Throughput::BytesDecimal(bd) => {
                for val in values {
                    *val = (bd as f64) / ((*val / 1024u64.pow(2) as f64) * 1000.0).round() / 1000.0;
                }
                "MB"
            }
        }
    }

    fn scale_for_machines(&self, values: &mut [f64]) -> &'static str {
        for val in values {
            *val = ((*val / 1024u64.pow(2) as f64) * 1000.0).round() / 1000.0;
        }
        "MB"
    }
}

fn bytes_as_string(num_bytes: usize) -> String {
    if num_bytes < 1024 {
        format!("{} bytes", num_bytes)
    } else if num_bytes >= 1024 && num_bytes < 1024usize.pow(2) {
        format!("{} kB", num_bytes / 1024)
    } else if num_bytes >= 1024usize.pow(2) && num_bytes < 1024usize.pow(3) {
        // scale to get accurate decimal values
        format!(
            "{:.2} MB",
            ((num_bytes as f32 / 1024u64.pow(2) as f32) * 1000.0).round() / 1000.0
        )
    } else if num_bytes >= 1024usize.pow(3) && num_bytes < 1024usize.pow(4) {
        format!(
            "{:.2} GB",
            ((num_bytes as f32 / 1024u64.pow(3) as f32) * 1000000.0).round() / 1000000.0
        )
    } else {
        format!(
            "{:.2} TB",
            ((num_bytes as f32 / 1024u64.pow(4) as f32) * 1000000000.0).round() / 1000000000.0
        )
    }
}

fn abs_diff(x: usize, y: usize) -> usize {
    if y < x {
        x - y
    } else {
        y - x
    }
}
