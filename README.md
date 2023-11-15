# Proof of Liabilities protocol implemented in Rust

[![Crates.io](https://img.shields.io/crates/v/dapol?style=flat-square)](https://crates.io/crates/dapol)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/silversixpence-crypto/dapol/ci.yml?branch=main&style=flat-square)](https://github.com/silversixpence-crypto/dapol/actions/workflows/ci.yml?query=branch%3Amain)

Licensed under [MIT](LICENSE).

## About

Implementation of the DAPOL+ protocol introduced in the "Generalized Proof of Liabilities" by Yan Ji and Konstantinos Chalkias ACM CCS 2021 paper, available here: https://eprint.iacr.org/2021/1350

Top-level doc for the project: https://hackmd.io/p0dy3R0RS5qpm3sX-_zreA

## What is contained in this code

This library offers an efficient build algorithm for constructing a binary Merkle Sum Tree representing the liabilities of an organization. Efficiency is achieved through parallelization. Details on the algorithm used can be found in [the multi-threaded builder file](https://github.com/silversixpence-crypto/dapol/blob/main/src/binary_tree/tree_builder/multi_threaded.rs).

The paper describes a few different accumulator variants. The Sparse Merkle Sum Tree is the DAPOL+ accumulator, but there are a few different axes of variation, such as how the list of entities is embedded within the tree. The 4 accumulator variants are simply slightly different versions of the Sparse Merkle Sum Tree. Only the Non-Deterministic Mapping Sparse Merkle Tree variant has been implemented so far.

The code offers inclusion proof generation & verification using the Bulletproofs protocol for the range proofs.

## Still to be done

This project is currently still a work in progress, but is ready for
use as is. The code has _not_ been audited yet (as of Nov 2023). Progress can be tracked here: https://github.com/silversixpence-crypto/dapol/issues/91

A Rust crate has not been released yet, progress can be tracked here: https://github.com/silversixpence-crypto/dapol/issues/13

A spec for this code still needs to be written: https://github.com/silversixpence-crypto/dapol/issues/17

A fuzzing technique should be used for the unit tests: https://github.com/silversixpence-crypto/dapol/issues/46

Performance can be improved: https://github.com/silversixpence-crypto/dapol/issues/44

Alternate accumulators mentioned in the paper should be built: https://github.com/silversixpence-crypto/dapol/issues/9 https://github.com/silversixpence-crypto/dapol/issues/8 https://github.com/silversixpence-crypto/dapol/issues/7

Other than the above there are a few minor tasks to do, each of which has an issue for tracking.

## How this code can be used

There is both a Rust API and a CLI. Details for both can be found in the sections below.

### Rust API

The library has not been released as a crate yet (as of Nov 2023) but the API has the following capabilities:
- build a tree using the builder pattern or a configuration file
- generate inclusion proofs from a list of entity IDs (tree required)
- verify an inclusion proof using a root hash (no tree required)

See the examples directory for details on how to use the API.

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


