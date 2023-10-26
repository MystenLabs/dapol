//! Command Line Interface.
//!
//! Output of `--help`:
//! ```bash,ignore
//! DAPOL+ Proof of Liabilities protocol in Rust
//!
//!     Usage: dapol [OPTIONS] <--entity-file <ENTITY_FILE>|--random-entities <RANDOM_ENTITIES>>
//!
//!     Options:
//!         -e, --entity-file <ENTITY_FILE>
//!             Path to file containing entity ID & liability entries (supported file types: csv)
//!         -r, --random-entities <RANDOM_ENTITIES>
//!             Randomly generate a number of entities
//!         -v, --verbose...
//!             More output per occurrence
//!         -q, --quiet...
//!             Less output per occurrence
//!         --height <HEIGHT>
//!             Height to use for the binary tree
//!         -s, --secrets <SECRETS>
//!             TOML file containing secrets (see secrets_example.toml)
//!         -h, --help
//!             Print help
//!         -V, --version
//!             Print version
//! ```
// TODO DOCS replace above help text with better description

use clap::{command, Args, Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use patharg::{InputArg, OutputArg};

use std::str::FromStr;

use crate::binary_tree::Height;

// STENT TODO print out the root when the tree is done building
// STENT TODO we want a keep-running flag after new or from-file, for doing
// proofs

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Initial command for the program.
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,
    // /// Choose an accumulator type for the tree.
    // #[arg(short, long, value_enum)]
    // pub accumulator: AccumulatorType,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Construct a tree.
    BuildTree {
        /// Choose the accumulator type for the tree.
        #[command(subcommand)]
        accumulator_type: AccumulatorTypeCommand,

        /// Generate inclusion proofs for the provided entity IDs, after
        /// building the tree (TODO not implemented yet).
        #[clap(short, long, value_name = "ENTITY_IDS_PATH")]
        gen_proofs: Option<InputArg>,

        /// Keep the program running to initiate more commands (TODO not
        /// implemented yet).
        #[clap(short, long)]
        keep_alive: bool,
    },
    GenProofs {},
}

//#[derive(ValueEnum, Debug, Clone)]
#[derive(Debug, Subcommand)]
pub enum AccumulatorTypeCommand {
    /// Read accumulator type and other tree configuration from a file.
    FromConfig {
        /// Path to the config file.
        file_path: InputArg,
    },
    /// Use the non-deterministic mapping sparse Merkle tree.
    NdmSmt {
        #[command(subcommand)]
        tree_build_type: TreeBuildCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum TreeBuildCommand {
    /// Create a new tree from the given parameters.
    New {
        /// Height to use for the binary tree.
        #[arg(long, value_parser = Height::from_str, default_value = Height::default(), value_name = "U8_INT")]
        height: Height,

        /// TOML file containing secrets (e.g. secrets_example.toml).
        #[clap(short, long, value_name = "FILE_PATH")]
        secrets_file: Option<InputArg>,

        #[command(flatten)]
        entity_source: EntitySource,

        /// Serialize the tree to a file (a default file name will be given if
        /// only a directory is provided) (file extension is .dapoltree).
        #[clap(short = 'S', long, value_name = "PATH")]
        serialize: Option<OutputArg>,
    },
    /// Deserialize a tree from a file.
    Deserialize { path: InputArg },
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
pub struct EntitySource {
    /// Path to file containing entity ID & liability entries (supported file
    /// types: csv).
    #[arg(short, long, value_name = "FILE_PATH")]
    pub entities_file: Option<InputArg>,

    /// Randomly generate a number of entities.
    #[arg(short, long, value_name = "NUM_ENTITIES")]
    pub random_entities: Option<u64>,
}
