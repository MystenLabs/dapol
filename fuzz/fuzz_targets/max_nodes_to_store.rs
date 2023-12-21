#![no_main]

use libfuzzer_sys::fuzz_target;
use dapol::Height;

fuzz_target!(|randomness: u64| {
    dapol::fuzz::fuzz_max_nodes_to_store(randomness);
});
