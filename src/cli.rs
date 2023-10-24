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

use clap::{command, Args, Parser, Subcommand, ValueEnum};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use patharg::{InputArg, OutputArg};

use std::str::FromStr;

use crate::binary_tree::Height;

pub const SERIALIZED_TREE_EXTENSION: &str = "dapoltree";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    #[arg(short, long, value_enum)]
    pub accumulator: AccumulatorType,
}

// STENT TODO print out the root when the tree is done building
#[derive(Debug, Subcommand)]
pub enum Commands {
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
    FromFile { path: InputArg },
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

#[derive(ValueEnum, Debug, Clone)]
pub enum AccumulatorType {
    NdmSmt,
}
