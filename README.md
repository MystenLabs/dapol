# DAPOL+ implementation
Implementation of the DAPOL+ protocol introduced in the "Generalized Proof of Liabilities" by Yan Ji and Konstantinos Chalkias ACM CCS 2021 paper, available here: https://eprint.iacr.org/2021/1350

*Note: This is a work in progress, and is not yet ready for production use.

## CLI usage

For now you must clone the repo to use the CLI. Once you have cloned the repo you can invoke the CLI like so:
```bash
cargo run -- help
```

Building a tree can be done
- from a config file (see tree_config_example.toml)
- from CLI arguments
- by deserializing an already-built tree

Build a tree using config file (full log verbosity):
```bash
cargo run -- -vvv build-tree config-file ./tree_config_example.toml
```

Add serialization:
```bash
cargo run -- -vvv build-tree config-file ./tree_config_example.toml --serialize
```

Generate proofs:
```bash
cargo run -- -vvv build-tree config-file ./tree_config_example.toml --serialize --gen-proofs ./entities_example.csv
```

Other options can be explored with:
```bash
cargo run -- build-tree help
cargo run -- gen-proofs help
cargo run -- verify-proof help
```
