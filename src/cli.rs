//! Command Line Interface.
//!
//! Output of `--help`:
//! ```
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
// TODO replace above help text with better description

use clap::{command, Args, Parser};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use clio::Input;
use patharg::InputArg;

use std::str::FromStr;

use crate::binary_tree::Height;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub entity_source: EntitySource,

    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// Height to use for the binary tree.
    #[arg(long, value_parser = Height::from_str)]
    pub height: Option<Height>,

    /// TOML file containing secrets (see secrets_example.toml).
    #[clap(short, long, value_parser)]
    pub secrets: Option<Input>,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
pub struct EntitySource {
    /// Path to file containing entity ID & liability entries (supported file
    /// types: csv).
    #[arg(short, long)]
    pub entity_file: Option<InputArg>,

    /// Randomly generate a number of entities.
    #[arg(short, long)]
    pub random_entities: Option<u64>,
}
