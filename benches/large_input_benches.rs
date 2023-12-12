use once_cell::sync::Lazy;
use std::time::Instant;

use dapol::accumulators::NdmSmtConfigBuilder;
use dapol::initialize_machine_parallelism;

mod inputs;
use inputs::{max_thread_counts, num_entities_greater_than, tree_heights};

mod memory_usage_estimation;
use memory_usage_estimation::estimated_total_memory_usage_mb;

mod utils;
use utils::{abs_diff, bytes_to_string, system_total_memory_mb};

/// Determines how many runs are done for number of entities.
/// The higher this value the fewer runs that are done.
///
/// Some of the tree builds can take a few hours, and Criterion does a minimum
/// of 10 samples per bench. So this value gives us to decide how much of the
/// num_entities
static MIN_ENTITIES_FOR_MANUAL_BENCHES: Lazy<u64> = Lazy::new(|| {
    std::env::var("MIN_ENTITIES_FOR_MANUAL_BENCHES")
        .unwrap_or("100000".to_string())
        .parse()
        .unwrap()
});

/// This is required to get jemalloc_ctl to work properly.
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() {
    use dapol::{Height, MaxThreadCount};

    let epoch = jemalloc_ctl::epoch::mib().unwrap();
    let allocated = jemalloc_ctl::stats::allocated::mib().unwrap();

    let total_mem = system_total_memory_mb();

    initialize_machine_parallelism();

    println!(
        "==========================================================\n \
              Manual benchmarks"
    );

    for h in tree_heights().iter() {
        for t in max_thread_counts().iter() {
            for n in num_entities_greater_than(*MIN_ENTITIES_FOR_MANUAL_BENCHES).iter() {
                // only: (height 32, max_thread_count 16, num_entities 30000000)
                if *h != Height::from(32) || *t != MaxThreadCount::from(16) || *n != 30000000u64 {
                    println!("Skipping, targeting only (height 32, max_thread_count 16, num_entities 30000000)");
                    continue;
                }

                // ==============================================================
                // Input validation.

                {
                    // We attempt to guess the amount of memory that the tree
                    // build will require, and if that is greater than the
                    // amount of memory available on the machine then we skip
                    // the input tuple.

                    let expected_mem = estimated_total_memory_usage_mb(h, n);

                    if total_mem < expected_mem {
                        println!(
                            "Skipping input height_{}/num_entities_{} since estimated memory \
                                  usage {} is greater than the system max {}",
                            h.as_u32(),
                            n,
                            expected_mem,
                            total_mem
                        );

                        continue;
                    }
                }

                // Do not try build the tree if the number of entities exceeds
                // the maximum number allowed. If this check is not done then
                // we would get an error on tree build.
                if n > &h.max_bottom_layer_nodes() {
                    println!(
                        "Skipping input height_{}/num_entities_{} since number of entities is \
                              greater than max allowed",
                        h.as_u32(),
                        n
                    );

                    continue;
                }

                println!(
                    "\nRunning benchmark for input values \
                     (height {}, max_thread_count {}, num_entities {})",
                    h.as_u32(),
                    t.as_u8(),
                    n
                );

                // ==============================================================
                // Tree build.

                epoch.advance().unwrap();
                let mem_before = allocated.read().unwrap();
                println!("Memory before build {}", bytes_as_string(mem_before));
                let time_start = Instant::now();

                let ndm_smt = NdmSmtConfigBuilder::default()
                    .height(h.clone())
                    .max_thread_count(t.clone())
                    .num_random_entities(*n)
                    .build()
                    .parse()
                    .expect("Unable to parse NdmSmtConfig");

                let tree_build_time = time_start.elapsed();
                epoch.advance().unwrap();
                let mem_after = allocated.read().unwrap();
                println!("Memory after build {}", bytes_as_string(mem_after));
                let mem_used_tree_build = abs_diff(mem_after, mem_before);

                // ==============================================================
                // Tree serialization.

                // let src_dir = env!("CARGO_MANIFEST_DIR");
                // let target_dir = Path::new(&src_dir).join("target");
                // let dir = target_dir.join("serialized_trees");
                // let path = Accumulator::parse_accumulator_serialization_path(dir).unwrap();
                // let acc = Accumulator::NdmSmt(ndm_smt);

                // let time_start = Instant::now();
                // acc.serialize(path.clone());
                // let serialization_time = time_start.elapsed();

                // let file_size = std::fs::metadata(path)
                //     .expect("Unable to get serialized tree metadata for {path}")
                //     .len();

                // ==============================================================
                // Print stats.

                println!(
                    "\nTime taken to build tree: {:?}\n \
                     Memory used to build tree: {}\n \
                     Time taken to serialize tree: {:?}\n \
                     Serialized tree file size: {}\n \
                     ========================================================================",
                    tree_build_time,
                    bytes_to_string(mem_used_tree_build),
                    "NA",
                    "NA",
                    // serialization_time,
                    // bytes_to_string(file_size as usize)
                );
            }
        }
    }
}
