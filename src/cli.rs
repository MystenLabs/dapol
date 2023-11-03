//! Command Line Interface.
//!
//! See [LONG_ABOUT] for more information.

use clap::{command, Args, Parser, Subcommand, ValueEnum};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use patharg::{InputArg, OutputArg};
use primitive_types::H256;

use std::str::FromStr;

use crate::{
    binary_tree::Height,
    inclusion_proof::DEFAULT_UPPER_BOUND_BIT_LENGTH,
    percentage::{Percentage, ONE_HUNDRED_PERCENT},
};

// TODO we want a keep-running flag after new or from-file, for doing
// proofs

const LONG_ABOUT: &str = "DAPOL+ Proof of Liabilities protocol in Rust

MOREMORETODO";
// TODO more long docs (above , and also for some of the commands)

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = LONG_ABOUT)]
pub struct Cli {
    /// Initial command for the program.
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Construct a tree.
    BuildTree {
        /// Choose the accumulator type for the tree.
        #[command(subcommand)]
        build_kind: BuildKindCommand,

        /// Generate inclusion proofs for the provided entity IDs, after
        /// building the tree.
        #[arg(short, long, value_name = "ENTITY_IDS_FILE_PATH", global = true)]
        gen_proofs: Option<InputArg>,

        // /// Keep the program running to initiate more commands (TODO not
        // /// implemented yet).
        // #[arg(short, long)]
        // keep_alive: bool,
        /// Serialize the tree to a file (a default file name will be given if
        /// only a directory is provided) (file extension is .dapoltree)
        /// (this option is ignored if 'deserialize' command is used).
        #[arg(short = 'S', long, value_name = "FILE_PATH", global = true)]
        serialize: Option<OutputArg>,
    },
    /// Generate inclusion proofs for entities.
    GenProofs {
        /// List of entity IDs to generate proofs for, can be a file path or
        /// simply a comma separated list read from stdin (usi "-" to
        /// indicate stdin).
        #[arg(short, long)]
        entity_ids: InputArg,

        /// Path to the tree file that will be deserialized.
        #[arg(short, long, value_name = "FILE_PATH")]
        tree_file: InputArg,

        /// Percentage of the range proofs that
        /// are aggregated using the Bulletproofs protocol.
        #[arg(short, long, value_parser = Percentage::from_str, default_value = ONE_HUNDRED_PERCENT, value_name = "PERCENTAGE")]
        range_proof_aggregation: Percentage,

        /// Upper bound for the range proofs is 2^(this_number).
        #[arg(short, long, default_value_t = DEFAULT_UPPER_BOUND_BIT_LENGTH, value_name = "U8_INT")]
        upper_bound_bit_length: u8,
    },
    /// Verify an inclusion proof.
    VerifyProof {
        /// File path for the serialized inclusion proof json file.
        #[arg(short, long)]
        file_path: InputArg,

        /// Hash digest/bytes for the root node of the tree.
        #[arg(short, long, value_parser = H256::from_str, value_name = "BYTES")]
        root_hash: H256,
    },
}

#[derive(Debug, Subcommand)]
pub enum BuildKindCommand {
    /// Create a new tree using the CLI.
    New {
        /// Choose an accumulator type for the tree.
        #[arg(short, long, value_enum)]
        accumulator: AccumulatorType,

        /// Height to use for the binary tree.
        #[arg(long, value_parser = Height::from_str, default_value = Height::default(), value_name = "U8_INT")]
        height: Height,

        /// TOML file containing secrets (e.g. secrets_example.toml).
        #[arg(short, long, value_name = "FILE_PATH")]
        secrets_file: Option<InputArg>,

        #[command(flatten)]
        entity_source: EntitySource,
    },
    /// Read accumulator type and other tree configuration from a file.
    ConfigFile {
        /// Path to the config file.
        file_path: InputArg,
    },
    /// Deserialize a tree from a file.
    Deserialize { path: InputArg },
}

#[derive(ValueEnum, Debug, Clone)]
pub enum AccumulatorType {
    NdmSmt,
    // TODO other accumulators..
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
