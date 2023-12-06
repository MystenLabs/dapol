# Proof of Liabilities protocol implemented in Rust

[![Crates.io](https://img.shields.io/crates/v/dapol?style=flat-square)](https://crates.io/crates/dapol)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/silversixpence-crypto/dapol/ci.yml?branch=main&style=flat-square)](https://github.com/silversixpence-crypto/dapol/actions/workflows/ci.yml?query=branch%3Amain)

Licensed under [MIT](LICENSE).

## About

Implementation of the DAPOL+ protocol introduced in the "Generalized Proof of Liabilities" by Yan Ji and Konstantinos Chalkias ACM CCS 2021 paper, available [here](https://eprint.iacr.org/2021/1350)

See the [top-level doc for the project](https://hackmd.io/p0dy3R0RS5qpm3sX-_zreA) if you would like to know more about Proof of Liabilities.

## Still to be done

This project is currently still a work in progress, but is ready for
use as is. The code has _not_ been audited yet (as of Nov 2023). Progress can be tracked [here](https://github.com/silversixpence-crypto/dapol/issues/91).

A Rust crate has not been released yet, progress can be tracked [here](https://github.com/silversixpence-crypto/dapol/issues/13).

A spec for this code still needs to be [written](https://github.com/silversixpence-crypto/dapol/issues/17).

A fuzzing technique should be used for the unit [tests](https://github.com/silversixpence-crypto/dapol/issues/46).

Performance can be [improved](https://github.com/silversixpence-crypto/dapol/issues/44).

Alternate accumulators mentioned in the paper should be built:
- [Deterministic mapping SMT](https://github.com/silversixpence-crypto/dapol/issues/9)
- [ORAM-based SMT](https://github.com/silversixpence-crypto/dapol/issues/8)
- [Hierarchical SMTs](https://github.com/silversixpence-crypto/dapol/issues/7)

Other than the above there are a few minor tasks to do, each of which has an issue for tracking.

## How this code can be used

There is both a Rust API and a CLI. Details for both can be found in the sections below.

### Rust API

The library has not been released as a crate yet (as of Nov 2023) but the API has the following capabilities:
- build a tree using the builder pattern or a configuration file
- generate inclusion proofs from a list of entity IDs (tree required)
- verify an inclusion proof using a root hash (no tree required)

See the [examples](https://github.com/silversixpence-crypto/dapol/examples) directory for details on how to use the API.

### CLI

You will need to have the rust compiler installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs >> ./rustup-init.sh
./rustup-init.sh -y --no-modify-path
rm -f ./rustup-init.sh
```

For now you must clone the repo to use the CLI. Once you have cloned the repo build everything:
```bash
# run inside the repo
cargo build --release
```

You can invoke the CLI like so:
```bash
./target/release/dapol help
```

The CLI offers 3 main operations: tree building, proof generation & proof verification. All options can be explored with:
```bash
./target/release/dapol build-tree help
./target/release/dapol gen-proofs help
./target/release/dapol verify-proof help
```

#### Tree building

Building a tree can be done:
- from a config file (see tree_config_example.toml)
- from CLI arguments
- by deserializing an already-built tree

Build a tree using config file (full log verbosity):
```bash
./target/release/dapol -vvv build-tree config-file ./tree_config_example.toml
```

Add serialization:
```bash
./target/release/dapol -vvv build-tree config-file ./tree_config_example.toml --serialize .
```

Deserialize a tree from a file:
```bash
./target/release/dapol -vvv build-tree deserialize <file>
```

Generate proofs (proofs will live in the `./inclusion_proofs/` directory):
```bash
./target/release/dapol -vvv build-tree config-file ./tree_config_example.toml --gen-proofs ./examples/entities_example.csv
```

#### Proof generation

As seen above, the proof generation can be done via the tree build command, but it can also be done via its own command, which offers some more options around how the proofs are generated.

```bash
./target/release/dapol -vvv gen-proofs --entity-ids ./examples/entities_example.csv --tree-file <serialized_tree_file>
```

The proof generation command only offers 1 way to inject the tree (deserialization), as apposed to the tree build which offers different options.

#### Proof verification

```bash
./target/release/dapol -vvv verify-proof --file-path <inclusion_proof_file> --root-hash <hash>
```

The root hash is logged out at info level when the tree is built or deserialized.

## Benchmarks

To run the benchmarks first clone the repo and then run:
```bash
MAX_ENTITIES_FOR_CRITERION=100000 MIN_ENTITIES_FOR_MANUAL_BENCHES=100000 cargo bench
```
The env vars are optional. The benches are split into 2 parts: Criterion for small benches and manual for large benches. A set of tuples is used as input to the benches:

![](resources/readme_eq_benchmark.svg)

Some of the values of $n$ cause the benchmarks to take *really* long (multiple hours), and so using Criterion (which takes a minimum of 10 samples per bench) makes things too slow. This is why the benches are split by the $n$ input: smaller $n$ is fine for Criterion, larger $n$ is handled by manual benches that only run once. The env vars control where this split occurs.

You may experience an error building the benches if you are on a fresh Linux machine. If the jemalloc-sys package fails to build then maybe [this](https://github.com/nervosnetwork/ckb/issues/1886) will help.


