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
    let n = num_bytes as u64;

    let kb = 1024u64;
    let mb = kb * 1000;
    let gb = mb * 1000;
    let tb = gb * 1000;

    if n < kb {
        format!("{} bytes", num_bytes)
    } else if n < mb {
        format!("{} kB", n / kb)
    } else if n < gb {
        format!("{:.2} MB", n as f64 / mb as f64)
    } else if n < tb {
        format!("{:.2} GB", n as f64 / gb as f64)
    } else {
        format!("{:.2} TB", n as f64 / tb as f64)
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

    println!("buf capacity: {:<6}", bytes_to_string(buf.capacity()));

    println!("Memory usage: {} allocated", bytes_to_string(diff),);
}
