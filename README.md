# DAPOL+ implementation

Implementation of the DAPOL+ protocol introduced in the "Generalized Proof of Liabilities" by Yan Ji and Konstantinos Chalkias ACM CCS 2021 paper, available here: https://eprint.iacr.org/2021/1350

**NOTE** This project is currently still a work in progress, but is ready for
use as is. The code has _not_ been audited yet (as of Nov 2023).

Top-level doc for the project: https://hackmd.io/p0dy3R0RS5qpm3sX-_zreA

## How this code can be used

There is both a Rust API and a CLI.

## Rust API

TODO

## CLI usage

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

### Tree building

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
./target/release/dapol -vvv build-tree config-file ./tree_config_example.toml --gen-proofs ./entities_example.csv
```

### Proof generation

As seen above, the proof generation can be done via the tree build command, but it can also be done via its own command, which offers some more options around how the proofs are generated.

```bash
./target/release/dapol -vvv gen-proofs --entity-ids entities_example.csv --tree-file <serialized_tree_file>
```

The proof generation command only offers 1 way to inject the tree (deserialization), as apposed to the tree build which offers different options.

### Proof verification

```bash
./target/release/dapol -vvv verify-proof --file-path <inclusion_proof_file> --root-hash <hash>
```

The root hash is logged out at info level when the tree is built or deserialized.


