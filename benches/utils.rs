//! Utility functions for the benchmarks.

use sysinfo::{System, SystemExt};

/// Total memory available in MB.
pub fn system_total_memory_mb() -> f64 {
    System::new_all();
    let mut sys = System::new_all();
    sys.refresh_all();
    let mem_bytes = sys.total_memory();
    mem_bytes as f64 / 1024u64.pow(2) as f64
}

pub fn abs_diff(x: usize, y: usize) -> usize {
    if y < x {
        x - y
    } else {
        y - x
    }
}

pub fn bytes_to_string(num_bytes: usize) -> String {
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

// -------------------------------------------------------------------------------------------------
// Testing jemalloc_ctl to make sure it gives expected memory readings.

#[allow(dead_code)]
pub fn bench_test_jemalloc_readings() {
    use jemalloc_ctl::{epoch, stats};

    let e = epoch::mib().unwrap();
    let alloc = stats::allocated::mib().unwrap();

    e.advance().unwrap();
    let before = alloc.read().unwrap();

    // 1 MB
    let buf: Vec<u8> = Vec::with_capacity(1024u32.pow(2) as usize);

    e.advance().unwrap();
    let after = alloc.read().unwrap();

    let diff = after - before;

    println!(
        "buf capacity: {:<6}",
        bytes_to_string(buf.capacity())
    );

    println!("Memory usage: {} allocated", bytes_to_string(diff),);
}
